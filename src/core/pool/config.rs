//! Configuration manager for connection pool
//!
//! Handles configuration updates and validation for the connection pool.
//! Separates configuration management logic from the core pool operations.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use tracing;

use crate::core::{connection::UpstreamConnection, models::Config};

/// Manager for handling connection pool configuration updates
///
/// This manager is responsible for:
/// - Validating configuration changes
/// - Updating pool state based on new configurations
/// - Managing server instance creation and cleanup
/// - Ensuring configuration consistency
#[derive(Debug, Default)]
pub struct PoolConfigManager;

impl PoolConfigManager {
    /// Create a new pool configuration manager
    pub fn new() -> Self {
        Self
    }

    /// Update the connection pool configuration
    ///
    /// This method handles the complex logic of updating a connection pool's configuration:
    /// 1. Validates the new configuration
    /// 2. Determines which servers need to be added/removed
    /// 3. Creates new server instances for added servers
    /// 4. Cleans up instances for removed servers
    /// 5. Preserves existing connections where possible
    ///
    /// # Arguments
    /// * `connections` - Mutable reference to the connection map
    /// * `cancellation_tokens` - Mutable reference to the cancellation token map
    /// * `new_config` - The new configuration to apply
    ///
    /// # Returns
    /// * `Ok(())` - If configuration update completed successfully
    /// * `Err(...)` - If configuration validation or update failed
    pub fn update_configuration(
        connections: &mut HashMap<String, HashMap<String, UpstreamConnection>>,
        cancellation_tokens: &mut HashMap<
            String,
            HashMap<String, tokio_util::sync::CancellationToken>,
        >,
        new_config: Arc<Config>,
    ) -> Result<()> {
        tracing::debug!("Updating connection pool configuration");

        // Step 1: Validate the new configuration
        Self::validate_configuration(&new_config)?;

        // Step 2: Calculate configuration changes
        let changes = Self::calculate_configuration_changes(connections, &new_config);

        // Step 3: Apply the changes
        Self::apply_configuration_changes(connections, cancellation_tokens, changes)?;

        tracing::info!(
            "Updated connection pool configuration with {} servers",
            connections.len()
        );

        Ok(())
    }

    /// Initialize connections for a new configuration
    ///
    /// This method creates initial server instances for all servers in the configuration.
    /// It's typically called when setting up a new connection pool.
    ///
    /// # Arguments
    /// * `connections` - Mutable reference to the connection map (should be empty)
    /// * `config` - The configuration to initialize from
    pub fn initialize_connections(
        connections: &mut HashMap<String, HashMap<String, UpstreamConnection>>,
        config: &Config,
    ) {
        tracing::debug!(
            "Initializing connections for {} servers",
            config.mcp_servers.len()
        );

        for server_id in config.mcp_servers.keys() {
            // Create a new connection instance
            Self::create_server_instance(connections, server_id);
        }

        // Count total instances
        let total_instances: usize = connections.values().map(|instances| instances.len()).sum();

        tracing::info!(
            "Initialized connection pool with {} servers and {} instances",
            connections.len(),
            total_instances
        );
    }

    /// Validate a configuration before applying it
    fn validate_configuration(config: &Config) -> Result<()> {
        // Basic validation - ensure no duplicate server names
        let server_names: Vec<&String> = config.mcp_servers.keys().collect();
        let unique_names: HashSet<&String> = server_names.iter().cloned().collect();

        if server_names.len() != unique_names.len() {
            return Err(anyhow::anyhow!(
                "Configuration contains duplicate server names"
            ));
        }

        // Additional validation can be added here
        // - Check for valid server configurations
        // - Validate transport settings
        // - Check for required fields

        tracing::debug!(
            "Configuration validation passed for {} servers",
            config.mcp_servers.len()
        );
        Ok(())
    }

    /// Calculate what changes need to be made to apply a new configuration
    fn calculate_configuration_changes(
        current_connections: &HashMap<String, HashMap<String, UpstreamConnection>>,
        new_config: &Config,
    ) -> ConfigurationChanges {
        let current_servers: HashSet<String> = current_connections.keys().cloned().collect();
        let new_servers: HashSet<String> = new_config
            .mcp_servers
            .keys()
            .filter(|&name| name != "proxy") // Skip proxy server
            .cloned()
            .collect();

        let servers_to_add: HashSet<String> =
            new_servers.difference(&current_servers).cloned().collect();
        let servers_to_remove: HashSet<String> =
            current_servers.difference(&new_servers).cloned().collect();
        let servers_to_keep: HashSet<String> = current_servers
            .intersection(&new_servers)
            .cloned()
            .collect();

        ConfigurationChanges {
            servers_to_add,
            servers_to_remove,
            servers_to_keep,
        }
    }

