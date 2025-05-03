// Connection management functionality for UpstreamConnectionPool

use anyhow::Result;
use rmcp::{model::Tool, service::RunningService, RoleClient};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing;

use crate::core::connect_sse_server;
use crate::core::stdio::connect_stdio_server_with_ct;
use crate::core::types::ConnectionStatus;

use super::UpstreamConnectionPool;

impl UpstreamConnectionPool {
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
        match connect_stdio_server_with_ct(server_name, server_config, ct).await {
            Ok((service, tools, pid)) => {
                // Update connection
                self.update_connection(server_name, instance_id, service, tools);

                // If we have a process ID, update resource monitoring
                if let Some(pid) = pid {
                    // Store the process ID for resource monitoring
                    if let Some(_process_monitor) = &self.process_monitor {
                        tracing::info!(
                            "Monitoring process resources for '{}' instance '{}' (PID: {})",
                            server_name,
                            instance_id,
                            pid
                        );

                        // Update the connection with process ID
                        if let Ok(conn) = self.get_instance_mut(server_name, instance_id) {
                            conn.process_id = Some(pid);
                        }
                    }
                }

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
}
