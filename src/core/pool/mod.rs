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
    connection::UpstreamConnection,
    foundation::{monitor::ProcessMonitor, types::ConnectionStatus},
    models::Config,
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
    /// 
    /// Returns Ok(()) on success, or Err(CoreError) if configuration update fails.
    pub fn set_config(
        &mut self,
        config: Arc<Config>,
    ) -> Result<(), crate::core::foundation::error::CoreError> {
        // Use the configuration manager to handle the complex logic
        PoolConfigManager::update_configuration(
            &mut self.connections,
            &mut self.cancellation_tokens,
            config.clone(),
        )?;

        // Update the stored configuration reference
        self.config = config;
        tracing::info!("Pool configuration updated successfully");
        Ok(())
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
    ///
    /// This method implements "create-use-destroy" lifecycle for validation instances:
    /// 1. Check if validation instance already exists in session
    /// 2. If not, create temporary validation instance
    /// 3. Instance will be destroyed after use (handled by caller)
    pub async fn get_or_create_validation_instance(
        &mut self,
        server_name: &str,
        session_id: &str,
        _ttl: Duration, // TTL not used for validation instances per requirements
    ) -> Result<Option<&UpstreamConnection>, anyhow::Error> {
        // Check if server connection already exists in this session
        if let Some(session_servers) = self.validation_sessions.get(session_id) {
            if session_servers.contains_key(server_name) {
                return Ok(self.validation_sessions
                    .get(session_id)
                    .and_then(|session| session.get(server_name)));
            }
        }

        // Create temporary validation instance
        let connection = self.create_temporary_validation_instance(server_name, session_id).await?;

        // Store in validation_sessions
        let session_servers = self.validation_sessions.entry(session_id.to_string()).or_default();
        session_servers.insert(server_name.to_string(), connection);

        // Return reference to the stored connection
        Ok(self.validation_sessions
            .get(session_id)
            .and_then(|session| session.get(server_name)))
    }

    /// Create a temporary validation instance for a server
    ///
    /// This creates a temporary connection that will be used for capability inspection
    /// and then immediately destroyed. It does not affect the server's enabled status.
    async fn create_temporary_validation_instance(
        &mut self,
        server_name: &str,
        session_id: &str,
    ) -> Result<UpstreamConnection, anyhow::Error> {
        tracing::info!("Creating temporary validation instance for server: {}", server_name);

        // Get database connection
        let db = self.database.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not available"))?;

        // Get server configuration from database
        let server = crate::config::server::get_server(&db.pool, server_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_name))?;

        // Convert database Server to MCPServerConfig (reusing existing conversion logic)
        let server_config = self.convert_server_to_config(&server, &db.pool).await?;

        // Create temporary connection instance with validation prefix
        let instance_id = format!("validation-{}-{}", server_name, session_id);
        let mut connection = crate::core::connection::UpstreamConnection::new(instance_id);

        // Set validation status to distinguish from production instances
        connection.status = ConnectionStatus::Validating;

        // Connect to server using unified transport interface
        let (service, tools, capabilities, _process_id) =
            crate::core::transport::unified::connect_server(
                server_name,
                &server_config,
                server.server_type,
                server_config.transport_type.unwrap_or_default(),
                None, // No cancellation token needed for short-lived validation
                Some(&db.pool),
                self.runtime_cache.as_ref().map(|rc| rc.as_ref()),
            ).await?;

        // Update connection with service and capabilities
        connection.update_connected(service, tools, capabilities);

        tracing::info!("Created temporary validation instance for server '{}'", server_name);
        Ok(connection)
    }

    /// Convert database Server model to MCPServerConfig
    ///
    /// Reuses the conversion logic from core/foundation/loader.rs
    async fn convert_server_to_config(
        &self,
        server: &crate::config::models::Server,
        pool: &sqlx::Pool<sqlx::Sqlite>,
    ) -> Result<crate::core::models::MCPServerConfig, anyhow::Error> {
        // Get server arguments (reusing existing logic)
        let args = if let Some(id) = &server.id {
            let server_args = crate::config::server::get_server_args(pool, id).await?;
            if server_args.is_empty() {
                None
            } else {
                let mut sorted_args: Vec<_> = server_args.into_iter().collect();
                sorted_args.sort_by_key(|arg| arg.arg_index);
                Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect())
            }
        } else {
            None
        };

        // Get server environment variables (reusing existing logic)
        let env = if let Some(id) = &server.id {
            let env_map = crate::config::server::get_server_env(pool, id).await?;
            if env_map.is_empty() {
                None
            } else {
                Some(env_map)
            }
        } else {
            None
        };

        // Create MCPServerConfig (reusing existing structure)
        Ok(crate::core::models::MCPServerConfig {
            kind: server.server_type,
            command: server.command.clone(),
            args,
            url: server.url.clone(),
            env,
            transport_type: server.transport_type,
        })
    }

    /// Destroy a validation instance after use
    ///
    /// This implements the "immediate cleanup" part of the create-use-destroy lifecycle
    pub async fn destroy_validation_instance(
        &mut self,
        server_name: &str,
        session_id: &str,
    ) -> Result<(), anyhow::Error> {
        if let Some(session_servers) = self.validation_sessions.get_mut(session_id) {
            if let Some(mut connection) = session_servers.remove(server_name) {
                // Disconnect the service if still connected
                if connection.is_connected() {
                    connection.update_disconnected();
                }
                tracing::info!("Destroyed validation instance for server '{}'", server_name);
            }
        }
        Ok(())
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
