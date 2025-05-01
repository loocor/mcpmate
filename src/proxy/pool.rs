// MCP Proxy connection pool module
// Contains the UpstreamConnectionPool struct and related functionality

use anyhow::{self, Context, Result};
use rmcp::{model::Tool, service::RunningService, RoleClient};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use tracing;

use super::{
    connect_sse_server, connect_stdio_server, connection::UpstreamConnection,
    types::ConnectionStatus,
};
use crate::config::Config;

/// Pool of connections to upstream MCP servers
#[derive(Debug, Clone)]
pub struct UpstreamConnectionPool {
    /// Map of server name to connection
    pub connections: HashMap<String, UpstreamConnection>,
    /// Server configuration
    pub config: Arc<Config>,
    /// Rule configuration
    pub rule_config: Arc<HashMap<String, bool>>,
}

impl UpstreamConnectionPool {
    /// Create a new connection pool
    pub fn new(config: Arc<Config>, rule_config: Arc<HashMap<String, bool>>) -> Self {
        Self {
            connections: HashMap::new(),
            config,
            rule_config,
        }
    }

    /// Initialize the connection pool with all enabled servers
    pub fn initialize(&mut self) {
        for (name, _server_config) in &self.config.mcp_servers {
            // Skip the proxy server itself
            if name == "proxy" {
                continue;
            }

            // Check if the server is enabled in the rule configuration
            let enabled = self.rule_config.get(name).copied().unwrap_or(false);
            if !enabled {
                tracing::info!("Server '{}' is disabled, skipping", name);
                continue;
            }

            // Create a new connection
            self.connections
                .insert(name.clone(), UpstreamConnection::new(name.clone()));
        }

        tracing::info!(
            "Initialized connection pool with {} enabled servers",
            self.connections.len()
        );
    }

    /// Trigger connection to all servers in the pool without waiting for completion
    pub fn trigger_connect_all(&mut self) {
        // Get all server names
        let server_names: Vec<String> = self.connections.keys().cloned().collect();

        // Trigger connection for each server
        for name in server_names {
            if let Err(e) = self.trigger_connect(&name) {
                tracing::warn!("Failed to trigger connection to server '{}': {}", name, e);
            }
        }
    }

