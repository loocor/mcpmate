//! Pool - connection pool management layer
//!
//! Provides connection pool management for upstream MCP servers, including:
//! - connection lifecycle management
//! - health monitoring and reconnection
//! - parallel connection capabilities
//! - resource monitoring and limits

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{self, Result};
use tokio_util::sync::CancellationToken;
use tracing;

use crate::core::{
    connection::UpstreamConnection, foundation::monitor::ProcessMonitor, models::Config,
};

// Core pool functionality modules
mod connection;
mod health;
mod monitoring;
mod parallel;

// Business logic managers (separated from core pool logic)
mod config;
mod database;
mod helpers;
mod sync;

// Re-export managers for external use
pub use config::PoolConfigManager;
pub use sync::ServerSyncManager;

/// Pool of connections to upstream MCP servers
///
/// This is the core connection pool that manages active connections to upstream MCP servers.
/// It focuses purely on connection storage, access, and basic lifecycle management.
/// Business logic for configuration synchronization and server management is handled
/// by dedicated managers (PoolConfigManager and ServerSyncManager).
#[derive(Debug, Clone)]
pub struct UpstreamConnectionPool {
    /// Map of server name to map of instance ID to connection
    pub connections: HashMap<String, HashMap<String, UpstreamConnection>>,
    /// Server configuration
    pub config: Arc<Config>,
    /// Map of server name to map of instance ID to cancellation token
    pub cancellation_tokens: HashMap<String, HashMap<String, CancellationToken>>,
    /// Process monitor for tracking resource usage
    pub process_monitor: Option<Arc<ProcessMonitor>>,
    /// Database reference for checking server status (used by sync manager)
    pub database: Option<Arc<crate::config::database::Database>>,
    /// Runtime cache for fast runtime queries
    pub runtime_cache: Option<Arc<crate::runtime::RuntimeCache>>,
}

impl UpstreamConnectionPool {
    /// Create a new connection pool
    ///
    /// # Arguments
    /// * `config` - The server configuration
    /// * `database` - Optional database reference for checking server status
    pub fn new(
        config: Arc<Config>,
        database: Option<Arc<crate::config::database::Database>>,
    ) -> Self {
        // Create process monitor with 5 second update interval
        let process_monitor = Arc::new(ProcessMonitor::new(Duration::from_secs(5)));

        // Start process monitoring
        ProcessMonitor::start_monitoring(process_monitor.clone());

        Self {
            connections: HashMap::new(),
            config,
            cancellation_tokens: HashMap::new(),
            process_monitor: Some(process_monitor),
            database,
            runtime_cache: None, // Will be set by the proxy server
        }
    }

    /// Update the configuration using the configuration manager
    ///
    /// This method delegates to PoolConfigManager for the actual configuration logic.
    /// It maintains the public API while separating business logic concerns.
    pub fn set_config(
        &mut self,
        config: Arc<Config>,
    ) {
        // Use the configuration manager to handle the complex logic
        if let Err(e) = PoolConfigManager::update_configuration(
            &mut self.connections,
            &mut self.cancellation_tokens,
            config.clone(),
        ) {
            tracing::error!("Failed to update pool configuration: {}", e);
            return;
        }

        // Update the stored configuration reference
        self.config = config;
    }

    /// Set the database reference
    pub fn set_database(
        &mut self,
        database: Option<Arc<crate::config::database::Database>>,
    ) {
        self.database = database;
        tracing::info!("Database reference updated for connection pool");
    }

    /// Set the runtime cache reference
    pub fn set_runtime_cache(
        &mut self,
        runtime_cache: Option<Arc<crate::runtime::RuntimeCache>>,
    ) {
        self.runtime_cache = runtime_cache;
        tracing::info!("Runtime cache reference updated for connection pool");
    }

    /// Initialize the connection pool with all servers
    ///
    /// This method delegates to PoolConfigManager for the initialization logic.
    pub fn initialize(&mut self) {
        PoolConfigManager::initialize_connections(&mut self.connections, &self.config);
    }

    // Instance helper methods are now in instance_helpers.rs

    /// Sync all servers based on active configuration suites
    ///
    /// This method delegates to ServerSyncManager for the complex synchronization logic.
    /// It maintains the public API while separating business logic concerns.
    pub async fn sync_servers_from_active_suits(&mut self) -> Result<()> {
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not available for server sync"))?;

        let sync_manager = ServerSyncManager::new(db.clone());
        sync_manager.sync_servers_from_active_suites(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, sync::Arc};

    fn create_test_config() -> Config {
        Config {
            mcp_servers: HashMap::new(),
            pagination: None,
        }
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let config = Arc::new(create_test_config());
        let pool = UpstreamConnectionPool::new(config, None);

        assert!(pool.connections.is_empty());
        assert!(pool.cancellation_tokens.is_empty());
        assert!(pool.process_monitor.is_some());
    }

    #[tokio::test]
    async fn test_pool_initialization() {
        let config = Arc::new(create_test_config());
        let mut pool = UpstreamConnectionPool::new(config, None);

        pool.initialize();

        // Should not crash and should handle empty config gracefully
        assert!(pool.connections.is_empty());
    }

    #[tokio::test]
    async fn test_connection_state_hash() {
        let config = Arc::new(create_test_config());
        let pool = UpstreamConnectionPool::new(config, None);

        let hash1 = pool.calculate_connection_state_hash();
        let hash2 = pool.calculate_connection_state_hash();

        // Hash should be consistent for the same state
        assert_eq!(hash1, hash2);
    }
}
