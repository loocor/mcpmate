use crate::core::{cache::RedbCacheManager, pool::UpstreamConnectionPool};
use anyhow::Result;
use std::sync::Arc;
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
    pub async fn prewarm_enabled_servers_if_cache_miss(&self) -> anyhow::Result<()> {
        use crate::core::cache::{CacheQuery, FreshnessLevel};

        let servers = crate::config::server::get_all_servers(&self.database.pool).await?;

        for server in servers {
            let Some(server_id) = &server.id else {
                continue;
            };

            let query = CacheQuery {
                server_id: server_id.clone(),
                freshness_level: FreshnessLevel::Cached,
                include_disabled: false,
            };
            let cached = match self.redb.get_server_data(&query).await {
                Ok(res) => res.data.is_some(),
                Err(e) => {
                    tracing::warn!(server = %server.name, error = %e, "Cache lookup failed; will attempt prewarm");
                    false
                }
            };
            if cached {
                continue;
            }

            tracing::info!(server = %server.name, "Prewarming capability cache via temporary validation instance");
            // Mark refreshing to help UI/consumers avoid treating empty cache as final
            let _ = self.redb.set_refreshing(server_id, std::time::Duration::from_secs(60)).await;
            if let Err(e) = crate::config::server::capabilities::sync_via_connection_pool(
                &self.pool,
                &self.redb,
                &self.database.pool,
                server_id,
                &server.name,
                10,
            )
            .await
            {
                tracing::warn!(server = %server.name, error = %e, "Cache prewarm failed");
                // Clear marker on failure to avoid sticky state; TTL would clear eventually but explicit is better
                let _ = self.redb.clear_refreshing(server_id).await;
            } else {
                tracing::debug!(server = %server.name, "Cache prewarm completed");
                let _ = self.redb.clear_refreshing(server_id).await;
            }
        }

        Ok(())
    }

    /// List capabilities for a server with unified flow and optional temporary instance fallback
    pub async fn list(
        &self,
        ctx: &crate::core::capability::runtime::ListCtx,
    ) -> Result<crate::core::capability::runtime::ListResult> {
        // 1) Try runtime pipeline (REDB-first -> runtime)
        let mut result = crate::core::capability::runtime::list(ctx, &self.redb, &self.pool, &self.database).await?;

        // 2) If runtime had no peer, optionally create a temporary validation instance and retry once
        if result.items.is_empty() && !result.meta.cache_hit && !result.meta.had_peer {
            let server_name = crate::core::capability::resolver::to_name(&ctx.server_id)
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| ctx.server_id.clone());

            {
                let mut pool = self.pool.lock().await;
                // NOTE: Using validation instance API as a temporary instance provider
                // This aligns with "black-box" creation controlled by pool layer
                let _ = pool
                    .get_or_create_validation_instance(
                        &server_name,
                        CAPABILITY_VALIDATION_SESSION,
                        std::time::Duration::from_secs(60),
                    )
                    .await?;
            }

            // Retry runtime pipeline once
            result = crate::core::capability::runtime::list(ctx, &self.redb, &self.pool, &self.database).await?;
        }

        Ok(result)
    }
}
