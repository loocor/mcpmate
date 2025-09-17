use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use crate::core::{cache::RedbCacheManager, pool::UpstreamConnectionPool};

/// Session identifier used for temporary validation instances when capability
/// queries require on-demand peers.
pub const CAPABILITY_VALIDATION_SESSION: &str = "capability-service";

/// High-level orchestration service for capabilities.
///
/// Responsibilities:
/// - REDB-first
/// - Runtime via Sandwich
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

    /// List capabilities for a server with unified flow and optional temporary instance fallback
    pub async fn list(
        &self,
        ctx: &crate::core::sandwich::ListCtx,
    ) -> Result<crate::core::sandwich::ListResult> {
        // 1) Try Sandwich path
        let mut result = crate::core::sandwich::Sandwich::list(ctx, &self.redb, &self.pool, &self.database).await?;

        // 2) If runtime had no peer, optionally create a temporary validation instance and retry once
        if result.items.is_empty() && !result.meta.cache_hit && !result.meta.had_peer {
            let server_name = crate::core::capability::global_server_mapping_manager()
                .get_name_by_id(&ctx.server_id)
                .await
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

            // Retry Sandwich once
            result = crate::core::sandwich::Sandwich::list(ctx, &self.redb, &self.pool, &self.database).await?;
        }

        Ok(result)
    }
}
