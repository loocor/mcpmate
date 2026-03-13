//! Event-driven capability synchronization
//!
//! This module provides a lightweight capability sync manager specifically designed
//! for event-driven scenarios where we only need real-time sync without persistent caching.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tracing::debug;

use crate::core::pool::UpstreamConnectionPool;

/// Lightweight capability manager for event-driven sync
///
/// Unlike the full CapabilityManager, this version:
/// - Only depends on database and connection pool
/// - No RedbCacheManager dependency (avoiding file lock conflicts)
/// - Optimized for real-time event-driven sync
/// - Directly writes to SQLite without intermediate caching
pub struct EventDrivenCapabilityManager {
    db_pool: Arc<Pool<Sqlite>>,
    connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
}

impl EventDrivenCapabilityManager {
    /// Create new event-driven capability manager
    pub fn new(
        db_pool: Arc<Pool<Sqlite>>,
        connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        Self {
            db_pool,
            connection_pool,
        }
    }

    /// Sync capabilities for a single server that just connected successfully
    ///
    /// This method updates the SQLite server_config.capabilities field based on
    /// the actual capabilities discovered from the connected server.
    pub async fn sync_single_server(
        &self,
        server_id: &str,
    ) -> Result<()> {
        // Get server name for logging purposes
        let server_name = crate::config::operations::utils::get_server_name(&self.db_pool, server_id)
            .await
            .unwrap_or_else(|_| server_id.to_string());

        debug!(
            "Event-driven capability sync for server '{}' (ID: {}) - syncing capabilities to SQLite",
            server_name, server_id
        );

        // Get server connection info
        let (tools_count, supports_prompts, supports_resources) = {
            let pool = self.connection_pool.lock().await;

            let instances = pool
                .connections
                .get(server_id)
                .context("Server not found in connection pool")?;

            let conn = instances
                .values()
                .find(|conn| conn.is_connected())
                .context("No connected instance found for server")?;

            let tools_count = conn.tools.len();
            let supports_prompts = conn.supports_prompts();
            let supports_resources = conn.supports_resources();

            (tools_count, supports_prompts, supports_resources)
        };

        // Update capabilities in SQLite
        let supports_tools = tools_count > 0;
        crate::config::server::capabilities::overwrite_capabilities(
            &self.db_pool,
            server_id,
            supports_tools,
            supports_prompts,
            supports_resources,
        )
        .await?;

        debug!(
            "Event-driven capability sync completed for server '{}' (ID: {}): tools={}, prompts={}, resources={}",
            server_name, server_id, supports_tools, supports_prompts, supports_resources
        );

        Ok(())
    }
}