    /// Apply the calculated configuration changes
    fn apply_configuration_changes(
        connections: &mut HashMap<String, HashMap<String, UpstreamConnection>>,
        cancellation_tokens: &mut HashMap<
            String,
            HashMap<String, tokio_util::sync::CancellationToken>,
        >,
        changes: ConfigurationChanges,
    ) -> Result<()> {
        // Remove servers that are no longer in the configuration
        for server_name in &changes.servers_to_remove {
            connections.remove(server_name);
            cancellation_tokens.remove(server_name);
            tracing::debug!("Removed server '{}' from connection pool", server_name);
        }

        // Add new servers from the configuration
        for server_id in &changes.servers_to_add {
            Self::create_server_instance(connections, server_id);
            tracing::debug!("Added server '{}' to connection pool", server_id);
        }

        // Servers to keep remain unchanged (preserve existing connections)
        tracing::debug!(
            "Configuration changes applied: {} added, {} removed, {} kept",
            changes.servers_to_add.len(),
            changes.servers_to_remove.len(),
            changes.servers_to_keep.len()
        );

        Ok(())
    }

    /// Create a new server instance in the connection pool
    fn create_server_instance(
        connections: &mut HashMap<String, HashMap<String, UpstreamConnection>>,
        server_id: &str,
    ) {
        // Skip if connection already exists
        if connections.contains_key(server_id) {
            return;
        }

        // Create a new connection (still use server_id as the name for now)
        let connection = UpstreamConnection::new(server_id.to_string());
        let instance_id = connection.id.clone();

        // Create a new map for this server
        let instances = connections.entry(server_id.to_string()).or_default();

        // Add the connection to the map
        instances.insert(instance_id, connection);
    }
}

/// Represents the changes needed to update a configuration
#[derive(Debug, Clone)]
struct ConfigurationChanges {
    /// Servers that need to be added to the pool
    servers_to_add: HashSet<String>,
    /// Servers that need to be removed from the pool
    servers_to_remove: HashSet<String>,
    /// Servers that should be kept (no changes needed)
    servers_to_keep: HashSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config(server_names: Vec<&str>) -> Config {
        let mut mcp_servers = HashMap::new();
        for name in server_names {
            mcp_servers.insert(
                name.to_string(),
                crate::core::models::MCPServerConfig {
                    kind: crate::common::server::ServerType::Stdio,
                    command: Some("test-command".to_string()),
                    args: Some(vec!["arg1".to_string()]),
                    url: None,
                    env: None,
                    transport_type: Some(crate::common::server::TransportType::Stdio),
                },
            );
        }

        Config {
            mcp_servers,
            pagination: None,
        }
    }

    #[test]
    fn test_configuration_validation() {
        let config = create_test_config(vec!["server1", "server2"]);
        assert!(PoolConfigManager::validate_configuration(&config).is_ok());
    }

    #[test]
    fn test_configuration_changes_calculation() {
        let mut connections = HashMap::new();
        connections.insert("server1".to_string(), HashMap::new());
        connections.insert("server2".to_string(), HashMap::new());

        let new_config = create_test_config(vec!["server2", "server3"]);
        let changes = PoolConfigManager::calculate_configuration_changes(&connections, &new_config);

        assert_eq!(changes.servers_to_add.len(), 1);
        assert!(changes.servers_to_add.contains("server3"));
        assert_eq!(changes.servers_to_remove.len(), 1);
        assert!(changes.servers_to_remove.contains("server1"));
        assert_eq!(changes.servers_to_keep.len(), 1);
        assert!(changes.servers_to_keep.contains("server2"));
    }

    #[test]
    fn test_initialize_connections() {
        let mut connections = HashMap::new();
        let config = create_test_config(vec!["server1", "server2", "proxy"]);

        PoolConfigManager::initialize_connections(&mut connections, &config);

        // Should have 2 servers (proxy is skipped)
        assert_eq!(connections.len(), 2);
        assert!(connections.contains_key("server1"));
        assert!(connections.contains_key("server2"));
        assert!(!connections.contains_key("proxy"));
    }
}