    /// Trigger a connection to a specific server without waiting for completion
    pub fn trigger_connect(&mut self, server_name: &str) -> Result<()> {
        // Check if the server exists
        let conn = self.connections.get(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        // Avoid connecting if already connecting
        if matches!(conn.status, ConnectionStatus::Connecting) {
            return Ok(());
        }

        // Check if the server is disabled or paused
        if matches!(
            conn.status,
            ConnectionStatus::Disabled | ConnectionStatus::Paused
        ) {
            return Err(anyhow::anyhow!(
                "Server '{}' is {} and cannot be connected",
                server_name,
                conn.status
            ));
        }

        // Update status and increment connection attempts
        {
            let conn = self.connections.get_mut(server_name).unwrap();
            conn.update_connecting();
        }

        tracing::info!("Triggering connection to server '{}'...", server_name);

        // Get the server type
        let server_type = {
            let server_config = self.config.mcp_servers.get(server_name).unwrap();
            server_config.kind.clone()
        };

        // Clone necessary data for the background task
        let server_name = server_name.to_string();
        let pool = Arc::new(Mutex::new(self.clone()));

        // Spawn a background task to perform the actual connection
        tokio::spawn(async move {
            // This is a background task that will connect to the server
            let result = match server_type.as_str() {
                "stdio" => {
                    tracing::info!(
                        "Background task: Connecting to stdio server '{}'...",
                        server_name
                    );
                    // Server configuration is already in config_clone

                    // Connect to the server
                    let mut pool_guard = pool.lock().await;
                    pool_guard.connect_stdio(&server_name).await
                }
                "sse" => {
                    tracing::info!(
                        "Background task: Connecting to SSE server '{}'...",
                        server_name
                    );
                    // Server configuration is already in config_clone

                    // Connect to the server
                    let mut pool_guard = pool.lock().await;
                    pool_guard.connect_sse(&server_name).await
                }
                _ => {
                    let error_msg = format!("Unsupported server type: {}", server_type);
                    Err(anyhow::anyhow!(error_msg))
                }
            };

            // Log the result
            match &result {
                Ok(_) => tracing::info!("Background task: Connected to server '{}'", server_name),
                Err(e) => {
                    tracing::error!(
                        "Background task: Failed to connect to server '{}': {}",
                        server_name,
                        e
                    );

                    // Update connection status to failed
                    if let Ok(mut pool_guard) = pool.try_lock() {
                        if let Some(conn) = pool_guard.connections.get_mut(&server_name) {
                            conn.update_failed(e.to_string());
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Connect to a specific server (blocking version)
    pub async fn connect(&mut self, server_name: &str) -> Result<()> {
        // Check if we should connect
        {
            let conn = self.connections.get(server_name).context(format!(
                "Server '{}' not found in connection pool",
                server_name
            ))?;

            // Avoid connecting if already connecting
            if matches!(conn.status, ConnectionStatus::Connecting) {
                return Ok(());
            }
        };

        // Update status and increment connection attempts
        {
            let conn = self.connections.get_mut(server_name).unwrap();
            conn.update_connecting();
        }

        tracing::info!("Connecting to server '{}'...", server_name);

        // Get the server type
        let server_type = {
            let server_config = self.config.mcp_servers.get(server_name).unwrap();
            server_config.kind.clone()
        };

        // Connect based on server type
        let result = match server_type.as_str() {
            "stdio" => self.connect_stdio(server_name).await,
            "sse" => self.connect_sse(server_name).await,
            _ => {
                let error_msg = format!("Unsupported server type: {}", server_type);
                let conn = self.connections.get_mut(server_name).unwrap();
                conn.update_failed(error_msg.clone());
                Err(anyhow::anyhow!(error_msg))
            }
        };

        // Handle connection result
        if let Err(e) = &result {
            let conn = self.connections.get_mut(server_name).unwrap();
            conn.update_failed(e.to_string());
            tracing::error!("Failed to connect to server '{}': {}", server_name, e);
        }

        result
    }

    /// Helper function to update connection after successful connection
    fn update_connection(
        &mut self,
        server_name: &str,
        service: RunningService<RoleClient, ()>,
        tools: Vec<Tool>,
    ) {
        let conn = self.connections.get_mut(server_name).unwrap();
        conn.update_connected(service, tools);
        tracing::info!(
            "Connected to server '{}', found {} tools",
            server_name,
            conn.tools.len()
        );
    }

    /// Connect to a stdio server
    async fn connect_stdio(&mut self, server_name: &str) -> Result<()> {
        // Get server configuration
        let server_config = self.config.mcp_servers.get(server_name).unwrap();

        // Connect to the server using the proxy module function
        match connect_stdio_server(server_name, server_config).await {
            Ok((service, tools)) => {
                // Update connection
                self.update_connection(server_name, service, tools);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Connect to an SSE server
    async fn connect_sse(&mut self, server_name: &str) -> Result<()> {
        // Get server configuration
        let server_config = self.config.mcp_servers.get(server_name).unwrap();

        // Connect to the server using the proxy module function
        match connect_sse_server(server_name, server_config).await {
            Ok((service, tools)) => {
                // Update connection
                self.update_connection(server_name, service, tools);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Disconnect from a server
    pub async fn disconnect(&mut self, server_name: &str) -> Result<()> {
        let conn = self.connections.get_mut(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        // If there's an active service, take it and cancel it
        if let Some(service) = conn.service.take() {
            if let Err(e) = service.cancel().await {
                tracing::warn!("Error cancelling service for '{}': {}", server_name, e);
            }
        }

        // Update connection status
        conn.update_disconnected();
        tracing::info!("Disconnected from server '{}'", server_name);

        Ok(())
    }

    /// Reconnect to a server
    pub async fn reconnect(&mut self, server_name: &str) -> Result<()> {
        // First disconnect
        self.disconnect(server_name).await?;

        // Get connection for backoff calculation
        let conn = self.connections.get(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        // Calculate backoff time using exponential backoff
        let backoff = std::cmp::min(
            30,                                                   // Maximum 30 seconds
            2u64.pow(std::cmp::min(5, conn.connection_attempts)), // Exponential backoff, max 2^5=32 seconds
        );

        tracing::info!(
            "Waiting {}s before reconnecting to '{}'",
            backoff,
            server_name
        );
        sleep(Duration::from_secs(backoff)).await;

        // Reconnect
        self.connect(server_name).await
    }

    /// Connect to all servers in parallel
    pub async fn connect_all(&mut self) -> Result<()> {
        // First trigger connection for all servers without waiting
        self.trigger_connect_all();

        // Return immediately, connections will happen in the background
        Ok(())
    }

    /// Disconnect from all servers
    pub async fn disconnect_all(&mut self) -> Result<()> {
        for server_name in self.connections.keys().cloned().collect::<Vec<_>>() {
            if let Err(e) = self.disconnect(&server_name).await {
                tracing::error!("Failed to disconnect from server '{}': {}", server_name, e);
            }
        }
        Ok(())
    }

    /// Disable a server (manually prevent connections)
    pub async fn disable_server(&mut self, server_name: &str) -> Result<()> {
        // Check if already disabled
        {
            let conn = self.connections.get(server_name).context(format!(
                "Server '{}' not found in connection pool",
                server_name
            ))?;

            // If already disabled, do nothing
            if matches!(conn.status, ConnectionStatus::Disabled) {
                tracing::info!("Server '{}' is already disabled", server_name);
                return Ok(());
            }

            // Check if connected
            if conn.is_connected() {
                // Disconnect first (in a separate scope)
                self.disconnect(server_name).await?;
            }
        }

        // Now update status to disabled
        let conn = self.connections.get_mut(server_name).unwrap();
        conn.update_disabled();
        tracing::info!("Server '{}' has been disabled", server_name);

        Ok(())
    }

    /// Enable a server (allow connections)
    pub async fn enable_server(&mut self, server_name: &str) -> Result<()> {
        let conn = self.connections.get_mut(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        // If not disabled or paused, do nothing
        if !matches!(
            conn.status,
            ConnectionStatus::Disabled | ConnectionStatus::Paused
        ) {
            tracing::info!("Server '{}' is already enabled", server_name);
            return Ok(());
        }

        // Update status to disconnected (ready to connect)
        conn.update_disconnected();
        tracing::info!("Server '{}' has been enabled", server_name);

        Ok(())
    }

    /// Pause a server (temporarily prevent connections)
    pub async fn pause_server(&mut self, server_name: &str) -> Result<()> {
        let conn = self.connections.get_mut(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        // If already paused, do nothing
        if matches!(conn.status, ConnectionStatus::Paused) {
            tracing::info!("Server '{}' is already paused", server_name);
            return Ok(());
        }

        // Update status to paused
        conn.update_paused();
        tracing::info!("Server '{}' has been paused", server_name);

        Ok(())
    }

    /// Get server status
    pub fn get_server_status(&self, server_name: &str) -> Result<String> {
        let conn = self.connections.get(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        Ok(conn.status_string())
    }

    /// Get all server statuses
    pub fn get_all_server_statuses(&self) -> HashMap<String, String> {
        self.connections
            .iter()
            .map(|(name, conn)| (name.clone(), conn.status_string()))
            .collect()
    }

    /// Start health check task
    pub fn start_health_check(connection_pool: Arc<Mutex<Self>>) {
        tokio::spawn(async move {
            let pool_clone = connection_pool.clone();

            loop {
                // Wait for health check interval
                sleep(Duration::from_secs(30)).await;

                // Check all connections
                let mut reconnect_servers = Vec::new();
                {
                    let pool_guard = pool_clone.lock().await;
                    for (name, conn) in &pool_guard.connections {
                        // Only monitor servers that should be monitored
                        if !conn.should_monitor() {
                            continue;
                        }

                        match &conn.status {
                            ConnectionStatus::Connected => {
                                // Check if the service is still alive
                                if let Some(_service) = &conn.service {
                                    // We can't directly check if the service is closed
                                    // Instead, we'll periodically try to reconnect to ensure health
                                    if conn.time_since_last_connection() > Duration::from_secs(300)
                                    {
                                        tracing::info!(
                                            "Health check: Periodic reconnect for '{}'",
                                            name
                                        );
                                        reconnect_servers.push(name.clone());
                                    }
                                } else {
                                    // If service is None but status is Connected, something is wrong
                                    tracing::warn!("Health check: Server '{}' has Connected status but no service, will reconnect", name);
                                    reconnect_servers.push(name.clone());
                                }
                            }
                            ConnectionStatus::Failed(_) => {
                                // Reconnect failed servers after a delay
                                if conn.time_since_last_connection() > Duration::from_secs(60) {
                                    reconnect_servers.push(name.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Reconnect servers that need it
                for server_name in reconnect_servers {
                    tracing::info!("Health check: Attempting to reconnect to '{}'", server_name);
                    let mut pool_guard = pool_clone.lock().await;
                    if let Err(e) = pool_guard.reconnect(&server_name).await {
                        tracing::warn!(
                            "Health check: Failed to reconnect to '{}': {}",
                            server_name,
                            e
                        );
                    }
                }
            }
        });
    }
}
