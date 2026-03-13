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
/// - Loading server configurations from active profile
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

    /// Sync all servers in the connection pool based on active profile
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
    pub async fn sync_servers_from_active_profile(
        &self,
        pool: &mut UpstreamConnectionPool,
    ) -> Result<()> {
        tracing::debug!("Starting server synchronization from active profile");

        // Step 1: Load current active configuration
        let config = self.load_active_configuration().await?;

        // Step 2: Update connection pool configuration
        pool.set_config(Arc::new(config))?;

        // Step 3: Calculate required server state changes
        let sync_plan = self.calculate_sync_plan(pool)?;

        // Step 4: Execute the synchronization plan
        self.execute_sync_plan(pool, sync_plan).await?;

        tracing::info!("Server synchronization completed successfully");
        Ok(())
    }

    /// Load the current active configuration from database
    async fn load_active_configuration(&self) -> Result<Config> {
        tracing::debug!("Loading server configuration from active profile");

        let (_, config) = crate::core::foundation::loader::load_servers_from_active_profile(&self.database)
            .await
            .context("Failed to load servers from active profile")?;

        tracing::debug!("Loaded configuration with {} servers", config.mcp_servers.len());
        Ok(config)
    }

    /// Calculate what changes need to be made to synchronize the pool
    fn calculate_sync_plan(
        &self,
        pool: &UpstreamConnectionPool,
    ) -> Result<ServerSyncPlan> {
        let required_servers: HashSet<String> = pool.config.mcp_servers.keys().cloned().collect();
        let current_servers: HashSet<String> = pool.connections.keys().cloned().collect();

        let servers_to_start: HashSet<String> = required_servers.difference(&current_servers).cloned().collect();
        let servers_to_stop: HashSet<String> = current_servers.difference(&required_servers).cloned().collect();
        let servers_to_check: HashSet<String> = required_servers.intersection(&current_servers).cloned().collect();

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
        // Start new servers (lazy: create placeholder instances without connecting)
        for server_name in plan.servers_to_start {
            tracing::info!(
                "Registering server '{}' for lazy startup (connection deferred)",
                server_name
            );

            let instances = pool.connections.entry(server_name.clone()).or_default();
            if instances.is_empty() {
                let connection = crate::core::pool::UpstreamConnection::new(server_name.clone());
                instances.insert(connection.id.clone(), connection);
            }

            if let Some(instance) = instances.values_mut().next() {
                if matches!(
                    instance.status,
                    crate::core::foundation::types::ConnectionStatus::Shutdown
                ) {
                    instance.status = crate::core::foundation::types::ConnectionStatus::Idle;
                }
            }
        }

        // Stop removed servers
        for server_name in plan.servers_to_stop {
            tracing::info!("Stopping removed server: {}", server_name);
            if let Err(e) = pool.update_server_status(&server_name, false).await {
                tracing::warn!("Failed to stop removed server '{}': {}", server_name, e);
            }
        }

        // Existing servers keep placeholder, wait for first demand to trigger connection
        for server_name in plan.servers_to_connect {
            let instances = pool.connections.entry(server_name.clone()).or_default();
            if instances.is_empty() {
                let connection = crate::core::pool::UpstreamConnection::new(server_name.clone());
                instances.insert(connection.id.clone(), connection);
            }

            if let Some(instance) = instances.values_mut().next() {
                if matches!(
                    instance.status,
                    crate::core::foundation::types::ConnectionStatus::Shutdown
                        | crate::core::foundation::types::ConnectionStatus::Initializing
                ) {
                    instance.status = crate::core::foundation::types::ConnectionStatus::Idle;
                }
            }

            tracing::info!(
                "Server '{}' kept idle; connection will be established on first demand",
                server_name
            );
        }

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
    /// Servers exist but are not connected (keep placeholder, trigger connection on demand)
    servers_to_connect: HashSet<String>,
}
