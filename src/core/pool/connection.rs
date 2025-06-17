//! Pool connection management functionality
//!
//! Provides connection lifecycle management for UpstreamConnectionPool including:
//! - connection establishment and teardown
//! - reconnection logic with exponential backoff
//! - parallel connection capabilities
//! - service lifecycle management

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use rmcp::{RoleClient, model::Tool, service::RunningService};
use tracing;

use super::UpstreamConnectionPool;
use crate::{
    common::server::TransportType,
    core::{
        foundation::types::{
            ConnectionOperation, // action to perform on the connection
            ConnectionStatus,    // status of the connection
        },
        transport::{
            connect_http_server,                     // connect to an HTTP server
            connect_sse_server,                      // connect to an SSE server
            connect_stdio_server_with_ct_and_db, // connect to a stdio server with a cancellation token and a database
            connect_stdio_server_with_runtime_cache, // connect to a stdio server with a runtime cache
        },
    },
};

impl UpstreamConnectionPool {
    /// Reconnect to a specific instance of a server (non-blocking)
    ///
    /// This method schedules a reconnection without blocking the connection pool.
    /// The actual reconnection happens asynchronously after the backoff period.
    pub async fn reconnect(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        // First disconnect
        self.disconnect(server_name, instance_id).await?;

        // Get connection for backoff calculation
        let conn = self.get_instance(server_name, instance_id)?;

        // Calculate backoff time using exponential backoff with longer delays for better fault isolation
        // MAX 300 seconds (5 minutes), exponential up to 2^8=256 seconds
        let backoff = std::cmp::min(300, 2u64.pow(std::cmp::min(8, conn.connection_attempts)));

        tracing::info!(
            "Scheduling reconnection to '{}' instance '{}' in {}s (non-blocking)",
            server_name,
            instance_id,
            backoff
        );

        // Schedule asynchronous reconnection without blocking the connection pool
        self.schedule_async_reconnect(server_name, instance_id, backoff)
            .await
    }

    /// Schedule an asynchronous reconnection without blocking the connection pool
    async fn schedule_async_reconnect(
        &mut self,
        server_name: &str,
        instance_id: &str,
        backoff_seconds: u64,
    ) -> Result<()> {
        // Clone necessary data for the async task
        let server_name = server_name.to_string();
        let instance_id = instance_id.to_string();

        // Create a weak reference to avoid circular dependencies
        let pool_weak = Arc::downgrade(&Arc::new(tokio::sync::Mutex::new(self.clone())));

        // Spawn async reconnection task
        tokio::spawn(async move {
            // Wait for backoff period without blocking anything
            tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;

            tracing::info!(
                "Executing scheduled reconnection to '{}' instance '{}' after {}s delay",
                server_name,
                instance_id,
                backoff_seconds
            );

            // Try to upgrade the weak reference
            if let Some(pool_arc) = pool_weak.upgrade() {
                // Attempt reconnection with minimal lock time
                let result = {
                    let mut pool = pool_arc.lock().await;
                    pool.trigger_connect(&server_name, &instance_id).await
                };

                match result {
                    Ok(()) => {
                        tracing::info!(
                            "Scheduled reconnection to '{}' instance '{}' completed successfully",
                            server_name,
                            instance_id
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "Scheduled reconnection to '{}' instance '{}' failed: {}",
                            server_name,
                            instance_id,
                            e
                        );
                    }
                }
            } else {
                tracing::debug!(
                    "Connection pool no longer available for scheduled reconnection to '{}' instance '{}'",
                    server_name,
                    instance_id
                );
            }
        });

        Ok(())
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

    /// Trigger connection to a specific instance of a server
    pub async fn trigger_connect(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        self.connect_core(server_name, instance_id, false).await
    }

    /// Connect to a specific instance of a server and wait for result
    pub async fn connect(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        self.connect_core(server_name, instance_id, true).await
    }

    /// Core connection logic
    async fn connect_core(
        &mut self,
        server_name: &str,
        instance_id: &str,
        _wait_for_result: bool,
    ) -> Result<()> {
        // Get server configuration (clone to avoid borrowing issues)
        let server_config = self
            .config
            .mcp_servers
            .get(server_name)
            .ok_or_else(|| anyhow::anyhow!("Server '{}' not found in configuration", server_name))?
            .clone();

        // Update connection status to initializing
        {
            let conn = self.get_instance_mut(server_name, instance_id)?;
            conn.update_initializing();
        }

        // Connect based on server type
        let result = match server_config.kind.as_str() {
            "stdio" => self.connect_stdio(server_name, instance_id).await,
            "sse" => self.connect_sse(server_name, instance_id).await,
            "http" => self.connect_http(server_name, instance_id).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported server type: {}",
                server_config.kind
            )),
        };

