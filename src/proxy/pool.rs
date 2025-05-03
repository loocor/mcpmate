// MCP Proxy connection pool module
// Contains the UpstreamConnectionPool struct and related functionality

use anyhow::{self, Context, Result};
use rmcp::{model::Tool, service::RunningService, RoleClient};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing;

use super::{connect_sse_server, connection::UpstreamConnection, types::ConnectionStatus};
use crate::config::Config;

/// Pool of connections to upstream MCP servers
#[derive(Debug, Clone)]
pub struct UpstreamConnectionPool {
    /// Map of server name to map of instance ID to connection
    pub connections: HashMap<String, HashMap<String, UpstreamConnection>>,
    /// Server configuration
    pub config: Arc<Config>,
    /// Rule configuration
    pub rule_config: Arc<HashMap<String, bool>>,
    /// Map of server name to map of instance ID to cancellation token
    pub cancellation_tokens: HashMap<String, HashMap<String, CancellationToken>>,
}

impl UpstreamConnectionPool {
    /// Create a new connection pool
    pub fn new(config: Arc<Config>, rule_config: Arc<HashMap<String, bool>>) -> Self {
        Self {
            connections: HashMap::new(),
            config,
            rule_config,
            cancellation_tokens: HashMap::new(),
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
            let connection = UpstreamConnection::new(name.clone());
            let instance_id = connection.id.clone();

            // Create a new map for this server if it doesn't exist
            let instances = self
                .connections
                .entry(name.clone())
                .or_insert_with(HashMap::new);

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
            "Initialized connection pool with {} enabled servers and {} instances",
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
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        instances.get(instance_id).context(format!(
            "Instance '{}' not found for server '{}'",
            instance_id, server_name
        ))
    }

    /// Helper method to get a mutable reference to a specific instance of a server
    pub fn get_instance_mut(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<&mut UpstreamConnection> {
        let instances = self.connections.get_mut(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
        ))?;

        instances.get_mut(instance_id).context(format!(
            "Instance '{}' not found for server '{}'",
            instance_id, server_name
        ))
    }

    /// Helper method to get the default instance of a server
    pub fn get_default_instance(&self, server_name: &str) -> Result<(String, &UpstreamConnection)> {
        let instances = self.connections.get(server_name).context(format!(
            "Server '{}' not found in connection pool",
            server_name
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
            "Server '{}' not found in connection pool",
            server_name
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

    /// Trigger connection to all servers in the pool without waiting for completion
    pub async fn trigger_connect_all(&mut self) {
        // Get all server names
        let server_names: Vec<String> = self.connections.keys().cloned().collect();

        // Trigger connection for each server
        for name in server_names {
            if let Err(e) = self.trigger_connect_default(&name).await {
                tracing::warn!("Failed to trigger connection to server '{}': {}", name, e);
            }
        }
    }

    /// Trigger a connection to the default instance of a specific server without waiting for completion
    pub async fn trigger_connect_default(&mut self, server_name: &str) -> Result<()> {
        // Get the instance ID
        let instance_id = {
            let (id, _) = self.get_default_instance(server_name)?;
            id
        };

        // Trigger connection for this instance
        self.trigger_connect(server_name, &instance_id).await
    }

    /// Trigger a connection to a specific server instance without waiting for completion
    pub async fn trigger_connect(&mut self, server_name: &str, instance_id: &str) -> Result<()> {
        // Check if the instance exists
        let conn = self.get_instance(server_name, instance_id)?;

        // Avoid connecting if already initializing
        if matches!(conn.status, ConnectionStatus::Initializing) {
            return Ok(());
        }

        // Check if the server is shutdown
        if matches!(conn.status, ConnectionStatus::Shutdown) {
            // This is fine, we can connect from shutdown state
        } else if matches!(conn.status, ConnectionStatus::Error(_)) {
            // This is also fine, we can reconnect from error state
        }

        // Update status and increment connection attempts
        {
            let conn = self.get_instance_mut(server_name, instance_id)?;
            conn.update_connecting();
        }

        tracing::info!(
            "Triggering connection to server '{}' instance '{}'...",
            server_name,
            instance_id
        );

        // Get the server type
        let server_type = {
            let server_config = self.config.mcp_servers.get(server_name).unwrap();
            server_config.kind.clone()
        };

        // Connect based on server type
        match server_type.as_str() {
            "stdio" => self.connect_stdio(server_name, instance_id).await?,
            "sse" => self.connect_sse(server_name, instance_id).await?,
            _ => {
                let error_msg = format!("Unsupported server type: {}", server_type);
                let conn = self.get_instance_mut(server_name, instance_id)?;
                conn.update_failed(error_msg.clone());
                return Err(anyhow::anyhow!(error_msg));
            }
        };

        Ok(())
    }

    /// Connect to the default instance of a specific server (blocking version)
    pub async fn connect_default(&mut self, server_name: &str) -> Result<()> {
        // Get the instance ID
        let instance_id = {
            let (id, _) = self.get_default_instance(server_name)?;
            id
        };

        // Connect this instance
        self.connect(server_name, &instance_id).await
    }

    /// Connect to a specific server instance (blocking version)
    pub async fn connect(&mut self, server_name: &str, instance_id: &str) -> Result<()> {
        // Check if the instance exists
        let conn = self.get_instance(server_name, instance_id)?;

        // Avoid connecting if already initializing
        if matches!(conn.status, ConnectionStatus::Initializing) {
            return Ok(());
        }

        // Update status and increment connection attempts
        {
            let conn = self.get_instance_mut(server_name, instance_id)?;
            conn.update_connecting();
        }

        tracing::info!(
            "Connecting to server '{}' instance '{}'...",
            server_name,
            instance_id
        );

        // Get the server type
        let server_type = {
            let server_config = self.config.mcp_servers.get(server_name).unwrap();
            server_config.kind.clone()
        };

        // Connect based on server type
        let result = match server_type.as_str() {
            "stdio" => self.connect_stdio(server_name, instance_id).await,
            "sse" => self.connect_sse(server_name, instance_id).await,
            _ => {
                let error_msg = format!("Unsupported server type: {}", server_type);
                let conn = self.get_instance_mut(server_name, instance_id)?;
                conn.update_failed(error_msg.clone());
                Err(anyhow::anyhow!(error_msg))
            }
        };

        // Handle connection result
        if let Err(e) = &result {
            let conn = self.get_instance_mut(server_name, instance_id)?;
            conn.update_failed(e.to_string());
            tracing::error!(
                "Failed to connect to server '{}' instance '{}': {}",
                server_name,
                instance_id,
                e
            );
        }

        result
    }

    /// Helper function to update connection after successful connection
    fn update_connection(
        &mut self,
        server_name: &str,
        instance_id: &str,
        service: RunningService<RoleClient, ()>,
        tools: Vec<Tool>,
    ) {
        // Update the connection with the service and tools
        let conn = self.get_instance_mut(server_name, instance_id).unwrap();
        conn.update_connected(service, tools);

        tracing::info!(
            "Connected to server '{}' instance '{}', found {} tools",
            server_name,
            instance_id,
            conn.tools.len()
        );
    }

    /// Connect to a stdio server
    async fn connect_stdio(&mut self, server_name: &str, instance_id: &str) -> Result<()> {
        // Get server configuration
        let server_config = self.config.mcp_servers.get(server_name).unwrap();

        // Create a new cancellation token
        let ct = CancellationToken::new();

        // Store the cancellation token
        self.cancellation_tokens
            .entry(server_name.to_string())
            .or_insert_with(HashMap::new)
            .insert(instance_id.to_string(), ct.clone());

        // Connect to the server using the proxy module function with cancellation token
        match super::stdio::connect_stdio_server_with_ct(server_name, server_config, ct).await {
            Ok((service, tools)) => {
                // Update connection
                self.update_connection(server_name, instance_id, service, tools);

                // We'll check the connection status in the health check task

                Ok(())
            }
            Err(e) => {
                // Remove the cancellation token if connection failed
                if let Some(tokens) = self.cancellation_tokens.get_mut(server_name) {
                    tokens.remove(instance_id);
                }
                Err(e)
            }
        }
    }

    /// Connect to an SSE server
    async fn connect_sse(&mut self, server_name: &str, instance_id: &str) -> Result<()> {
        // Get server configuration
        let server_config = self.config.mcp_servers.get(server_name).unwrap();

        // Connect to the server using the proxy module function
        match connect_sse_server(server_name, server_config).await {
            Ok((service, tools)) => {
                // Update connection
                self.update_connection(server_name, instance_id, service, tools);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Disconnect from the default instance of a server
    pub async fn disconnect_default(&mut self, server_name: &str) -> Result<()> {
        // Get the instance ID
        let instance_id = {
            let (id, _) = self.get_default_instance(server_name)?;
            id
        };

        // Disconnect this instance
        self.disconnect(server_name, &instance_id).await
    }

    /// Disconnect from a specific instance of a server
    pub async fn disconnect(&mut self, server_name: &str, instance_id: &str) -> Result<()> {
        // Take the service from the connection
        let service = {
            let conn = self.get_instance_mut(server_name, instance_id)?;
            conn.service.take()
        };

        // If there's an active service, cancel it
        if let Some(service) = service {
            match service.cancel().await {
                Ok(quit_reason) => {
                    tracing::info!(
                        "Service for server '{}' instance '{}' cancelled with reason: {:?}",
                        server_name,
                        instance_id,
                        quit_reason
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Error cancelling service for '{}' instance '{}': {}",
                        server_name,
                        instance_id,
                        e
                    );
                }
            }
        }

        // Cancel the token if it exists
        let token_opt = self
            .cancellation_tokens
            .get_mut(server_name)
            .and_then(|tokens| tokens.remove(instance_id));

        if let Some(token) = token_opt {
            token.cancel();
            tracing::debug!(
                "Cancelled token for server '{}' instance '{}'",
                server_name,
                instance_id
            );
        }

        // Update connection status
        {
            let conn = self.get_instance_mut(server_name, instance_id)?;
            conn.update_disconnected();
        }

        tracing::info!(
            "Disconnected from server '{}' instance '{}'",
            server_name,
            instance_id
        );

        Ok(())
    }

    /// Reconnect to the default instance of a server
    pub async fn reconnect_default(&mut self, server_name: &str) -> Result<()> {
        // Get the instance ID
        let instance_id = {
            let (id, _) = self.get_default_instance(server_name)?;
            id
        };

        // Reconnect this instance
        self.reconnect(server_name, &instance_id).await
    }

    /// Reconnect to a specific instance of a server
    pub async fn reconnect(&mut self, server_name: &str, instance_id: &str) -> Result<()> {
        // First disconnect
        self.disconnect(server_name, instance_id).await?;

        // Get connection for backoff calculation
        let conn = self.get_instance(server_name, instance_id)?;

        // Calculate backoff time using exponential backoff
        let backoff = std::cmp::min(
            30,                                                   // Maximum 30 seconds
            2u64.pow(std::cmp::min(5, conn.connection_attempts)), // Exponential backoff, max 2^5=32 seconds
        );

        tracing::info!(
            "Waiting {}s before reconnecting to '{}' instance '{}'",
            backoff,
            server_name,
            instance_id
        );
        sleep(Duration::from_secs(backoff)).await;

        // Reconnect
        self.trigger_connect(server_name, instance_id).await
    }

    /// Connect to all servers in parallel
    pub async fn connect_all(&mut self) -> Result<()> {
        // First trigger connection for all servers without waiting
        self.trigger_connect_all().await;

        // Return immediately, connections will happen in the background
        Ok(())
    }

    /// Disconnect from all servers
    pub async fn disconnect_all(&mut self) -> Result<()> {
        for server_name in self.connections.keys().cloned().collect::<Vec<_>>() {
            // Get all instances for this server
            if let Some(instances) = self.connections.get(&server_name) {
                for instance_id in instances.keys().cloned().collect::<Vec<_>>() {
                    if let Err(e) = self.disconnect(&server_name, &instance_id).await {
                        tracing::error!(
                            "Failed to disconnect from server '{}' instance '{}': {}",
                            server_name,
                            instance_id,
                            e
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Get status of the default instance of a server
    pub fn get_server_status(&self, server_name: &str) -> Result<String> {
        // Check if the server exists
        if !self.connections.contains_key(server_name) {
            return Err(anyhow::anyhow!(
                "Server '{}' not found in connection pool",
                server_name
            ));
        }

        // Get the default instance
        let (instance_id, conn) = self.get_default_instance(server_name)?;

        Ok(format!(
            "{} (instance {})",
            conn.status_string(),
            instance_id
        ))
    }

    /// Get status of a specific instance of a server
    pub fn get_instance_status(&self, server_name: &str, instance_id: &str) -> Result<String> {
        let conn = self.get_instance(server_name, instance_id)?;

        Ok(conn.status_string())
    }

    /// Get all server instances with cloned connections
    pub fn get_all_server_instances(&self) -> HashMap<String, Vec<(String, UpstreamConnection)>> {
        let mut result = HashMap::new();

        for (server_name, instances) in &self.connections {
            let instance_clones: Vec<(String, UpstreamConnection)> = instances
                .iter()
                .map(|(id, conn)| (id.clone(), conn.clone()))
                .collect();

            result.insert(server_name.clone(), instance_clones);
        }

        result
    }

    /// Get server type
    pub fn get_server_type(&self, server_name: &str) -> Option<String> {
        self.config
            .mcp_servers
            .get(server_name)
            .map(|cfg| cfg.kind.clone())
    }

    /// Perform an operation on a specific instance
    pub async fn perform_instance_operation(
        &mut self,
        server_name: &str,
        instance_id: &str,
        operation: &str,
    ) -> Result<()> {
        // Get the instance
        let conn = self.get_instance_mut(server_name, instance_id)?;

        // Check if the operation is allowed
        if !conn.can_perform_operation(operation)
            && !(operation == "force_disconnect" && conn.status.can_force_disconnect())
            && !(operation == "reset_reconnect" && conn.status.can_reset_reconnect())
        {
            return Err(anyhow::anyhow!(
                "Operation '{}' is not allowed in the current state: {}",
                operation,
                conn.status
            ));
        }

        // Perform the operation
        match operation {
            "disconnect" => {
                // Normal disconnect
                self.disconnect(server_name, instance_id).await
            }
            "force_disconnect" => {
                // Force disconnect (works in any state except Shutdown)
                if conn.status.can_force_disconnect() {
                    self.disconnect(server_name, instance_id).await
                } else {
                    Err(anyhow::anyhow!(
                        "Cannot force disconnect in the current state: {}",
                        conn.status
                    ))
                }
            }
            "reconnect" => {
                // Normal reconnect
                self.reconnect(server_name, instance_id).await
            }
            "reset_reconnect" => {
                // Reset and reconnect (works in any state)
                // First disconnect if needed
                if !matches!(conn.status, ConnectionStatus::Shutdown) {
                    if let Err(e) = self.disconnect(server_name, instance_id).await {
                        tracing::warn!("Error during reset_reconnect disconnect phase: {}", e);
                    }
                }

                // Reset connection attempts counter
                let conn = self.get_instance_mut(server_name, instance_id)?;
                conn.reset_connection_attempts();

                // Then reconnect
                self.trigger_connect(server_name, instance_id).await
            }
            "cancel" => {
                // Cancel initialization (only works in Initializing state)
                if matches!(conn.status, ConnectionStatus::Initializing) {
                    self.disconnect(server_name, instance_id).await
                } else {
                    Err(anyhow::anyhow!(
                        "Cannot cancel in the current state: {}",
                        conn.status
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unknown operation: {}", operation)),
        }
    }

    /// Wait for a service to exit and handle the exit reason
    pub async fn waiting_for_service_exit(
        connection_pool: Arc<Mutex<Self>>,
        server_name: String,
        instance_id: String,
    ) {
        // Wait for a short delay to allow the service to initialize
        sleep(Duration::from_secs(1)).await;

        // Lock the pool and check if the service is still running
        let mut pool = connection_pool.lock().await;

        // Check if the instance still exists
        if let Ok(conn) = pool.get_instance_mut(&server_name, &instance_id) {
            // Only update if the service is not connected
            if !conn.is_connected() {
                tracing::info!(
                    "Service for server '{}' instance '{}' is not connected",
                    server_name,
                    instance_id
                );

                // Drop the lock before sleeping
                drop(pool);

                // Wait for a short delay
                sleep(Duration::from_secs(5)).await;

                // Try to reconnect
                let mut pool = connection_pool.lock().await;
                if let Err(e) = pool.reconnect(&server_name, &instance_id).await {
                    tracing::error!(
                        "Failed to reconnect to '{}' instance '{}' after check: {}",
                        server_name,
                        instance_id,
                        e
                    );
                }
            }
        }
    }

    /// Start health check task
    pub fn start_health_check(connection_pool: Arc<Mutex<Self>>) {
        tokio::spawn(async move {
            let pool_clone = connection_pool.clone();

            loop {
                // Wait for health check interval (2 minutes)
                sleep(Duration::from_secs(120)).await;

                // Check connection status for all instances
                {
                    let mut pool = pool_clone.lock().await;
                    if let Err(e) = pool.check_connection_status().await {
                        tracing::error!("Error checking connection status: {}", e);
                    }
                }

                // Check all connections for periodic reconnects
                let mut reconnects = Vec::new();
                {
                    let pool_guard = pool_clone.lock().await;
                    for (server_name, instances) in &pool_guard.connections {
                        for (instance_id, conn) in instances {
                            // Update last health check time
                            let now = std::time::Instant::now();

                            // Only monitor instances that should be monitored
                            if !matches!(
                                conn.status,
                                ConnectionStatus::Ready | ConnectionStatus::Error(_)
                            ) {
                                continue;
                            }

                            match &conn.status {
                                ConnectionStatus::Ready => {
                                    // Check if the service is still alive
                                    if let Some(_service) = &conn.service {
                                        // Periodic reconnect to ensure health
                                        if now > conn.last_connected
                                            && now.duration_since(conn.last_connected)
                                                > Duration::from_secs(3600)
                                        // Every 60 minutes
                                        {
                                            tracing::info!(
                                                "Health check: Periodic reconnect for '{}' instance '{}'",
                                                server_name,
                                                instance_id
                                            );
                                            reconnects
                                                .push((server_name.clone(), instance_id.clone()));
                                        }
                                    } else {
                                        // If service is None but status is Ready, something is wrong
                                        tracing::warn!("Health check: Server '{}' instance '{}' has Ready status but no service, will reconnect", server_name, instance_id);
                                        reconnects.push((server_name.clone(), instance_id.clone()));
                                    }
                                }
                                ConnectionStatus::Error(_) => {
                                    // Reconnect error instances after a delay
                                    if now > conn.last_connected
                                        && now.duration_since(conn.last_connected)
                                            > Duration::from_secs(60)
                                    {
                                        reconnects.push((server_name.clone(), instance_id.clone()));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Reconnect instances that need it
                for (server_name, instance_id) in reconnects {
                    tracing::info!(
                        "Health check: Attempting to reconnect to '{}' instance '{}'",
                        server_name,
                        instance_id
                    );
                    let mut pool_guard = pool_clone.lock().await;
                    if let Err(e) = pool_guard.reconnect(&server_name, &instance_id).await {
                        tracing::warn!(
                            "Health check: Failed to reconnect to '{}' instance '{}': {}",
                            server_name,
                            instance_id,
                            e
                        );
                    }
                }
            }
        });
    }

    /// Check connection status for all instances
    pub async fn check_connection_status(&mut self) -> Result<()> {
        // Get all instances that need checking
        let instances_to_check = {
            let mut result = Vec::new();

            for (server_name, instances) in &self.connections {
                for (instance_id, conn) in instances {
                    // Check both Ready and Error states
                    if (matches!(conn.status, ConnectionStatus::Ready) && conn.service.is_some())
                        || matches!(conn.status, ConnectionStatus::Error(_))
                    {
                        result.push((server_name.clone(), instance_id.clone()));
                    }
                }
            }

            result
        };

        // Check each instance
        for (server_name, instance_id) in instances_to_check {
            // Get the connection
            let conn = match self.get_instance(&server_name, &instance_id) {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            match &conn.status {
                ConnectionStatus::Ready => {
                    // Check if the service is still connected
                    if !conn.is_connected() {
                        tracing::warn!(
                            "Connection check: Service for '{}' instance '{}' is not connected",
                            server_name,
                            instance_id
                        );

                        // Try to reconnect
                        if let Err(e) = self.reconnect(&server_name, &instance_id).await {
                            tracing::error!(
                                "Connection check: Failed to reconnect to '{}' instance '{}': {}",
                                server_name,
                                instance_id,
                                e
                            );
                        }
                    }
                }
                ConnectionStatus::Error(error_details) => {
                    // Check if we should retry based on error type and failure count
                    let should_retry = match error_details.error_type {
                        super::types::ErrorType::Temporary => {
                            // Use exponential backoff for temporary errors
                            let backoff_seconds = std::cmp::min(
                                300,                                                     // Maximum 5 minutes
                                2u64.pow(std::cmp::min(8, error_details.failure_count)), // Exponential backoff, max 2^8=256 seconds
                            );

                            // Calculate time since last failure
                            let now = chrono::Utc::now().timestamp() as u64;
                            let seconds_since_last_failure =
                                now.saturating_sub(error_details.last_failure_time);

                            // Only retry if enough time has passed based on backoff
                            if seconds_since_last_failure >= backoff_seconds {
                                tracing::info!(
                                    "Connection check: Retrying temporary error for '{}' instance '{}' after {}s (failure count: {})",
                                    server_name, instance_id, seconds_since_last_failure, error_details.failure_count
                                );
                                true
                            } else {
                                tracing::debug!(
                                    "Connection check: Waiting {}s before retrying '{}' instance '{}' (failure count: {})",
                                    backoff_seconds - seconds_since_last_failure,
                                    server_name, instance_id, error_details.failure_count
                                );
                                false
                            }
                        }
                        super::types::ErrorType::Permanent => {
                            // Don't retry permanent errors
                            false
                        }
                        super::types::ErrorType::Unknown => {
                            // For unknown errors, retry with a fixed backoff
                            let backoff_seconds = 60; // 1 minute

                            // Calculate time since last failure
                            let now = chrono::Utc::now().timestamp() as u64;
                            let seconds_since_last_failure =
                                now.saturating_sub(error_details.last_failure_time);

                            // Only retry if enough time has passed
                            seconds_since_last_failure >= backoff_seconds
                        }
                    };

                    // If we should retry, attempt to reconnect
                    if should_retry {
                        // Check if we've exceeded the maximum retry count for temporary errors
                        let max_retries_exceeded =
                            matches!(error_details.error_type, super::types::ErrorType::Temporary)
                                && error_details.failure_count > 10;

                        if max_retries_exceeded {
                            // Store the failure count for later use
                            let failure_count = error_details.failure_count;

                            // We need to break out of the match and for loop to avoid borrowing issues
                            // No need to explicitly drop a reference

                            // Convert to permanent error after too many retries
                            {
                                let conn = self.get_instance_mut(&server_name, &instance_id)?;
                                conn.update_permanent_error(format!(
                                    "Too many failed reconnection attempts ({}). Manual intervention required.",
                                    failure_count
                                ));
                            }

                            tracing::error!(
                                "Connection check: Too many failed reconnection attempts for '{}' instance '{}' ({}). Marking as permanent error.",
                                server_name, instance_id, failure_count
                            );
                            continue;
                        }

                        // Try to reconnect
                        if let Err(e) = self.reconnect(&server_name, &instance_id).await {
                            tracing::error!(
                                "Connection check: Failed to reconnect to '{}' instance '{}': {}",
                                server_name,
                                instance_id,
                                e
                            );
                        }
                    }
                }
                _ => {
                    // Other states don't need checking
                }
            }
        }

        Ok(())
    }

    /// Calculate a hash value representing the current state of the connection pool
    /// This can be used to detect changes in the connection pool
    pub fn calculate_connection_state_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash the number of servers
        self.connections.len().hash(&mut hasher);

        // For each server, hash its name and the state of each instance
        for (server_name, instances) in &self.connections {
            server_name.hash(&mut hasher);
            instances.len().hash(&mut hasher);

            for (instance_id, conn) in instances {
                instance_id.hash(&mut hasher);

                // Hash the connection status
                let status_str = format!("{:?}", conn.status);
                status_str.hash(&mut hasher);

                // Hash the number of tools
                conn.tools.len().hash(&mut hasher);

                // Hash the tool names
                for tool in &conn.tools {
                    tool.name.to_string().hash(&mut hasher);
                }
            }
        }

        hasher.finish()
    }
}
