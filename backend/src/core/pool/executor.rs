//! Pool connection execution and management functionality
//!
//! Provides connection lifecycle management for UpstreamConnectionPool including:
//! - connection establishment and teardown
//! - reconnection logic with exponential backoff
//! - parallel connection capabilities
//! - service lifecycle management

use std::{future::Future, sync::Arc, time::Duration};

use anyhow::Result;
use rmcp::model::{ServerCapabilities, Tool};
use tracing;

use super::{FailureKind, UpstreamConnectionPool};
use crate::{
    common::{
        server::{ServerType, TransportType},
        sync::SyncHelper,
    },
    core::capability::ConnectionSelection,
    core::{
        events,
        foundation::types::{
            ConnectionOperation, // action to perform on the connection
            ConnectionStatus,    // status of the connection
        },
        transport::connect_http_server,
    },
};

/// Parallel connection manager (scheduler only)
pub struct ParallelConnectionManager {
    pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
}

impl ParallelConnectionManager {
    /// Create a new parallel connection manager
    pub fn new(pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>) -> Self {
        Self { pool }
    }

    /// High-performance parallel connection to all servers (scheduler only)
    pub async fn trigger_connect_all_parallel(&self) -> Result<()> {
        // Get server instances (read-only operation)
        let server_instances = {
            let pool = self.pool.lock().await;
            pool.connections
                .keys()
                .filter_map(|server_id| {
                    pool.get_default_instance_id(server_id)
                        .ok()
                        .map(|instance_id| (server_id.clone(), instance_id))
                })
                .collect::<Vec<_>>()
        };

        if server_instances.is_empty() {
            tracing::debug!("No servers to connect to");
            return Ok(());
        }

        tracing::info!("Starting parallel connection to {} servers", server_instances.len());

        let pool = self.pool.clone();

        // Use unified sync framework for concurrent connections (scheduler only)
        let sync_result = SyncHelper::execute_concurrent_sync(
            server_instances,
            "server_connections",
            4, // max concurrent connections
            move |(server_id, instance_id)| {
                let pool = pool.clone();
                async move { Self::connect_single_server(server_id, instance_id, pool).await }
            },
        )
        .await;

        tracing::info!(
            "Parallel connection completed: {}/{} tasks successful ({:.1}% success rate)",
            sync_result.synced,
            sync_result.processed,
            sync_result.success_rate()
        );

        Ok(())
    }

    /// Single server connection task (stateless, parallelizable)
    async fn connect_single_server(
        server_id: String,
        instance_id: String,
        pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>>,
    ) -> Result<()> {
        // Delegate to pool's single executor; events/tokens handled inside connect_internal
        let mut pool = pool.lock().await;
        let _ = pool.connect_via_scheduler(&server_id, &instance_id).await;
        Ok(())
    }
}

impl UpstreamConnectionPool {
    /// Build a human-readable server label: "<name> (<id>)" if DB is available, else just id
    async fn server_label(
        &self,
        server_id: &str,
    ) -> String {
        if let Some(db) = &self.database {
            if let Ok(name) = crate::config::operations::utils::get_server_name(&db.pool, server_id).await {
                return format!("{} ({})", name, server_id);
            }
        }
        server_id.to_string()
    }
    /// Create a parallel connection manager for this pool
    pub fn create_parallel_manager(pool: Arc<tokio::sync::Mutex<Self>>) -> ParallelConnectionManager {
        ParallelConnectionManager::new(pool)
    }

    /// Trigger parallel connection to all servers using the new manager
    pub async fn trigger_connect_all_parallel_new(pool: Arc<tokio::sync::Mutex<Self>>) -> Result<()> {
        let manager = Self::create_parallel_manager(pool);
        manager.trigger_connect_all_parallel().await
    }

