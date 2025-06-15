// MCP Proxy connection pool module
// Contains the UpstreamConnectionPool struct and related functionality

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{self, Context, Result};
use tokio_util::sync::CancellationToken;
use tracing;

use crate::core::{connection::UpstreamConnection, models::Config, monitor::ProcessMonitor};

// Import submodules
mod connection;
mod health;
mod monitoring;
mod parallel;
mod sync;
mod utils;

/// Pool of connections to upstream MCP servers
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
    /// Database reference for checking server status
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

    /// Update the configuration
    pub fn set_config(
        &mut self,
        config: Arc<Config>,
    ) {
        self.config = config;
    }

    /// Initialize the connection pool with all servers
    pub fn initialize(&mut self) {
        for name in self.config.mcp_servers.keys() {
            // Skip the proxy server itself
            if name == "proxy" {
                continue;
            }

            // Create a new connection
            let connection = UpstreamConnection::new(name.clone());
            let instance_id = connection.id.clone();

            // Create a new map for this server if it doesn't exist
            let instances = self.connections.entry(name.clone()).or_default();

            // Add the connection to the map
            instances.insert(instance_id, connection);
        }

        // Count total instances
        let total_instances: usize = self
            .connections
            .values()
            .map(|instances| instances.len())
            .sum();

        tracing::info!(
            "Initialized connection pool with {} servers and {} instances",
            self.connections.len(),
            total_instances
        );
    }

    /// Helper method to get a specific instance of a server
    pub fn get_instance(
        &self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<&UpstreamConnection> {
        let instances = self.connections.get(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        instances.get(instance_id).context(format!(
            "Instance '{instance_id}' not found for server '{server_name}'"
        ))
    }

    /// Helper method to get a mutable reference to a specific instance of a server
    pub fn get_instance_mut(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<&mut UpstreamConnection> {
        let instances = self.connections.get_mut(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        instances.get_mut(instance_id).context(format!(
            "Instance '{instance_id}' not found for server '{server_name}'"
        ))
    }

    /// Helper method to get the default instance of a server
    pub fn get_default_instance(
        &self,
        server_name: &str,
    ) -> Result<(String, &UpstreamConnection)> {
        let instances = self.connections.get(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        if instances.is_empty() {
            return Err(anyhow::anyhow!(
                "No instances found for server '{}'",
                server_name
            ));
        }

        // Get the first instance (for now, we'll just use the first one as default)
        let (instance_id, connection) = instances.iter().next().unwrap();
        Ok((instance_id.clone(), connection))
    }

    /// Helper method to get a mutable reference to the default instance of a server
    pub fn get_default_instance_mut(
        &mut self,
        server_name: &str,
    ) -> Result<(String, &mut UpstreamConnection)> {
        let instances = self.connections.get_mut(server_name).context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

        if instances.is_empty() {
            return Err(anyhow::anyhow!(
                "No instances found for server '{}'",
                server_name
            ));
        }

        // Get the first instance (for now, we'll just use the first one as default)
        let instance_id = instances.keys().next().unwrap().clone();
        let connection = instances.get_mut(&instance_id).unwrap();
        Ok((instance_id, connection))
    }
}
