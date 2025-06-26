//! Server synchronization manager
//!
//! Handles the business logic for synchronizing servers based on configuration changes.
//! This includes loading configurations from database, comparing server states,
//! and managing server lifecycle (start/stop/connect).

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing;

use super::UpstreamConnectionPool;
use crate::config::database::Database;
use crate::core::models::Config;

/// Manager for synchronizing server configurations and states
///
/// This manager handles the business logic of:
/// - Loading server configurations from active configuration suites
/// - Comparing required vs current server states
/// - Managing server lifecycle (start/stop/connect)
/// - Ensuring proper server state transitions
#[derive(Debug)]
pub struct ServerSyncManager {
    /// Database reference for loading configurations
    database: Arc<Database>,
}

impl ServerSyncManager {
    /// Create a new server sync manager
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Sync all servers in the connection pool based on active configuration suites
    ///
    /// This is the main entry point for server synchronization. It:
    /// 1. Loads the current active configuration from database
    /// 2. Updates the connection pool configuration
    /// 3. Calculates required server state changes
    /// 4. Executes the necessary server lifecycle operations
    ///
    /// # Arguments
    /// * `pool` - Mutable reference to the connection pool to sync
    ///
    /// # Returns
    /// * `Ok(())` - If synchronization completed successfully
    /// * `Err(...)` - If any step of synchronization failed
    pub async fn sync_servers_from_active_suites(
        &self,
        pool: &mut UpstreamConnectionPool,
    ) -> Result<()> {
        tracing::debug!("Starting server synchronization from active configuration suites");

        // Step 1: Load current active configuration
        let config = self.load_active_configuration().await?;

        // Step 2: Update connection pool configuration
        pool.set_config(Arc::new(config));

        // Step 3: Calculate required server state changes
        let sync_plan = self.calculate_sync_plan(pool)?;

        // Step 4: Execute the synchronization plan
        self.execute_sync_plan(pool, sync_plan).await?;

        tracing::info!("Server synchronization completed successfully");
        Ok(())
    }

    /// Load the current active configuration from database
    async fn load_active_configuration(&self) -> Result<Config> {
        tracing::debug!("Loading server configuration from active configuration suites");

        let (_, config) =
            crate::core::foundation::loader::load_servers_from_active_suits(&self.database)
                .await
                .context("Failed to load servers from active configuration suites")?;

        tracing::debug!(
            "Loaded configuration with {} servers",
            config.mcp_servers.len()
        );
        Ok(config)
    }

    /// Calculate what changes need to be made to synchronize the pool
    fn calculate_sync_plan(
        &self,
        pool: &UpstreamConnectionPool,
    ) -> Result<ServerSyncPlan> {
        let required_servers: HashSet<String> = pool.config.mcp_servers.keys().cloned().collect();
        let current_servers: HashSet<String> = pool.connections.keys().cloned().collect();

        let servers_to_start: HashSet<String> = required_servers
            .difference(&current_servers)
            .cloned()
            .collect();
        let servers_to_stop: HashSet<String> = current_servers
            .difference(&required_servers)
            .cloned()
            .collect();
        let servers_to_check: HashSet<String> = required_servers
            .intersection(&current_servers)
            .cloned()
            .collect();

        // Filter servers that need connection (existing but not connected)
        let mut servers_to_connect = HashSet::new();
        for server_name in &servers_to_check {
            let needs_connection = if let Some(instances) = pool.connections.get(server_name) {
                instances.values().all(|conn| !conn.is_connected())
            } else {
                true // No instances exist, definitely needs connection
            };

            if needs_connection {
                servers_to_connect.insert(server_name.clone());
            }
        }

        let plan = ServerSyncPlan {
            servers_to_start,
            servers_to_stop,
            servers_to_connect,
        };

        tracing::debug!(
            "Calculated sync plan: {} to start, {} to stop, {} to connect",
            plan.servers_to_start.len(),
            plan.servers_to_stop.len(),
            plan.servers_to_connect.len()
        );

        Ok(plan)
    }

    /// Execute the calculated synchronization plan
    async fn execute_sync_plan(
        &self,
        pool: &mut UpstreamConnectionPool,
        plan: ServerSyncPlan,
    ) -> Result<()> {
        // Start new servers
        for server_name in plan.servers_to_start {
            tracing::info!("Starting new server: {}", server_name);
            if let Err(e) = pool.update_server_status(&server_name, true).await {
                tracing::warn!("Failed to start new server '{}': {}", server_name, e);
            }
        }

        // Stop removed servers
        for server_name in plan.servers_to_stop {
            tracing::info!("Stopping removed server: {}", server_name);
            if let Err(e) = pool.update_server_status(&server_name, false).await {
                tracing::warn!("Failed to stop removed server '{}': {}", server_name, e);
            }
        }

        // Connect existing servers that are in shutdown state
        for server_name in plan.servers_to_connect {
            tracing::info!(
                "Connecting existing server in shutdown state: {}",
                server_name
            );
            if let Err(e) = self.ensure_server_connected(pool, &server_name).await {
                tracing::warn!("Failed to connect existing server '{}': {}", server_name, e);
            }
        }

        Ok(())
    }

    /// Ensure a server is connected (create instance if needed and connect)
    ///
    /// This method handles the detailed logic of ensuring a server has a connected instance:
    /// 1. Create instance if it doesn't exist
    /// 2. Get the default instance ID
    /// 3. Trigger connection for that instance
    async fn ensure_server_connected(
        &self,
        pool: &mut UpstreamConnectionPool,
        server_name: &str,
    ) -> Result<()> {
        // Create instance if it doesn't exist
        if !pool.connections.contains_key(server_name) {
            let connection =
                crate::core::connection::UpstreamConnection::new(server_name.to_string());
            let instance_id = connection.id.clone();
            let instances = pool.connections.entry(server_name.to_string()).or_default();
            instances.insert(instance_id, connection);
        }

        // Get default instance ID and trigger connection
        let instance_id = pool.get_default_instance_id(server_name)?;
        pool.trigger_connect(server_name, &instance_id).await?;

        tracing::info!("Successfully ensured server '{}' is connected", server_name);
        Ok(())
    }
}

/// Plan for synchronizing servers
///
/// This struct represents the calculated changes needed to bring the connection pool
/// into sync with the required configuration.
#[derive(Debug, Clone)]
struct ServerSyncPlan {
    /// Servers that need to be started (new servers)
    servers_to_start: HashSet<String>,
    /// Servers that need to be stopped (removed servers)
    servers_to_stop: HashSet<String>,
    /// Servers that exist but need to be connected (shutdown servers)
    servers_to_connect: HashSet<String>,
}