    /// Reconnect to a specific instance of a server (non-blocking)
    ///
    /// Simplified: rely on pool-level FailureState backoff; attempt immediate reconnect.
    pub async fn reconnect(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        // First perform a non-blocking disconnect
        self.disconnect_non_blocking(server_id, instance_id).await?;
        tracing::info!(
            "Scheduling immediate reconnection to server_id '{}' instance '{}' (pool-level backoff applies)",
            server_id,
            instance_id
        );
        self.connect_internal(server_id, instance_id).await
    }

    /// Internal implementation for disconnect with a non_blocking option
    async fn disconnect_inner(
        &mut self,
        server_id: &str,
        instance_id: &str,
        non_blocking: bool,
    ) -> Result<()> {
        if non_blocking {
            let label = self.server_label(server_id).await;
            tracing::debug!("Non-blocking disconnect for '{}' instance '{}'", label, instance_id);
        } else {
            let label = self.server_label(server_id).await;
            tracing::info!(
                "disconnect() called for '{}' instance '{}' - Stack trace requested",
                label,
                instance_id
            );
            let backtrace = std::backtrace::Backtrace::capture();
            tracing::info!("Disconnect initiated from: {}", backtrace);
        }

        self.cancel_connection_token(server_id, instance_id);

        let service = {
            let conn = self.get_instance_mut(server_id, instance_id)?;
            conn.service.take()
        };

        if non_blocking {
            let conn = self.get_instance_mut(server_id, instance_id)?;
            conn.update_disconnected();

            if let Some(service_arc) = service {
                self.cancel_service_async(server_id, instance_id, service_arc);
            }

            let label = self.server_label(server_id).await;
            tracing::info!(
                "Non-blocking disconnect completed for '{}' instance '{}'",
                label,
                instance_id
            );
        } else {
            if let Some(service_arc) = service {
                self.cancel_service_with_timeout(server_id, instance_id, service_arc)
                    .await;
            }

            let conn = self.get_instance_mut(server_id, instance_id)?;
            conn.update_disconnected();

            let label = self.server_label(server_id).await;
            tracing::info!("Disconnected from server '{}' instance '{}'", label, instance_id);
        }

        // Clean up all affinity-owned state for this instance:
        // - Production route mappings pointing to this instance
        // - Client-bound connection entries for this instance
        self.cleanup_disconnected_instance(server_id, instance_id);

        Ok(())
    }

    /// Disconnect from a specific instance of a server with improved resource cleanup
    pub async fn disconnect(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        self.disconnect_inner(server_id, instance_id, false).await
    }

    /// Non-blocking disconnect that avoids long service cancellation timeouts
    pub async fn disconnect_non_blocking(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        self.disconnect_inner(server_id, instance_id, true).await
    }

    /// Disconnect from all servers
    pub async fn disconnect_all(&mut self) -> Result<()> {
        for server_id in self.connections.keys().cloned().collect::<Vec<_>>() {
            if let Some(instances) = self.connections.get(&server_id) {
                for instance_id in instances.keys().cloned().collect::<Vec<_>>() {
                    if let Err(e) = self.disconnect(&server_id, &instance_id).await {
                        tracing::error!(
                            "Failed to disconnect from server '{}' instance '{}': {}",
                            server_id,
                            instance_id,
                            e
                        );
                    }
                }
            }
        }

        for (server_id, bound_id) in self.client_bound_connections.keys().cloned().collect::<Vec<_>>() {
            if let Some(instances) = self
                .client_bound_connections
                .get(&(server_id.clone(), bound_id.clone()))
            {
                for instance_id in instances.keys().cloned().collect::<Vec<_>>() {
                    if let Err(e) = self.disconnect(&server_id, &instance_id).await {
                        tracing::error!(
                            "Failed to disconnect client-bound instance '{}' for server '{}' bound '{}': {}",
                            instance_id,
                            server_id,
                            bound_id,
                            e
                        );
                    }
                }
            }
        }

        self.production_routes.clear();

        Ok(())
    }

    // Removed redundant implementations in favor of update_server_status

    /// Connect to a specific instance of a server
    pub async fn connect(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        self.connect_internal(server_id, instance_id).await
    }

