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
            connect_http_server, // connect to an HTTP server
            connect_sse_server,  // connect to an SSE server
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

        // For now, use immediate reconnect to avoid the Weak reference bug
        // TODO: Implement proper Arc-based async scheduling
        tracing::warn!(
            "DIAGNOSTIC: Using immediate reconnect to bypass Weak reference bug for '{}' instance '{}'",
            server_name,
            instance_id
        );

        // Immediate reconnect instead of async scheduling
        self.trigger_connect(server_name, instance_id).await
    }

    /// Disconnect from a specific instance of a server with improved resource cleanup
    pub async fn disconnect(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        tracing::info!(
            "DIAGNOSTIC: disconnect() called for '{}' instance '{}' - Stack trace requested",
            server_name,
            instance_id
        );

        // Add stack trace logging to identify caller
        let backtrace = std::backtrace::Backtrace::capture();
        tracing::info!("DIAGNOSTIC: Disconnect initiated from: {}", backtrace);

        // Step 1: Cancel the token first to stop new operations
        self.cancel_connection_token(server_name, instance_id);

        // Step 2: Take the service from the connection
        let service = {
            let conn = self.get_instance_mut(server_name, instance_id)?;
            conn.service.take()
        };

        // Step 3: Cancel active service if exists (early return if no service)
        if let Some(service_arc) = service {
            self.cancel_service_with_timeout(server_name, instance_id, service_arc)
                .await;
        }

        // Step 4: Update connection status
        let conn = self.get_instance_mut(server_name, instance_id)?;
        conn.update_disconnected();

        tracing::info!("Disconnected from server '{}' instance '{}'", server_name, instance_id);

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

    // Removed redundant implementations in favor of update_server_status

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
            _ => Err(anyhow::anyhow!("Unsupported server type: {}", server_config.kind)),
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

        // Use the unified transport interface to reduce code duplication
        let (service, tools, capabilities, process_id) = crate::core::transport::unified::connect_server(
            server_name,
            server_config,
            crate::common::server::ServerType::Stdio,
            crate::common::server::TransportType::Stdio,
            Some(ct),
            database_pool,
            self.runtime_cache.as_ref().map(|rc| rc.as_ref()),
        )
        .await?;

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

    /// Cancel connection token for a specific instance
    fn cancel_connection_token(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) {
        let token_opt = self
            .cancellation_tokens
            .get_mut(server_name)
            .and_then(|tokens| tokens.remove(instance_id));

        let Some(token) = token_opt else {
            return; // No token to cancel, early return
        };

        token.cancel();
        tracing::debug!(
            "Cancelled token for server '{}' instance '{}' to stop new operations",
            server_name,
            instance_id
        );
    }

    /// Cancel service with timeout handling
    async fn cancel_service_with_timeout(
        &self,
        server_name: &str,
        instance_id: &str,
        service_arc: Arc<RunningService<RoleClient, ()>>,
    ) {
        let cancel_timeout = Duration::from_secs(5);

        tracing::info!(
            "DIAGNOSTIC: About to cancel service for '{}' instance '{}' with {}s timeout",
            server_name,
            instance_id,
            cancel_timeout.as_secs()
        );

        // Try to extract the service from Arc for cancellation
        let service = match Arc::try_unwrap(service_arc) {
            Ok(service) => service,
            Err(_arc) => {
                tracing::warn!(
                    "DIAGNOSTIC: Cannot cancel service for '{}' instance '{}' - multiple references exist",
                    server_name,
                    instance_id
                );
                return; // Early return if multiple references exist
            }
        };

        // Handle service cancellation with timeout
        match tokio::time::timeout(cancel_timeout, service.cancel()).await {
            Ok(Ok(quit_reason)) => {
                tracing::info!(
                    "DIAGNOSTIC: Service for server '{}' instance '{}' cancelled gracefully with reason: {:?} - Timestamp: {}",
                    server_name,
                    instance_id,
                    quit_reason,
                    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f")
                );
            }
            Ok(Err(e)) => {
                tracing::warn!(
                    "DIAGNOSTIC: Error during graceful cancellation for '{}' instance '{}': {} - Timestamp: {}",
                    server_name,
                    instance_id,
                    e,
                    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f")
                );
            }
            Err(_) => {
                tracing::warn!(
                    "DIAGNOSTIC: Service cancellation timeout for '{}' instance '{}', resources may be leaked - Timestamp: {}",
                    server_name,
                    instance_id,
                    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f")
                );
            }
        }
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
        // Early return if connection cannot be retrieved
        let Ok(conn) = self.get_instance_mut(server_name, instance_id) else {
            tracing::error!(
                "Failed to update connection for '{}' instance '{}' - connection not found",
                server_name,
                instance_id
            );
            return;
        };

        // Check server capabilities
        let supports_resources = capabilities.as_ref().and_then(|caps| caps.resources.as_ref()).is_some();
        let supports_prompts = capabilities.as_ref().and_then(|caps| caps.prompts.as_ref()).is_some();

        // Clone service for database sync operations
        let service_for_sync = service.peer().clone();

        // Update connection properties
        conn.service = Some(Arc::new(service));
        conn.tools = tools.clone();
        conn.capabilities = capabilities;
        conn.update_ready();

        tracing::info!(
            "Updated connection for '{}' instance '{}' with service and {} tools",
            server_name,
            instance_id,
            conn.tools.len()
        );

        // Handle database sync (early return if no database)
        let Some(db) = &self.database else {
            return; // No database available, skip sync operations
        };

        self.spawn_database_sync_task(
            db.clone(),
            server_name.to_string(),
            instance_id.to_string(),
            tools,
            service_for_sync,
            supports_resources,
            supports_prompts,
        );
    }

    /// Spawn database sync operations in background task
    fn spawn_database_sync_task(
        &self,
        db_clone: Arc<crate::config::database::Database>,
        server_name_clone: String,
        instance_id_clone: String,
        tools_clone: Vec<Tool>,
        service_for_sync: rmcp::service::Peer<rmcp::service::RoleClient>,
        supports_resources: bool,
        supports_prompts: bool,
    ) {
        tokio::spawn(async move {
            // Always sync tools
            if let Err(e) =
                UpstreamConnectionPool::sync_tools_to_database(&db_clone, &server_name_clone, &tools_clone).await
            {
                tracing::error!(
                    "Failed to sync tools to database for server '{}': {}",
                    server_name_clone,
                    e
                );
            }

            // Early return if no additional sync needed
            if !supports_resources && !supports_prompts {
                tracing::debug!(
                    "Database sync operations completed for server '{}' (tools only)",
                    server_name_clone
                );
                return;
            }

            // Conditionally sync resources if server supports them
            if supports_resources {
                if let Err(e) = UpstreamConnectionPool::sync_resources_to_database_with_service(
                    &db_clone,
                    &server_name_clone,
                    &instance_id_clone,
                    &service_for_sync,
                )
                .await
                {
                    tracing::error!(
                        "Failed to sync resources to database for server '{}': {}",
                        server_name_clone,
                        e
                    );
                }
            }

            // Conditionally sync prompts if server supports them
            if supports_prompts {
                if let Err(e) = UpstreamConnectionPool::sync_prompts_to_database_with_service(
                    &db_clone,
                    &server_name_clone,
                    &instance_id_clone,
                    &service_for_sync,
                )
                .await
                {
                    tracing::error!(
                        "Failed to sync prompts to database for server '{}': {}",
                        server_name_clone,
                        e
                    );
                }
            }

            tracing::debug!("Database sync operations completed for server '{}'", server_name_clone);
        });
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

    // get_default_instance_id method is now in instance_helpers.rs

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

    /// Update server status in the connection pool
    ///
    /// This is a unified interface for managing server status:
    /// - If enabled=true:
    ///   1. Loads latest config from active suits
    ///   2. Updates pool configuration
    ///   3. Creates new connection if needed
    ///   4. Connects to the server
    /// - If enabled=false:
    ///   1. Disconnects all instances
    ///   2. Removes server from pool
    ///
    /// This implementation replaces the previous separate start_server, stop_server,
    /// and load_server_config_dynamic functions with a single, more consistent interface.
    pub async fn update_server_status(
        &mut self,
        server_name: &str,
        enabled: bool,
    ) -> Result<()> {
        if enabled {
            self.enable_server_internal(server_name).await
        } else {
            self.disable_server_internal(server_name).await
        }
    }

    /// Internal method to enable and start a server
    async fn enable_server_internal(
        &mut self,
        server_name: &str,
    ) -> Result<()> {
        // Early return if database not available
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not available"))?;

        // Load latest config for this server
        let (_, config) = crate::core::foundation::loader::load_servers_from_active_suits(db).await?;

        // Early return if server not found in config
        let Some(_server_config) = config.mcp_servers.get(server_name) else {
            return Err(anyhow::anyhow!(
                "Server '{}' not found in active configuration suits",
                server_name
            ));
        };

        // Update config and start server
        self.set_config(Arc::new(config))?;

        // Create new connection if needed
        if !self.connections.contains_key(server_name) {
            let connection = crate::core::connection::UpstreamConnection::new(server_name.to_string());
            let instance_id = connection.id.clone();
            let instances = self.connections.entry(server_name.to_string()).or_default();
            instances.insert(instance_id.clone(), connection);
        }

        // Get default instance ID and connect
        let instance_id = self.get_default_instance_id(server_name)?;
        self.trigger_connect(server_name, &instance_id).await?;

        tracing::info!("Server '{}' enabled and started", server_name);
        Ok(())
    }

    /// Internal method to disable and stop a server
    async fn disable_server_internal(
        &mut self,
        server_name: &str,
    ) -> Result<()> {
        // Early return if database not available
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not available"))?;

        // Check if server should remain enabled in any active suit
        if let Some(server_id) = self.get_server_id_by_name(db, server_name).await? {
            let still_enabled_in_suits =
                crate::config::server::is_server_enabled_in_any_active_suit(&db.pool, &server_id)
                    .await
                    .unwrap_or(false);

            // Early return if still enabled in other suits
            if still_enabled_in_suits {
                tracing::info!(
                    "Server '{}' disabled in one suit but still enabled in other active suits, keeping instance running",
                    server_name
                );
                return Ok(());
            }
        }

        // Disconnect all instances
        self.disconnect_all_instances(server_name).await;

        // Remove server from pool
        self.connections.remove(server_name);
        self.cancellation_tokens.remove(server_name);

        tracing::info!("Server '{}' disabled in all active suits and stopped", server_name);
        Ok(())
    }

    /// Helper method to get server ID by name
    async fn get_server_id_by_name(
        &self,
        db: &crate::config::database::Database,
        server_name: &str,
    ) -> Result<Option<String>> {
        let all_servers = crate::config::server::get_all_servers(&db.pool).await?;
        let server_id = all_servers
            .iter()
            .find(|s| s.name == server_name)
            .and_then(|s| s.id.clone());
        Ok(server_id)
    }

    /// Helper method to disconnect all instances of a server
    async fn disconnect_all_instances(
        &mut self,
        server_name: &str,
    ) {
        let Some(instances) = self.connections.get(server_name) else {
            return; // No instances to disconnect, early return
        };

        let instance_ids: Vec<String> = instances.keys().cloned().collect();
        for instance_id in instance_ids {
            if let Err(e) = self.disconnect(server_name, &instance_id).await {
                tracing::warn!(
                    "Failed to disconnect server '{}' instance '{}': {}",
                    server_name,
                    instance_id,
                    e
                );
            }
        }
    }
}
