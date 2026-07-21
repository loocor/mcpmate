//! Event-driven capability synchronization
//!
//! This module provides a lightweight capability sync manager specifically designed
//! for event-driven scenarios where a connected server supplies a fresh catalog observation.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tracing::debug;

use crate::core::pool::UpstreamConnectionPool;

/// Lightweight capability manager for event-driven sync
///
/// This manager handles connected-server observations directly:
/// - It only depends on the database and connection pool.
/// - It persists the complete observation through the transactional SQLite catalog.
pub struct EventDrivenCapabilityManager {
    db_pool: Arc<Pool<Sqlite>>,
    capability_cache: Arc<mcpmate_capability_store::DerivedCapabilityCache>,
    connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
}

impl EventDrivenCapabilityManager {
    /// Create new event-driven capability manager
    pub fn new(
        db_pool: Arc<Pool<Sqlite>>,
        capability_cache: Arc<mcpmate_capability_store::DerivedCapabilityCache>,
        connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        Self {
            db_pool,
            capability_cache,
            connection_pool,
        }
    }

    /// Sync capabilities for a single server that just connected successfully
    ///
    /// This method persists the complete observation from the connected server.
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

        let (instance_id, service, tools, capabilities) = {
            let pool = self.connection_pool.lock().await;

            let instances = pool
                .connections
                .get(server_id)
                .context("Server not found in connection pool")?;

            let conn = instances
                .values()
                .find(|conn| conn.is_connected())
                .context("No connected instance found for server")?;
            (
                conn.id.clone(),
                conn.service
                    .clone()
                    .context("Connected instance has no capability peer")?,
                conn.tools.clone(),
                conn.capabilities.clone(),
            )
        };
        let snapshot =
            match crate::config::server::capabilities::discover_from_service(service.as_ref(), tools, capabilities)
                .await
            {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    if let Some(failure) =
                        error.downcast_ref::<crate::config::server::capabilities::CapabilityInventoryDiscoveryError>()
                    {
                        crate::config::server::capabilities::record_capability_failure(
                            &self.db_pool,
                            self.capability_cache.as_ref(),
                            crate::config::server::capabilities::CapabilityFailureEvidence {
                                server_id: server_id.to_string(),
                                kind: failure.kind,
                                instance_id: Some(instance_id),
                                connection_generation: None,
                                reason: format!("{error:#}"),
                            },
                        )
                        .await
                        .context("Failed to persist terminal event-driven capability evidence")?;
                    }
                    return Err(error);
                }
            };

        crate::config::server::capabilities::commit_capability_observation(
            &self.db_pool,
            self.capability_cache.as_ref(),
            server_id,
            &server_name,
            snapshot,
            crate::core::pool::CapSyncFlags::ALL,
        )
        .await?;

        debug!(
            "Event-driven capability sync completed for server '{}' (ID: {})",
            server_name, server_id
        );

        Ok(())
    }
}