    /// Trigger connection to a specific instance of a server (alias for connect)
    pub async fn trigger_connect(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        self.connect_internal(server_id, instance_id).await
    }

    /// Ensure a server has at least one connected instance and return its instance_id.
    ///
    /// **Note**: This method uses `AffinityKey::Default` and bypasses affinity-aware routing.
    /// For production routing, use `ensure_connected_with_selection()` with a proper
    /// `ConnectionSelection` derived from `ClientContext::connection_selection()`.
    pub async fn ensure_connected(
        &mut self,
        server_id: &str,
    ) -> Result<String> {
        let selection = ConnectionSelection {
            server_id: server_id.to_string(),
            affinity_key: crate::core::capability::AffinityKey::Default,
        };
        self.ensure_connected_with_selection(&selection).await
    }

    /// Ensure a connection exists for a production route using affinity-aware routing.
    ///
    /// This method implements the full production routing lifecycle:
    /// 1. Check if a route already exists via `resolve_production_route`
    /// 2. If exists and instance is ready, return the instance_id
    /// 3. If not, allocate a new route via `allocate_production_route`
    /// 4. Establish the connection via `connect_internal`
    pub async fn ensure_connected_with_selection(
        &mut self,
        selection: &ConnectionSelection,
    ) -> Result<String> {
        if let Some(instance_id) = self.resolve_production_route(selection) {
            if let Ok(Some(ready_id)) = self.select_ready_instance_id(selection) {
                if ready_id == instance_id {
                    return Ok(instance_id);
                }
            }
        }

        let instance_id = self.allocate_production_route(selection);
        self.connect_internal(&selection.server_id, &instance_id).await?;
        Ok(instance_id)
    }

    /// Internal connection logic
    async fn connect_internal(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        if let Some(remaining) = self.remaining_backoff(server_id) {
            tracing::warn!(
                server_id = server_id,
                wait_secs = remaining.as_secs_f32(),
                "Connection attempt blocked due to active backoff"
            );
            return Err(anyhow::anyhow!(
                "Server '{}' is backing off for {:.1}s",
                server_id,
                remaining.as_secs_f32()
            ));
        }

        // Get server configuration (clone to avoid borrowing issues)
        let server_config = self
            .config
            .mcp_servers
            .get(server_id)
            .ok_or_else(|| anyhow::anyhow!("Server '{}' not found in configuration", server_id))?
            .clone();

        // Update connection status to initializing
        {
            let conn = self.get_instance_mut(server_id, instance_id)?;
            conn.update_initializing();
        }

        // Connect based on server type using enum matching (strict type safety)
        let result = match server_config.kind {
            ServerType::Stdio => self.connect_stdio(server_id, instance_id).await,
            ServerType::StreamableHttp => self.connect_http(server_id, instance_id).await,
        };

        // Handle connection result
        match result {
            Ok(()) => {
                tracing::info!(
                    "Successfully initiated connection to server_id '{}' instance '{}'",
                    server_id,
                    instance_id
                );
                self.clear_failure_state(server_id);
                if let Ok(conn) = self.get_instance_mut(server_id, instance_id) {
                    conn.reset_connection_attempts();
                }
                // Publish success event via unified outlet
                self.publish_startup_event(server_id, true, None).await;
                Ok(())
            }
            Err(e) => {
                // Set simple error status; pool-level backoff governs throttling
                if let Ok(conn) = self.get_instance_mut(server_id, instance_id) {
                    conn.update_failed(format!("Connection failed: {}", e));
                }

                tracing::error!(
                    "Failed to connect to server_id '{}' instance '{}': {}",
                    server_id,
                    instance_id,
                    e
                );
                let failure_reason = format!("{}", e);
                self.register_failure(server_id, FailureKind::Connect, Some(failure_reason));
                // Publish failure event via unified outlet
                self.publish_startup_event(server_id, false, Some(format!("{}", e)))
                    .await;
                Err(e)
            }
        }
    }

