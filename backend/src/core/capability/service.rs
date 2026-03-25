use crate::core::{cache::RedbCacheManager, pool::UpstreamConnectionPool};
use anyhow::{Context, Result, anyhow};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

/// Session identifier used for temporary validation instances when capability
/// queries require on-demand peers.
pub const CAPABILITY_VALIDATION_SESSION: &str = "capability-service";

/// High-level orchestration service for capabilities.
///
/// Responsibilities:
/// - REDB-first
/// - Runtime pipeline (directly via capability runtime helpers)
/// - Temporary instance fallback (delegated to pool validation instance)
/// - Async write-back to REDB (on success)
pub struct CapabilityService {
    pub redb: Arc<RedbCacheManager>,
    pub pool: Arc<Mutex<UpstreamConnectionPool>>,
    pub database: Arc<crate::config::database::Database>,
}

impl CapabilityService {
    pub fn new(
        redb: Arc<RedbCacheManager>,
        pool: Arc<Mutex<UpstreamConnectionPool>>,
        database: Arc<crate::config::database::Database>,
    ) -> Self {
        Self { redb, pool, database }
    }

    /// Prewarm REDB cache for enabled servers when cache is missing
    /// Uses temporary validation instances, writes back, and destroys them.
    /// Optimization: run prewarm with concurrency = CPU cores (bounded by servers count).
    pub async fn prewarm_enabled_servers_if_cache_miss(&self) -> anyhow::Result<()> {
        use crate::core::cache::{CacheQuery, FreshnessLevel};
        use futures::stream::{self, StreamExt};

        let servers = crate::config::server::get_all_servers(&self.database.pool).await?;
        if servers.is_empty() {
            return Ok(());
        }

        let concurrency = std::cmp::min(servers.len(), num_cpus::get());
        tracing::info!("Prewarm start: servers={}, concurrency={}", servers.len(), concurrency);

        let redb = self.redb.clone();
        let pool = self.pool.clone();
        let db_pool = self.database.pool.clone();

        stream::iter(servers)
            .for_each_concurrent(concurrency, move |server| {
                let redb = redb.clone();
                let pool = pool.clone();
                let db_pool = db_pool.clone();
                async move {
                    let Some(server_id) = &server.id else { return; };

                    // Cache hit, skip
                    let query = CacheQuery {
                        server_id: server_id.clone(),
                        freshness_level: FreshnessLevel::Cached,
                        include_disabled: false,
                        scope: crate::core::cache::CacheScope::shared_raw(),
                    };
                    let cached = match redb.get_server_data(&query).await {
                        Ok(res) => res.data.is_some(),
                        Err(e) => {
                            tracing::warn!(server = %server.name, error = %e, "Cache lookup failed; will attempt prewarm");
                            false
                        }
                    };
                    if cached { return; }

                    // Mark refreshing to help UI/consumers avoid treating empty cache as final
                    tracing::info!(server = %server.name, "Prewarming capability cache via temporary validation instance");
                    let _ = redb
                        .set_refreshing(server_id, std::time::Duration::from_secs(60))
                        .await;

                    let res = crate::config::server::capabilities::sync_via_connection_pool(
                        &pool,
                        &redb,
                        &db_pool,
                        server_id,
                        &server.name,
                        crate::config::server::capabilities::default_pool_lock_timeout_secs(),
                    )
                    .await;

                    match res {
                        Ok(_) => {
                            tracing::debug!(server = %server.name, "Cache prewarm completed");
                        }
                        Err(e) => {
                            tracing::warn!(server = %server.name, error = %e, "Cache prewarm failed");
                        }
                    }
                    let _ = redb.clear_refreshing(server_id).await;
                }
            })
            .await;

        tracing::info!("Prewarm finished");
        Ok(())
    }

    /// List capabilities for a server with unified flow and optional temporary instance fallback
    pub async fn list(
        &self,
        ctx: &crate::core::capability::runtime::ListCtx,
    ) -> Result<crate::core::capability::runtime::ListResult> {
        // 1) Try runtime pipeline (REDB-first -> runtime)
        let mut result = crate::core::capability::runtime::list(ctx, &self.redb, &self.pool, &self.database).await?;

        // 2) If runtime had no peer, create a temporary validation instance (if available) or fall back to standard instance
        if result.items.is_empty() && !result.meta.cache_hit && !result.meta.had_peer {
            let mut retried_with_validation = false;
            if let Some(session_id) = ctx.validation_session.as_deref() {
                let server_name = self.resolve_server_name(&ctx.server_id).await?;
                let mut pool_guard = self.pool.lock().await;
                let create_result = pool_guard
                    .get_or_create_validation_instance(&server_name, session_id, Duration::from_secs(300))
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to create validation instance for server '{}' (session '{}')",
                            server_name, session_id
                        )
                    })?;
                if create_result.is_some() {
                    retried_with_validation = true;
                }
                drop(pool_guard);

                if retried_with_validation {
                    result =
                        crate::core::capability::runtime::list(ctx, &self.redb, &self.pool, &self.database).await?;

                    // Only auto-cleanup the shared capability session; other sessions manage lifecycle themselves.
                    if session_id == CAPABILITY_VALIDATION_SESSION {
                        let mut pool_guard = self.pool.lock().await;
                        if let Err(e) = pool_guard.destroy_validation_instance(&server_name, session_id).await {
                            tracing::debug!(
                                server = %server_name,
                                session = %session_id,
                                error = %e,
                                "Failed to destroy validation instance after capability listing"
                            );
                        }
                    }
                }
            }

            if !retried_with_validation {
                let mut pool = self.pool.lock().await;
                pool.ensure_connected(&ctx.server_id).await?;
                drop(pool);

                result = crate::core::capability::runtime::list(ctx, &self.redb, &self.pool, &self.database).await?;
            }
        }

        Ok(result)
    }

    async fn resolve_server_name(
        &self,
        server_id: &str,
    ) -> Result<String> {
        if let Some(name) = crate::core::capability::resolver::to_name(server_id).await? {
            return Ok(name);
        }

        let server = crate::config::server::get_server_by_id(&self.database.pool, server_id)
            .await?
            .ok_or_else(|| anyhow!("Server '{}' not found in database", server_id))?;

        crate::core::capability::resolver::upsert(server_id, &server.name).await;
        Ok(server.name)
    }
}
