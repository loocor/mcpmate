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
    /// Exploration sessions: session_id -> map of server_name to connection (minimal skeleton)
    pub exploration_sessions: HashMap<String, HashMap<String, UpstreamConnection>>,
    /// Validation sessions: session_id -> map of server_name to connection (minimal skeleton)
    pub validation_sessions: HashMap<String, HashMap<String, UpstreamConnection>>,
    /// Exploration session expirations
    pub exploration_expirations: HashMap<String, std::time::Instant>,
    /// Validation session expirations
    pub validation_expirations: HashMap<String, std::time::Instant>,
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
            exploration_sessions: HashMap::new(),
            validation_sessions: HashMap::new(),
            exploration_expirations: HashMap::new(),
            validation_expirations: HashMap::new(),
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

    /// Create or refresh an exploration session with TTL
    pub fn upsert_exploration_session(&mut self, session_id: &str, ttl: Duration) {
        use std::time::Instant;
        self.exploration_sessions.entry(session_id.to_string()).or_default();
        self.exploration_expirations
            .insert(session_id.to_string(), Instant::now() + ttl);
    }

    /// Create or refresh a validation session with TTL
    pub fn upsert_validation_session(&mut self, session_id: &str, ttl: Duration) {
        use std::time::Instant;
        self.validation_sessions.entry(session_id.to_string()).or_default();
        self.validation_expirations
            .insert(session_id.to_string(), Instant::now() + ttl);
    }

    /// Cleanup expired exploration/validation sessions
    pub fn cleanup_expired_sessions(&mut self) {
        use std::time::Instant;
        let now = Instant::now();
        let expired_exploration: Vec<String> = self
            .exploration_expirations
            .iter()
            .filter_map(|(sid, &exp)| if exp <= now { Some(sid.clone()) } else { None })
            .collect();
        for sid in expired_exploration {
            self.exploration_expirations.remove(&sid);
            self.exploration_sessions.remove(&sid);
        }

        let expired_validation: Vec<String> = self
            .validation_expirations
            .iter()
            .filter_map(|(sid, &exp)| if exp <= now { Some(sid.clone()) } else { None })
            .collect();
        for sid in expired_validation {
            self.validation_expirations.remove(&sid);
            self.validation_sessions.remove(&sid);
        }
    }

    /// Get active instance counts for runtime status
    pub fn active_instance_counts(&self) -> (usize, usize, usize) {
        let production = self
            .connections
            .iter()
            .filter(|(_, m)| !m.is_empty())
            .count();
        let exploration = self.exploration_sessions.len();
        let validation = self.validation_sessions.len();
        (production, exploration, validation)
    }

    /// Get or create an exploration instance for a server
    pub fn get_or_create_exploration_instance(
        &mut self,
        server_name: &str,
        session_id: &str,
        ttl: Duration,
    ) -> Result<Option<&UpstreamConnection>, anyhow::Error> {
        // Ensure session exists
        self.upsert_exploration_session(session_id, ttl);
        
        // Check if server connection already exists in this session
        if let Some(session_servers) = self.exploration_sessions.get(session_id) {
            if let Some(connection) = session_servers.get(server_name) {
                return Ok(Some(connection));
            }
        }

        // For now, return None - full implementation would create actual connection
        // This would involve:
        // 1. Get server config from database
        // 2. Create new UpstreamConnection
        // 3. Initialize connection to server
        // 4. Store in exploration_sessions
        
        tracing::debug!("Exploration instance for server '{}' in session '{}' not implemented yet", 
                       server_name, session_id);
        Ok(None)
    }

    /// Get or create a validation instance for a server
    pub fn get_or_create_validation_instance(
        &mut self,
        server_name: &str,
        session_id: &str,
        ttl: Duration,
    ) -> Result<Option<&UpstreamConnection>, anyhow::Error> {
        // Ensure session exists
        self.upsert_validation_session(session_id, ttl);
        
        // Check if server connection already exists in this session
        if let Some(session_servers) = self.validation_sessions.get(session_id) {
            if let Some(connection) = session_servers.get(server_name) {
                return Ok(Some(connection));
            }
        }

        // For now, return None - full implementation would create actual connection
        tracing::debug!("Validation instance for server '{}' in session '{}' not implemented yet", 
                       server_name, session_id);
        Ok(None)
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