    /// Connect to stdio server
    async fn connect_stdio(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        let server_config = self.config.mcp_servers.get(server_id).unwrap();

        // Create cancellation token for this connection
        let ct = tokio_util::sync::CancellationToken::new();

        // Store the cancellation token
        self.cancellation_tokens
            .entry(server_id.to_string())
            .or_default()
            .insert(instance_id.to_string(), ct.clone());

        // Get database pool if available
        let database_pool = self.database.as_ref().map(|db| &db.pool);

        // Use the unified transport interface to reduce code duplication
        // Note: connect_server still needs server_name for logging, so we use server_id as name for now
        let (service, tools, capabilities, process_id) = crate::core::transport::unified::connect_server(
            server_id,
            server_config,
            crate::common::server::ServerType::Stdio,
            crate::common::server::TransportType::Stdio,
            Some(ct),
            database_pool,
            self.runtime_cache.as_ref().map(|rc| rc.as_ref()),
        )
        .await?;

        // Update connection with service
        self.update_connection(server_id, instance_id, service, tools, capabilities);

        // Update process ID if available
        if let Some(pid) = process_id {
            if let Ok(conn) = self.get_instance_mut(server_id, instance_id) {
                conn.process_id = Some(pid);
            }
        }

        Ok(())
    }

    /// Wrapper for scheduler to trigger connection using the single executor
    pub(crate) async fn connect_via_scheduler(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        self.connect_internal(server_id, instance_id).await
    }

    /// Publish connection startup result (success/failure) as a single unified event outlet
    async fn publish_startup_event(
        &self,
        server_id: &str,
        success: bool,
        error: Option<String>,
    ) {
        // Resolve server_name for event payload when database is available
        let server_name = if let Some(db) = &self.database {
            match crate::config::operations::utils::get_server_name(&db.pool, server_id).await {
                Ok(name) => name,
                Err(_) => server_id.to_string(),
            }
        } else {
            server_id.to_string()
        };

        events::EventBus::global().publish(events::Event::ServerConnectionStartupCompleted {
            server_id: server_id.to_string(),
            server_name,
            success,
            error,
        });
    }

