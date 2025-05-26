// Connection management functionality for UpstreamConnectionPool

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use rmcp::{RoleClient, model::Tool, service::RunningService};
use tokio::{sync::Mutex, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing;

use super::UpstreamConnectionPool;
use crate::{
    common::types::{ConnectionOperation, ServerType},
    core::{
        connect_http_server, connect_sse_server, transport::TransportType, types::ConnectionStatus,
    },
};

impl UpstreamConnectionPool {
    /// Helper function to log connection-related events
    fn log_connection_event(
        &self,
        level: tracing::Level,
        server_name: &str,
        instance_id: &str,
        message: &str,
    ) {
        match level {
            tracing::Level::INFO => tracing::info!(
                "{} for server '{}' instance '{}'",
                message,
                server_name,
                instance_id
            ),
            tracing::Level::ERROR => tracing::error!(
                "{} for server '{}' instance '{}'",
                message,
                server_name,
                instance_id
            ),
            tracing::Level::WARN => tracing::warn!(
                "{} for server '{}' instance '{}'",
                message,
                server_name,
                instance_id
            ),
            tracing::Level::DEBUG => tracing::debug!(
                "{} for server '{}' instance '{}'",
                message,
                server_name,
                instance_id
            ),
            tracing::Level::TRACE => tracing::trace!(
                "{} for server '{}' instance '{}'",
                message,
                server_name,
                instance_id
            ),
        }
    }

    /// Helper function to get the default instance ID
    fn get_default_instance_id(
        &self,
        server_name: &str,
    ) -> Result<String> {
        let (id, _) = self.get_default_instance(server_name)?;
        Ok(id)
    }

    /// Trigger connection to all servers in the pool without waiting for completion
    pub async fn trigger_connect_all(&mut self) {
        // Get all server names
        let server_names: Vec<String> = self.connections.keys().cloned().collect();

        // Trigger connection for each server
        for name in server_names {
            if let Ok(instance_id) = self.get_default_instance_id(&name) {
                if let Err(e) = self.trigger_connect(&name, &instance_id).await {
                    tracing::warn!("Failed to trigger connection to server '{}': {}", name, e);
                }
            }
        }
    }

    /// Core connection function that handles both trigger and wait modes
    async fn connect_core(
        &mut self,
        server_name: &str,
        instance_id: &str,
        wait_for_result: bool,
    ) -> Result<()> {
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

        // Log appropriate message based on mode
        if wait_for_result {
            self.log_connection_event(
                tracing::Level::INFO,
                server_name,
                instance_id,
                "Connecting to",
            );
        } else {
            self.log_connection_event(
                tracing::Level::INFO,
                server_name,
                instance_id,
                "Triggering connection to",
            );
        }

        // Get the server type
        // Note: Global availability check has been moved to merge_servers function in src/core/suit/merge.rs
        // This ensures servers are filtered out before connection attempts are made
        let server_type = {
            let server_config = match self.config.mcp_servers.get(server_name) {
                Some(config) => config,
                None => {
                    let error_msg = format!("Server configuration for '{server_name}' not found");
                    let conn = self.get_instance_mut(server_name, instance_id)?;
                    conn.update_failed(error_msg.clone());
                    return Err(anyhow::anyhow!(error_msg));
                }
            };

            server_config.kind
        };

        // Connect based on server type using enum matching
        let result = match server_type {
            ServerType::Stdio => self.connect_stdio(server_name, instance_id).await,
            ServerType::Sse => self.connect_http(server_name, instance_id).await,
            ServerType::StreamableHttp => self.connect_http(server_name, instance_id).await,
        };

        // Handle result based on mode
        if wait_for_result {
            // In wait mode, handle errors and return the result
            if let Err(e) = &result {
                let conn = self.get_instance_mut(server_name, instance_id)?;
                conn.update_failed(e.to_string());
                self.log_connection_event(
                    tracing::Level::ERROR,
                    server_name,
                    instance_id,
                    &format!("Failed to connect: {e}"),
                );
            }
            result
        } else {
            // In trigger mode, just return Ok unless there was an error
            match &result {
                Err(e) => Err(anyhow::anyhow!("{}", e)),
                Ok(_) => Ok(()),
            }
        }
    }

    /// Trigger a connection to a specific server instance without waiting for completion
    pub async fn trigger_connect(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        self.connect_core(server_name, instance_id, false).await
    }

    /// Connect to a specific server instance (blocking version)
    pub async fn connect(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        self.connect_core(server_name, instance_id, true).await
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
        conn.update_connected(service, tools.clone());

        tracing::info!(
            "Connected to server '{}' instance '{}', found {} tools",
            server_name,
            instance_id,
            conn.tools.len()
        );

        // Sync tools to database if database reference is available
        if let Some(db) = &self.database {
            // Spawn a task to sync tools to database to avoid blocking the connection process
            let db_clone = db.clone();
            let server_name_clone = server_name.to_string();
            let tools_clone = tools.clone();

            tokio::spawn(async move {
                if let Err(e) =
                    Self::sync_tools_to_database(&db_clone, &server_name_clone, &tools_clone).await
                {
                    tracing::error!("Failed to sync tools to database: {}", e);
                }
            });
        }
    }

    /// Sync tools to database
    ///
    /// This function syncs tools from a server to the database.
    /// It adds tools to all config suits that have the server enabled.
    async fn sync_tools_to_database(
        db: &Arc<crate::conf::database::Database>,
        server_name: &str,
        tools: &[Tool],
    ) -> anyhow::Result<()> {
        use anyhow::Context;

        tracing::info!(
            "Syncing {} tools from server '{}' to database",
            tools.len(),
            server_name
        );

        // Get the server ID
        let server = crate::conf::server::get_server(&db.pool, server_name)
            .await
            .context(format!("Failed to get server '{server_name}'"))?;

        if let Some(server) = server {
            if let Some(server_id) = &server.id {
                // Get all config suits that have this server enabled
                let all_suits = crate::conf::suit::get_all_config_suits(&db.pool)
                    .await
                    .context("Failed to get all config suits")?;

                let mut suits_with_server = Vec::new();

                for suit in all_suits {
                    if let Some(suit_id) = &suit.id {
                        // Get all servers in this suit
                        let suit_servers =
                            crate::conf::suit::get_config_suit_servers(&db.pool, suit_id)
                                .await
                                .context(format!("Failed to get servers for suit '{suit_id}'"))?;

                        // Check if this server is in the suit
                        for suit_server in suit_servers {
                            if suit_server.server_id == *server_id {
                                suits_with_server.push(suit.clone());
                                break;
                            }
                        }
                    }
                }

                tracing::info!(
                    "Found {} config suits with server '{}' enabled",
                    suits_with_server.len(),
                    server_name
                );

                // For each suit, add all tools
                for suit in suits_with_server {
                    if let Some(suit_id) = &suit.id {
                        // Get existing tools in this suit for this server
                        let existing_tools =
                            crate::conf::suit::get_config_suit_tools(&db.pool, suit_id)
                                .await
                                .context(format!("Failed to get tools for suit '{suit_id}'"))?;

                        let existing_tool_names: std::collections::HashSet<String> = existing_tools
                            .iter()
                            .filter(|t| t.server_id == *server_id)
                            .map(|t| t.tool_name.clone())
                            .collect();

                        // Add new tools to the suit
                        for tool in tools {
                            let tool_name = tool.name.to_string();

                            // Skip if tool already exists in this suit
                            if existing_tool_names.contains(&tool_name) {
                                continue;
                            }

                            // Add the tool to the suit (enabled by default)
                            match crate::conf::suit::add_tool_to_config_suit(
                                &db.pool, suit_id, server_id, &tool_name, true,
                            )
                            .await
                            {
                                Ok(_) => {
                                    tracing::debug!(
                                        "Added tool '{}' from server '{}' to suit '{}'",
                                        tool_name,
                                        server_name,
                                        suit.name
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to add tool '{}' from server '{}' to suit '{}': {}",
                                        tool_name,
                                        server_name,
                                        suit.name,
                                        e
                                    );
                                }
                            }
                        }

                        tracing::info!(
                            "Synced tools from server '{}' to suit '{}'",
                            server_name,
                            suit.name
                        );
                    }
                }

                tracing::info!(
                    "Successfully synced {} tools from server '{}' to database",
                    tools.len(),
                    server_name
                );

                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Server '{}' not found", server_name))
    }

    /// Connect to a stdio server
    async fn connect_stdio(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        // Get server configuration
        let server_config = match self.config.mcp_servers.get(server_name) {
            Some(config) => config,
            None => {
                let error_msg = format!("Server configuration for '{server_name}' not found");
                let conn = self.get_instance_mut(server_name, instance_id)?;
                conn.update_failed(error_msg.clone());
                return Err(anyhow::anyhow!(error_msg));
            }
        };

        // Create a new cancellation token
        let ct = CancellationToken::new();

        // Store the cancellation token
        self.cancellation_tokens
            .entry(server_name.to_string())
            .or_default()
            .insert(instance_id.to_string(), ct.clone());

        // Connect to the server using the proxy module function with cancellation token
        let database_pool = self.database.as_ref().map(|db| &db.pool);

        // Use runtime cache if available, otherwise fallback to old method
        tracing::debug!("Runtime cache available: {}", self.runtime_cache.is_some());
        let connect_result = if let Some(runtime_cache) = &self.runtime_cache {
            tracing::debug!(
                "Using runtime cache for stdio connection to '{}'",
                server_name
            );
            crate::core::stdio::connect_stdio_server_with_runtime_cache(
                server_name,
                server_config,
                ct,
                database_pool,
                runtime_cache,
            )
            .await
        } else {
            tracing::debug!(
                "Runtime cache not available, using fallback method for '{}'",
                server_name
            );
            crate::core::stdio::connect_stdio_server_with_ct_and_db(
                server_name,
                server_config,
                ct,
                database_pool,
            )
            .await
        };

        match connect_result {
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

    /// Connect to an HTTP-based server (SSE or Streamable HTTP)
    async fn connect_http(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        // Get server configuration
        let server_config = match self.config.mcp_servers.get(server_name) {
            Some(config) => config,
            None => {
                let error_msg = format!("Server configuration for '{server_name}' not found");
                let conn = self.get_instance_mut(server_name, instance_id)?;
                conn.update_failed(error_msg.clone());
                return Err(anyhow::anyhow!(error_msg));
            }
        };

        // Get transport type (core::TransportType)
        let transport_type = server_config.get_transport_type();

        // Check if we need to wait for the server ready event
        let server_type = server_config.kind;
        if crate::core::events::needs_transport_ready_wait(server_type, transport_type) {
            // Wait for the server transport layer to be ready, max 1000ms
            // Timeout is not an error, it just means the service might not have started yet
            if let Err(e) =
                crate::core::events::wait_for_transport_ready(transport_type, 1000).await
            {
                tracing::warn!(
                    "Waiting for {:?} transport layer to be ready failed: {}，continue to connect",
                    transport_type,
                    e
                );
            }
        }

        // Choose the appropriate connection function based on transport type
        let connect_result = if transport_type == TransportType::Sse {
            // For backward compatibility, use the old SSE function
            connect_sse_server(server_name, server_config).await
        } else {
            // Use the new function for Streamable HTTP
            connect_http_server(server_name, server_config, transport_type).await
        };

        // Handle the connection result
        match connect_result {
            Ok((service, tools)) => {
                // Update connection
                self.update_connection(server_name, instance_id, service, tools);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Disconnect from a specific instance of a server
    pub async fn disconnect(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
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

    /// Reconnect to a specific instance of a server
    pub async fn reconnect(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        // First disconnect
        self.disconnect(server_name, instance_id).await?;

        // Get connection for backoff calculation
        let conn = self.get_instance(server_name, instance_id)?;

        // Calculate backoff time using exponential backoff, MAX 30 seconds, MIN 2^5=32 seconds
        let backoff = std::cmp::min(30, 2u64.pow(std::cmp::min(5, conn.connection_attempts)));

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
        // Parse the operation string into enum
        let operation_type = operation
            .parse::<ConnectionOperation>()
            .map_err(|_| anyhow::anyhow!("Invalid operation: {}", operation))?;

        self.perform_instance_operation_typed(server_name, instance_id, operation_type)
            .await
    }

    /// Perform a typed operation on a specific instance (internal method)
    async fn perform_instance_operation_typed(
        &mut self,
        server_name: &str,
        instance_id: &str,
        operation: ConnectionOperation,
    ) -> Result<()> {
        // Get the instance
        let conn = self.get_instance_mut(server_name, instance_id)?;

        // Check if the operation is allowed using the new type-safe API
        let is_allowed = conn.status.can_perform_operation(operation);

        if !is_allowed {
            return Err(anyhow::anyhow!(
                "Operation '{}' is not allowed in the current state: {}",
                operation,
                conn.status
            ));
        }

        // Perform the operation using enum matching
        match operation {
            ConnectionOperation::Disconnect => self.disconnect(server_name, instance_id).await,
            ConnectionOperation::ForceDisconnect => self.disconnect(server_name, instance_id).await,
            ConnectionOperation::Reconnect => self.reconnect(server_name, instance_id).await,
            ConnectionOperation::Cancel => self.disconnect(server_name, instance_id).await,
            ConnectionOperation::ResetReconnect => {
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