        // Handle connection result
        match result {
            Ok(()) => {
                tracing::info!(
                    "Successfully initiated connection to '{}' instance '{}'",
                    server_name,
                    instance_id
                );
                Ok(())
            }
            Err(e) => {
                // Update connection with progressive failure escalation
                let conn = self.get_instance_mut(server_name, instance_id)?;
                conn.update_error_with_escalation(format!("Connection failed: {}", e));

                tracing::error!(
                    "Failed to connect to '{}' instance '{}': {} (progressive escalation applied)",
                    server_name,
                    instance_id,
                    e
                );
                Err(e)
            }
        }
    }

    /// Connect to stdio server
    async fn connect_stdio(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        let server_config = self.config.mcp_servers.get(server_name).unwrap();

        // Create cancellation token for this connection
        let ct = tokio_util::sync::CancellationToken::new();

        // Store the cancellation token
        self.cancellation_tokens
            .entry(server_name.to_string())
            .or_default()
            .insert(instance_id.to_string(), ct.clone());

        // Get database pool if available
        let database_pool = self.database.as_ref().map(|db| &db.pool);

        // Use the core transport module
        let (service, tools, capabilities, process_id) =
            if let Some(runtime_cache) = &self.runtime_cache {
                connect_stdio_server_with_runtime_cache(
                    server_name,
                    server_config,
                    ct,
                    database_pool,
                    runtime_cache,
                )
                .await?
            } else {
                connect_stdio_server_with_ct_and_db(server_name, server_config, ct, database_pool)
                    .await?
            };

        // Update connection with service
        self.update_connection(server_name, instance_id, service, tools, capabilities);

        // Update process ID if available
        if let Some(pid) = process_id {
            if let Ok(conn) = self.get_instance_mut(server_name, instance_id) {
                conn.process_id = Some(pid);
            }
        }

        Ok(())
    }

    /// Connect to SSE server
    async fn connect_sse(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        let server_config = self.config.mcp_servers.get(server_name).unwrap();

        // Use the core transport module
        let (service, tools, capabilities) = connect_sse_server(server_name, server_config).await?;

        // Update connection with service
        self.update_connection(server_name, instance_id, service, tools, capabilities);

        Ok(())
    }

    /// Connect to HTTP server
    async fn connect_http(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        let server_config = self.config.mcp_servers.get(server_name).unwrap();

        // Use the core transport module - default to StreamableHttp transport type
        let (service, tools, capabilities) =
            connect_http_server(server_name, server_config, TransportType::StreamableHttp).await?;

        // Update connection with service
        self.update_connection(server_name, instance_id, service, tools, capabilities);

        Ok(())
    }

    /// Update connection with service and metadata
    pub fn update_connection(
        &mut self,
        server_name: &str,
        instance_id: &str,
        service: RunningService<RoleClient, ()>,
        tools: Vec<Tool>,
        capabilities: Option<rmcp::model::ServerCapabilities>,
    ) {
        if let Ok(conn) = self.get_instance_mut(server_name, instance_id) {
            // Check if server supports resources and prompts
            let supports_resources = capabilities
                .as_ref()
                .and_then(|caps| caps.resources.as_ref())
                .is_some();
            let supports_prompts = capabilities
                .as_ref()
                .and_then(|caps| caps.prompts.as_ref())
                .is_some();

            // Clone service for database sync operations
            let service_for_sync = service.peer().clone();

            conn.service = Some(service);
            conn.tools = tools.clone();
            conn.capabilities = capabilities;
            conn.update_ready();

            tracing::info!(
                "Updated connection for '{}' instance '{}' with service and {} tools",
                server_name,
                instance_id,
                conn.tools.len()
            );

            // Sync to database if database reference is available
            if let Some(db) = &self.database {
                let db_clone = db.clone();
                let tools_clone = tools.clone();
                let server_name_clone = server_name.to_string();
                let instance_id_clone = instance_id.to_string();
                let service_for_sync = service_for_sync.clone();

                // Single task to handle all database sync operations concurrently
                tokio::spawn(async move {
                    // Build list of sync operations to execute
                    let mut sync_futures = Vec::new();

                    // Always sync tools
                    {
                        let db = db_clone.clone();
                        let server_name = server_name_clone.clone();
                        let tools = tools_clone.clone();
                        sync_futures.push(Box::pin(async move {
                            UpstreamConnectionPool::sync_tools_to_database(
                                &db,
                                &server_name,
                                &tools,
                            )
                            .await
                            .map_err(|e| ("tools", e))
                        })
                            as std::pin::Pin<
                                Box<
                                    dyn std::future::Future<
                                            Output = Result<(), (&str, anyhow::Error)>,
                                        > + Send,
                                >,
                            >);
                    }

                    // Conditionally sync resources if server supports them
                    if supports_resources {
                        let db = db_clone.clone();
                        let server_name = server_name_clone.clone();
                        let instance_id = instance_id_clone.clone();
                        let service = service_for_sync.clone();
                        sync_futures.push(Box::pin(async move {
                            UpstreamConnectionPool::sync_resources_to_database_with_service(
                                &db,
                                &server_name,
                                &instance_id,
                                &service,
                            )
                            .await
                            .map_err(|e| ("resources", e))
                        })
                            as std::pin::Pin<
                                Box<
                                    dyn std::future::Future<
                                            Output = Result<(), (&str, anyhow::Error)>,
                                        > + Send,
                                >,
                            >);
                    }

                    // Conditionally sync prompts if server supports them
                    if supports_prompts {
                        let db = db_clone.clone();
                        let server_name = server_name_clone.clone();
                        let instance_id = instance_id_clone.clone();
                        let service = service_for_sync.clone();
                        sync_futures.push(Box::pin(async move {
                            UpstreamConnectionPool::sync_prompts_to_database_with_service(
                                &db,
                                &server_name,
                                &instance_id,
                                &service,
                            )
                            .await
                            .map_err(|e| ("prompts", e))
                        })
                            as std::pin::Pin<
                                Box<
                                    dyn std::future::Future<
                                            Output = Result<(), (&str, anyhow::Error)>,
                                        > + Send,
                                >,
                            >);
                    }

                    // Execute all sync operations concurrently
                    let results = futures::future::join_all(sync_futures).await;

                    // Process results and log any errors
                    let mut success_count = 0;
                    let mut error_count = 0;

                    for result in results {
                        match result {
                            Ok(()) => success_count += 1,
                            Err((operation, error)) => {
                                error_count += 1;
                                tracing::error!(
                                    "Failed to sync {} to database for server '{}': {}",
                                    operation,
                                    server_name_clone,
                                    error
                                );
                            }
                        }
                    }

                    if error_count == 0 {
                        tracing::debug!(
                            "Successfully completed {} database sync operations for server '{}'",
                            success_count,
                            server_name_clone
                        );
                    } else {
                        tracing::warn!(
                            "Database sync completed for server '{}': {} successful, {} failed",
                            server_name_clone,
                            success_count,
                            error_count
                        );
                    }
                });
            }
        }
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
            ConnectionOperation::Connect => self.trigger_connect(server_name, instance_id).await,
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
            ConnectionOperation::Recover => {
                // Recover operation should be handled at the API level, not here
                // This is a manual operation that requires direct connection manipulation
                Err(anyhow::anyhow!(
                    "Recover operation should be handled directly via manual_re_enable method"
                ))
            }
        }
    }

    /// Get the default instance ID for a server
    pub fn get_default_instance_id(
        &self,
        server_name: &str,
    ) -> Result<String> {
        let (instance_id, _) = self.get_default_instance(server_name)?;
        Ok(instance_id)
    }

    /// Log a connection event
    pub fn log_connection_event(
        &self,
        level: tracing::Level,
        server_name: &str,
        instance_id: &str,
        message: &str,
    ) {
        match level {
            tracing::Level::ERROR => {
                tracing::error!("[{}:{}] {}", server_name, instance_id, message)
            }
            tracing::Level::WARN => tracing::warn!("[{}:{}] {}", server_name, instance_id, message),
            tracing::Level::INFO => tracing::info!("[{}:{}] {}", server_name, instance_id, message),
            tracing::Level::DEBUG => {
                tracing::debug!("[{}:{}] {}", server_name, instance_id, message)
            }
            tracing::Level::TRACE => {
                tracing::trace!("[{}:{}] {}", server_name, instance_id, message)
            }
        }
    }
}