    /// Connect to HTTP server
    async fn connect_http(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) -> Result<()> {
        // Prepare optional shared client
        let client_opt = if let Some(reg) = &self.http_clients {
            if let Some(url) = self.config.mcp_servers.get(server_id).and_then(|c| c.url.as_ref()) {
                if let Some(origin) = crate::core::pool::connection::HttpClientRegistry::origin_key(url) {
                    Some(reg.get_or_create(&origin).await)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(client) = &client_opt {
            if let Some(url) = self.config.mcp_servers.get(server_id).and_then(|c| c.url.as_ref()) {
                if let Some(origin) = crate::core::pool::connection::HttpClientRegistry::origin_key(url) {
                    tracing::debug!(
                        "[HTTP CLIENT][reuse] server_id={} origin={} client={:p}",
                        server_id,
                        origin,
                        client
                    );
                }
            }
        } else {
            tracing::debug!("[HTTP CLIENT][no-reuse] server_id={} (HTTP)", server_id);
        }

        self.connect_with_transport(server_id, instance_id, move |label, config| {
            let client_opt = client_opt.clone();
            async move {
                // If server has default headers, use a per-server client with those headers
                if let Some(h) = config.headers.as_ref() {
                    if !h.is_empty() {
                        let mut header_map = reqwest::header::HeaderMap::new();
                        for (k, v) in h.iter() {
                            if let (Ok(name), Ok(value)) = (
                                reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                                reqwest::header::HeaderValue::from_str(v),
                            ) {
                                let controlled = matches!(
                                    name.as_str().to_ascii_lowercase().as_str(),
                                    "accept"
                                        | "content-length"
                                        | "host"
                                        | "connection"
                                        | "transfer-encoding"
                                        | "mcp-protocol-version"
                                );
                                if controlled {
                                    continue;
                                }
                                header_map.insert(name, value);
                            }
                        }
                        let client = reqwest::Client::builder().default_headers(header_map).build()?;
                        return crate::core::transport::http::connect_http_server_with_client(
                            &label,
                            &config,
                            client,
                            TransportType::StreamableHttp,
                        )
                        .await;
                    }
                }
                // else fall back to shared client or plain connection
                if let Some(client) = client_opt {
                    crate::core::transport::http::connect_http_server_with_client(
                        &label,
                        &config,
                        client,
                        TransportType::StreamableHttp,
                    )
                    .await
                } else {
                    connect_http_server(&label, &config, TransportType::StreamableHttp).await
                }
            }
        })
        .await
    }

    async fn connect_with_transport<F, Fut>(
        &mut self,
        server_id: &str,
        instance_id: &str,
        connect_fn: F,
    ) -> Result<()>
    where
        F: FnOnce(String, crate::core::models::MCPServerConfig) -> Fut,
        Fut: Future<
            Output = Result<(
                crate::core::transport::ClientService,
                Vec<Tool>,
                Option<ServerCapabilities>,
            )>,
        >,
    {
        let server_config = self.config.mcp_servers.get(server_id).unwrap().clone();
        // Build a friendly label for transport-layer logging: "name (id)" or just id
        let label = crate::core::capability::resolver::label_by_id(server_id).await;
        let (service, tools, capabilities) = connect_fn(label, server_config).await?;
        self.update_connection(server_id, instance_id, service, tools, capabilities);
        Ok(())
    }

    /// Cancel connection token for a specific instance
    fn cancel_connection_token(
        &mut self,
        server_id: &str,
        instance_id: &str,
    ) {
        let token_opt = self
            .cancellation_tokens
            .get_mut(server_id)
            .and_then(|tokens| tokens.remove(instance_id));

        let Some(token) = token_opt else {
            return; // No token to cancel, early return
        };

        token.cancel();
        tracing::debug!(
            "Cancelled token for server '{}' instance '{}' to stop new operations",
            server_id,
            instance_id
        );
    }

    /// Cancel service with timeout handling
    async fn cancel_service_with_timeout(
        &self,
        server_id: &str,
        instance_id: &str,
        service_arc: Arc<crate::core::transport::ClientService>,
    ) {
        let cancel_timeout = Duration::from_secs(5);
        let label = self.server_label(server_id).await;
        tracing::info!(
            "About to cancel service for '{}' instance '{}' with {}s timeout",
            label,
            instance_id,
            cancel_timeout.as_secs()
        );

        // Try to extract the service from Arc for cancellation
        let service = match Arc::try_unwrap(service_arc) {
            Ok(service) => service,
            Err(_arc) => {
                tracing::warn!(
                    "Cannot cancel service for '{}' instance '{}' - multiple references exist",
                    label,
                    instance_id
                );
                return;
            }
        };

        // Handle service cancellation with timeout
        match tokio::time::timeout(cancel_timeout, service.cancel()).await {
            Ok(Ok(quit_reason)) => {
                tracing::info!(
                    "Service for server '{}' instance '{}' cancelled gracefully with reason: {:?}",
                    label,
                    instance_id,
                    quit_reason
                );
            }
            Ok(Err(e)) => {
                tracing::warn!(
                    "Error during graceful cancellation for '{}' instance '{}': {}",
                    label,
                    instance_id,
                    e
                );
            }
            Err(_) => {
                tracing::warn!(
                    "Service cancellation timeout for '{}' instance '{}' ({}s)",
                    label,
                    instance_id,
                    cancel_timeout.as_secs()
                );
            }
        }
    }

    /// Cancel service asynchronously without blocking the caller
    fn cancel_service_async(
        &self,
        server_id: &str,
        instance_id: &str,
        service_arc: Arc<crate::core::transport::ClientService>,
    ) {
        let instance_id = instance_id.to_string();

        // Resolve label synchronously by best-effort using cached id; we cannot await here.
        // For better readability, include id in logs.
        let label = server_id.to_string();

        // Spawn background task for service cancellation
        tokio::spawn(async move {
            let cancel_timeout = Duration::from_secs(3); // Reduced timeout for faster response

            tracing::debug!(
                "Async service cancellation started for '{}' instance '{}' with {}s timeout",
                label,
                instance_id,
                cancel_timeout.as_secs()
            );

            // Try to extract the service from Arc for cancellation
            let service = match Arc::try_unwrap(service_arc) {
                Ok(service) => service,
                Err(_arc) => {
                    tracing::debug!(
                        "Service for '{}' instance '{}' has multiple references, skipping cancellation",
                        label,
                        instance_id
                    );
                    return;
                }
            };

            // Handle service cancellation with timeout
            match tokio::time::timeout(cancel_timeout, service.cancel()).await {
                Ok(Ok(quit_reason)) => {
                    tracing::debug!(
                        "Service for '{}' instance '{}' cancelled gracefully: {:?}",
                        label,
                        instance_id,
                        quit_reason
                    );
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        "Error during async cancellation for '{}' instance '{}': {}",
                        label,
                        instance_id,
                        e
                    );
                }
                Err(_) => {
                    tracing::warn!(
                        "Async service cancellation timeout for '{}' instance '{}' ({}s)",
                        label,
                        instance_id,
                        cancel_timeout.as_secs()
                    );
                }
            }
        });
    }

    /// Update connection with service and metadata
    pub fn update_connection(
        &mut self,
        server_id: &str,
        instance_id: &str,
        service: crate::core::transport::ClientService,
        tools: Vec<Tool>,
        capabilities: Option<rmcp::model::ServerCapabilities>,
    ) {
        // Early return if connection cannot be retrieved
        let Ok(conn) = self.get_instance_mut(server_id, instance_id) else {
            tracing::error!(
                "Failed to update connection for '{}' instance '{}' - connection not found",
                server_id,
                instance_id
            );
            return;
        };

        // Check server capabilities
        let supports_resources = capabilities.as_ref().and_then(|caps| caps.resources.as_ref()).is_some();
        let supports_prompts = capabilities.as_ref().and_then(|caps| caps.prompts.as_ref()).is_some();

        // Clone service metadata before moving into Arc
        let peer_info = service.peer_info().cloned();
        let server_icons_payload = peer_info.as_ref().and_then(|info| info.server_info.icons.clone());

        // Clone service for database sync operations
        let service_for_sync = service.peer().clone();

        // Update connection properties
        // Set server_id on upstream client handler so notifications can be forwarded correctly
        service.service().set_server_id(server_id);
        conn.service = Some(Arc::new(service));
        conn.tools = tools.clone();
        conn.capabilities = capabilities;
        conn.update_ready();

        tracing::debug!(
            "Updated connection for '{}' instance '{}' with service and {} tools",
            server_id,
            instance_id,
            conn.tools.len()
        );

        // Handle database sync (early return if no database)
        let Some(db) = &self.database else {
            return; // No database available, skip sync operations
        };

        if let Some(peer) = peer_info.as_ref() {
            let db_clone = db.clone();
            let server_id_clone = server_id.to_string();
            let icons_for_update = server_icons_payload.clone();
            let server_version = peer.server_info.version.clone();
            let protocol_version = peer.protocol_version.to_string();

            tokio::spawn(async move {
                if let Err(e) = crate::config::server::meta::update_server_versions(
                    &db_clone.pool,
                    &server_id_clone,
                    Some(server_version.clone()),
                    protocol_version.clone(),
                )
                .await
                {
                    tracing::warn!(
                        server_id = %server_id_clone,
                        error = %e,
                        "Failed to upsert server version metadata"
                    );
                }

                if let Err(e) =
                    crate::config::server::meta::update_server_icons(&db_clone.pool, &server_id_clone, icons_for_update)
                        .await
                {
                    tracing::warn!(
                        server_id = %server_id_clone,
                        error = %e,
                        "Failed to upsert server icons"
                    );
                }
            });
        }

        self.spawn_database_sync_task(
            db.clone(),
            server_id.to_string(),
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
        server_id_clone: String,
        instance_id_clone: String,
        tools_clone: Vec<Tool>,
        service_for_sync: rmcp::service::Peer<rmcp::service::RoleClient>,
        supports_resources: bool,
        supports_prompts: bool,
    ) {
        tokio::spawn(async move {
            // Build flags: always include tools; resources imply resource templates as well
            let mut flags = crate::core::pool::CapSyncFlags::TOOLS;
            if supports_resources {
                flags = crate::core::pool::CapSyncFlags(
                    flags.0
                        | crate::core::pool::CapSyncFlags::RESOURCES.0
                        | crate::core::pool::CapSyncFlags::RESOURCE_TEMPLATES.0,
                );
            }
            if supports_prompts {
                flags = crate::core::pool::CapSyncFlags(flags.0 | crate::core::pool::CapSyncFlags::PROMPTS.0);
            }

            // Unified capabilities sync (single entry point)
            if let Err(e) = UpstreamConnectionPool::sync_capabilities(
                &db_clone,
                &server_id_clone,
                &instance_id_clone,
                &service_for_sync,
                flags,
                Some(&tools_clone),
            )
            .await
            {
                tracing::error!("Unified capability sync failed for server '{}': {}", server_id_clone, e);
            } else {
                tracing::debug!("Unified capability sync completed for server '{}'", server_id_clone);
            }
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
        // Validate operation before proceeding
        self.validate_operation(server_name, instance_id, operation).await?;

        // Route to appropriate operation handler
        match operation {
            ConnectionOperation::Disconnect | ConnectionOperation::ForceDisconnect | ConnectionOperation::Cancel => {
                self.disconnect(server_name, instance_id).await
            }
            ConnectionOperation::Reconnect => self.reconnect(server_name, instance_id).await,
            ConnectionOperation::Connect => self.connect(server_name, instance_id).await,
            ConnectionOperation::ResetReconnect => self.handle_reset_reconnect(server_name, instance_id).await,
            ConnectionOperation::Recover => Err(anyhow::anyhow!(
                "Recover operation should be handled directly via manual_re_enable method"
            )),
        }
    }

    /// Validate if operation is allowed in current state
    async fn validate_operation(
        &self,
        server_name: &str,
        instance_id: &str,
        operation: ConnectionOperation,
    ) -> Result<()> {
        let conn = self.get_instance(server_name, instance_id)?;

        if !conn.status.can_perform_operation(operation) {
            return Err(anyhow::anyhow!(
                "Operation '{}' is not allowed in the current state: {}",
                operation,
                conn.status
            ));
        }

        Ok(())
    }

    /// Handle reset reconnect operation
    async fn handle_reset_reconnect(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        // First disconnect if needed
        if !self.is_instance_shutdown(server_name, instance_id) {
            if let Err(e) = self.disconnect(server_name, instance_id).await {
                tracing::warn!("Error during reset_reconnect disconnect phase: {}", e);
            }
        }

        // Reset connection attempts counter
        self.reset_connection_attempts(server_name, instance_id)?;

        // Then reconnect
        self.connect_internal(server_name, instance_id).await
    }

    /// Check if instance is in shutdown state
    fn is_instance_shutdown(
        &self,
        server_name: &str,
        instance_id: &str,
    ) -> bool {
        self.get_instance(server_name, instance_id)
            .map(|conn| matches!(conn.status, ConnectionStatus::Shutdown))
            .unwrap_or(true) // Treat missing as shutdown
    }

    /// Reset connection attempts counter for instance
    fn reset_connection_attempts(
        &mut self,
        server_name: &str,
        instance_id: &str,
    ) -> Result<()> {
        let conn = self.get_instance_mut(server_name, instance_id)?;
        conn.reset_connection_attempts();
        Ok(())
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
    /// - If enabled=true: Loads latest config, creates connection, and connects
    /// - If enabled=false: Disconnects all instances and removes from pool
    pub async fn update_server_status(
        &mut self,
        server_id: &str,
        enabled: bool,
    ) -> Result<()> {
        if enabled {
            self.enable_server(server_id).await
        } else {
            self.disable_server(server_id).await
        }
    }

    /// Enable and start a server.
    ///
    /// This is an admin/control-plane operation that creates a shared connection.
    /// Production routing will use `ensure_connected_with_selection()` with proper affinity.
    pub async fn enable_server(
        &mut self,
        server_id: &str,
    ) -> Result<()> {
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not available"))?;

        let config = crate::core::foundation::loader::load_pool_base_config(db).await?;

        let Some(_server_config) = config.mcp_servers.get(server_id) else {
            return Err(anyhow::anyhow!(
                "Server '{}' not found in pool base configuration",
                server_id
            ));
        };

        self.set_config(Arc::new(config))?;

        if !self.connections.contains_key(server_id) {
            let connection = crate::core::pool::UpstreamConnection::new(server_id.to_string());
            let instance_id = connection.id.clone();
            let instances = self.connections.entry(server_id.to_string()).or_default();
            instances.insert(instance_id.clone(), connection);
        }

        let instance_id = self.get_default_instance_id(server_id)?;
        self.connect_internal(server_id, &instance_id).await?;

        tracing::info!("Server '{}' enabled and started", server_id);
        Ok(())
    }

    /// Disable and stop a server
    pub async fn disable_server(
        &mut self,
        server_id: &str,
    ) -> Result<()> {
        // Early return if database not available
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not available"))?;

        // Check if server should remain enabled in any active profile
        let still_enabled_in_profile =
            crate::config::server::is_server_enabled_in_any_active_profile(&db.pool, server_id)
                .await
                .unwrap_or(false);

        // Early return if still enabled in other profile
        if still_enabled_in_profile {
            tracing::info!(
                "Server '{}' disabled in one profile but still enabled in other active profile, keeping instance running",
                server_id
            );
            return Ok(());
        }

        // Disconnect all instances
        self.disconnect_all_instances(server_id).await;

        self.connections.remove(server_id);
        self.cancellation_tokens.remove(server_id);
        self.remove_all_client_bound_connections_for_server(server_id);
        self.remove_all_production_routes_for_server(server_id);

        tracing::info!("Server '{}' disabled in all active profile and stopped", server_id);
        Ok(())
    }

    /// Helper method to disconnect all instances of a server
    async fn disconnect_all_instances(
        &mut self,
        server_id: &str,
    ) {
        if let Some(instances) = self.connections.get(server_id) {
            let instance_ids: Vec<String> = instances.keys().cloned().collect();
            for instance_id in instance_ids {
                if let Err(e) = self.disconnect(server_id, &instance_id).await {
                    tracing::warn!(
                        "Failed to disconnect server '{}' instance '{}': {}",
                        server_id,
                        instance_id,
                        e
                    );
                }
            }
        }

        let bound_keys: Vec<(String, String)> = self
            .client_bound_connections
            .keys()
            .filter(|(sid, _)| sid == server_id)
            .cloned()
            .collect();
        for (sid, bound_id) in bound_keys {
            if let Some(instances) = self.client_bound_connections.get(&(sid.clone(), bound_id.clone())) {
                let instance_ids: Vec<String> = instances.keys().cloned().collect();
                for instance_id in instance_ids {
                    if let Err(e) = self.disconnect(&sid, &instance_id).await {
                        tracing::warn!(
                            "Failed to disconnect client-bound instance '{}' for server '{}' bound '{}': {}",
                            instance_id,
                            sid,
                            bound_id,
                            e
                        );
                    }
                }
            }
        }
    }
}
