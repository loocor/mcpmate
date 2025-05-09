// HTTP proxy server implementation for MCPMate

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult, ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool, Error as McpError, ServerHandler,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::core::TransportType;

use crate::{
    core::config::Config,
    core::tool::parse_tool_name,
    core::{tool::get_all_tools_with_smart_prefix, UpstreamConnectionPool},
    conf::Database,
};

/// HTTP Proxy Server that aggregates tools from multiple MCP servers
#[derive(Debug, Clone)]
pub struct HttpProxyServer {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    /// Tool name mapping cache
    tool_name_mapping_cache:
        Arc<Mutex<Option<HashMap<String, crate::core::tool::ToolNameMapping>>>>,
    /// Last time the tool name mapping was updated
    last_tool_mapping_update: Arc<Mutex<std::time::Instant>>,
    /// Last connection state hash, used to detect changes in the connection pool
    last_connection_state_hash: Arc<Mutex<u64>>,
    /// Database connection for tool configuration persistence
    pub db: Option<Arc<Database>>,
}

#[tool(tool_box)]
impl HttpProxyServer {
    /// Send a tool list changed notification to all connected clients
    ///
    /// This method is used by the API server to notify clients when the tool list has changed
    pub async fn notify_tool_list_changed(
        &self,
        context: RequestContext<rmcp::RoleServer>,
    ) -> Result<(), McpError> {
        // Get the peer from the context
        let peer = context.peer;

        // Send the notification
        if let Err(e) = peer.notify_tool_list_changed().await {
            tracing::error!("Failed to send tool list changed notification: {}", e);
            return Err(McpError::internal_error(
                format!("Failed to send tool list changed notification: {}", e),
                None,
            ));
        }

        tracing::info!("Sent tool list changed notification to client");
        Ok(())
    }

    /// Get the cached tool name mapping, or build a new one if the cache is expired or empty
    ///
    /// This optimized version uses a more efficient caching strategy:
    /// 1. Uses a longer cache expiration time (2 minutes instead of 30 seconds)
    /// 2. Prioritizes connection state hash changes over time-based expiration
    /// 3. Provides detailed logging about cache update reasons
    /// 4. Reduces lock contention by acquiring locks only when necessary
    async fn get_tool_name_mapping(&self) -> HashMap<String, crate::core::tool::ToolNameMapping> {
        // Cache expiration time (2 minutes)
        const CACHE_EXPIRATION: std::time::Duration = std::time::Duration::from_secs(120);

        // First, check if cache exists without calculating hash (fast path)
        let cache_exists = {
            let cache = self.tool_name_mapping_cache.lock().await;
            cache.is_some()
        };

        // If cache doesn't exist, we definitely need to update
        if !cache_exists {
            return self
                .rebuild_tool_mapping_cache("Cache is empty (first use)")
                .await;
        }

        // Calculate current connection state hash
        let current_hash = {
            let pool = self.connection_pool.lock().await;
            pool.calculate_connection_state_hash()
        };

        // Check if connection state has changed
        let hash_changed = {
            let last_hash = self.last_connection_state_hash.lock().await;
            *last_hash != current_hash
        };

        // If hash changed, update cache immediately
        if hash_changed {
            return self
                .rebuild_tool_mapping_cache("Connection state changed")
                .await;
        }

        // Check if cache has expired (only if hash hasn't changed)
        let cache_expired = {
            let last_update = self.last_tool_mapping_update.lock().await;
            last_update.elapsed() > CACHE_EXPIRATION
        };

        // If cache has expired, update it
        if cache_expired {
            return self.rebuild_tool_mapping_cache("Cache expired").await;
        }

        // Use the cached mapping (fast path)
        let cache = self.tool_name_mapping_cache.lock().await;
        cache.as_ref().unwrap().clone()
    }

    /// Helper method to rebuild the tool mapping cache
    async fn rebuild_tool_mapping_cache(
        &self,
        reason: &str,
    ) -> HashMap<String, crate::core::tool::ToolNameMapping> {
        // Calculate current hash
        let current_hash = {
            let pool = self.connection_pool.lock().await;
            pool.calculate_connection_state_hash()
        };

        // Build a new tool name mapping
        let start_time = std::time::Instant::now();
        let new_mapping = crate::core::tool::build_tool_name_mapping(&self.connection_pool).await;
        let build_time = start_time.elapsed();

        // Update the cache
        {
            let mut cache = self.tool_name_mapping_cache.lock().await;
            *cache = Some(new_mapping.clone());

            // Update the last update time
            let mut last_update = self.last_tool_mapping_update.lock().await;
            *last_update = std::time::Instant::now();

            // Update the last connection state hash
            let mut last_hash = self.last_connection_state_hash.lock().await;
            *last_hash = current_hash;
        }

        tracing::info!(
            "Updated tool name mapping cache with {} entries (reason: {}, build time: {:?})",
            new_mapping.len(),
            reason,
            build_time
        );

        new_mapping
    }

    /// Create a new HTTP proxy server
    pub fn new(config: Arc<Config>, rule_config: Arc<HashMap<String, bool>>) -> Self {
        // Create connection pool
        let mut pool = UpstreamConnectionPool::new(config, rule_config);

        // Initialize the pool
        pool.initialize();

        let connection_pool = Arc::new(Mutex::new(pool));

        // Start health check task
        UpstreamConnectionPool::start_health_check(connection_pool.clone());

        Self {
            connection_pool,
            tool_name_mapping_cache: Arc::new(Mutex::new(None)),
            last_tool_mapping_update: Arc::new(Mutex::new(std::time::Instant::now())),
            last_connection_state_hash: Arc::new(Mutex::new(0)),
            db: None, // Database will be initialized separately
        }
    }

    /// Initialize the database connection
    pub async fn init_database(&mut self) -> Result<()> {
        // Create database connection
        let db = Database::new().await?;

        // Initialize default values
        db.initialize_defaults().await?;

        // Store the database connection
        self.db = Some(Arc::new(db));

        tracing::info!("Database initialized successfully");
        Ok(())
    }

    /// Start the proxy server with specified transport type
    ///
    /// This method starts a proxy server with the specified transport type, address, and path.
    /// It's a convenience method that delegates to the appropriate specialized start method
    /// based on the transport type.
    ///
    /// Note: For maximum compatibility, consider using `start_unified` instead, which supports
    /// both Streamable HTTP and SSE protocols on the same port.
    ///
    /// # Arguments
    /// * `bind_address` - The socket address to bind the server to
    /// * `path` - The path for the server endpoint
    /// * `transport_type` - The transport type to use (SSE or Streamable HTTP)
    ///
    /// # Returns
    /// * `Result<()>` - Ok if the server started successfully, Err otherwise
    pub async fn start(
        &self,
        bind_address: SocketAddr,
        path: &str,
        transport_type: TransportType,
    ) -> Result<()> {
        tracing::info!(
            "Starting proxy server with transport type {:?} on {} at path {}",
            transport_type,
            bind_address,
            path
        );

        let result = match transport_type {
            TransportType::Sse => {
                tracing::info!("Using SSE transport mode (2024-11-05 MCP specification)");
                self.start_sse(bind_address, path)
                    .await
                    .context(format!("Failed to start SSE server on {}", bind_address))
            }
            TransportType::StreamableHttp => {
                tracing::info!(
                    "Using Streamable HTTP transport mode (2025-03-26 MCP specification)"
                );
                self.start_streamable_http(bind_address, path)
                    .await
                    .context(format!(
                        "Failed to start Streamable HTTP server on {}",
                        bind_address
                    ))
            }
            _ => {
                let err = anyhow::anyhow!(
                    "Unsupported transport type for server: {:?}. Supported types are SSE and StreamableHttp.",
                    transport_type
                );
                tracing::error!("{}", err);
                Err(err)
            }
        };

        if let Err(ref e) = result {
            tracing::error!("Failed to start server: {:#}", e); // Use {:#} to show the full error chain
        } else {
            tracing::info!("Server started successfully");
        }

        result
    }

    /// Create a service factory function that returns a new HttpProxyServer instance
    ///
    /// This helper method is used by all server start methods to create a factory function
    /// that returns a new HttpProxyServer instance for handling requests.
    fn create_service_factory(&self) -> impl Fn() -> Self + Clone + Send + Sync + 'static {
        let proxy_clone = self.clone();
        move || proxy_clone.clone()
    }

    /// Start the SSE server
    ///
    /// This method starts an SSE server on the specified address and path.
    /// The server will handle Server-Sent Events (SSE) connections from clients
    /// and route tool calls to the appropriate upstream servers.
    ///
    /// # Arguments
    /// * `bind_address` - The socket address to bind the server to
    /// * `sse_path` - The path for the SSE endpoint (e.g., "/sse")
    ///
    /// # Returns
    /// * `Result<()>` - Ok if the server started successfully, Err otherwise
    pub async fn start_sse(&self, bind_address: SocketAddr, sse_path: &str) -> Result<()> {
        tracing::info!(
            "Configuring SSE server on {} at path {}",
            bind_address,
            sse_path
        );

        // Create SSE server config
        let server_config = rmcp::transport::sse_server::SseServerConfig {
            bind: bind_address,
            sse_path: sse_path.to_string(),
            post_path: "/message".to_string(),
            ct: Default::default(),
            sse_keep_alive: Some(Duration::from_secs(15)),
        };

        // Create a factory function
        let factory = self.create_service_factory();

        // Start the SSE server
        tracing::info!("Starting SSE server...");
        let server = rmcp::transport::sse_server::SseServer::serve_with_config(server_config)
            .await
            .context("Failed to start SSE server")?;

        // Register our service with the server
        server.with_service(factory);

        tracing::info!(
            "Successfully started SSE server on {} at path {} with message path /message",
            bind_address,
            sse_path
        );
        Ok(())
    }

    /// Start the Streamable HTTP server
    ///
    /// This method starts a Streamable HTTP server on the specified address.
    /// The server will handle Streamable HTTP connections from clients
    /// and route tool calls to the appropriate upstream servers.
    ///
    /// # Arguments
    /// * `bind_address` - The socket address to bind the server to
    /// * `path` - The path for the Streamable HTTP endpoint (e.g., "/mcp")
    ///
    /// # Returns
    /// * `Result<()>` - Ok if the server started successfully, Err otherwise
    pub async fn start_streamable_http(&self, bind_address: SocketAddr, path: &str) -> Result<()> {
        // For Streamable HTTP, we use the specified path
        tracing::info!(
            "Configuring Streamable HTTP server on {} at path {}",
            bind_address,
            path
        );

        // Create a factory function
        let factory = self.create_service_factory();

        // Create Streamable HTTP server config
        let server_config =
            rmcp::transport::streamable_http_server::axum::StreamableHttpServerConfig {
                bind: bind_address,
                path: path.to_string(),
                ct: Default::default(),
                sse_keep_alive: Some(Duration::from_secs(15)),
            };

        // Start the Streamable HTTP server
        tracing::info!("Starting Streamable HTTP server...");
        let server =
            rmcp::transport::streamable_http_server::axum::StreamableHttpServer::serve_with_config(
                server_config,
            )
            .await
            .context("Failed to start Streamable HTTP server")?;

        // Register our service with the server
        server.with_service(factory);

        tracing::info!(
            "Successfully started Streamable HTTP server on {} at path {}",
            bind_address,
            path
        );
        Ok(())
    }

    /// Start the proxy server with both Streamable HTTP and SSE support
    ///
    /// This method starts a unified HTTP server that supports both Streamable HTTP and SSE protocols
    /// on the same port. It uses the following endpoints:
    /// - `/mcp` - Streamable HTTP endpoint (2025-03-26 MCP specification)
    /// - `/sse` - SSE endpoint (2024-11-05 MCP specification)
    /// - `/message` - SSE message endpoint (2024-11-05 MCP specification)
    ///
    /// This is the recommended way to start the server, as it provides maximum compatibility
    /// with different client implementations.
    ///
    /// # Arguments
    /// * `bind_address` - The socket address to bind the server to
    ///
    /// # Returns
    /// * `Result<()>` - Ok if the server started successfully, Err otherwise
    pub async fn start_unified(&self, bind_address: SocketAddr) -> Result<()> {
        tracing::info!(
            "Starting unified HTTP server on {} with both Streamable HTTP and SSE support",
            bind_address
        );

        // Import the UnifiedHttpServer
        use crate::http::unified::{UnifiedHttpServer, UnifiedHttpServerConfig};

        // Create unified server config with standard MCP endpoints
        let config = UnifiedHttpServerConfig {
            bind_address,
            streamable_http_path: "/mcp".to_string(), // 2025-03-26 spec endpoint
            sse_path: "/sse".to_string(),             // 2024-11-05 spec endpoint
            sse_message_path: "/message".to_string(), // 2024-11-05 spec endpoint
            keep_alive_interval: Some(Duration::from_secs(15)),
            cancellation_token: Default::default(),
        };

        // Create a factory function
        let factory = self.create_service_factory();

        // Create and start the unified server
        let server = UnifiedHttpServer::with_config(config);
        server
            .start(factory)
            .await
            .context("Failed to start unified HTTP server")?;

        tracing::info!(
            "Successfully started unified HTTP server on {} with endpoints /mcp, /sse, and /message",
            bind_address
        );
        Ok(())
    }
}

impl ServerHandler for HttpProxyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCPMate Proxy Server that aggregates tools from multiple MCP servers".into(),
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _: Option<rmcp::model::PaginatedRequestParam>,
        _: RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        // Get tools with smart prefixing
        let all_tools = get_all_tools_with_smart_prefix(&self.connection_pool).await;

        // Filter disabled tools if database is available
        let tools = if let Some(db) = &self.db {
            let mut filtered_tools = Vec::new();

            for tool in all_tools {
                // Parse the tool name to extract server prefix if present
                let (server_prefix, original_tool_name) = parse_tool_name(&tool.name);

                // Get the server name (either from prefix or from the tool name mapping)
                let server_name = if let Some(prefix) = server_prefix {
                    prefix.to_string()
                } else {
                    // If no prefix, try to get the server name from the tool name mapping
                    let tool_name_mapping = self.get_tool_name_mapping().await;
                    if let Some(mapping) = tool_name_mapping.get(&tool.name.to_string()) {
                        mapping.server_name.clone()
                    } else {
                        // If we can't determine the server, include the tool by default
                        filtered_tools.push(tool);
                        continue;
                    }
                };

                // Check if the tool is enabled
                match crate::conf::operations::is_tool_enabled(
                    &db.pool,
                    &server_name,
                    &original_tool_name,
                )
                .await
                {
                    Ok(enabled) => {
                        if enabled {
                            filtered_tools.push(tool);
                        } else {
                            tracing::debug!(
                                "Filtering out disabled tool '{}' from server '{}'",
                                original_tool_name,
                                server_name
                            );
                        }
                    }
                    Err(e) => {
                        // Log the error but include the tool by default
                        tracing::warn!(
                            "Error checking if tool '{}' is enabled: {}. Including by default.",
                            original_tool_name,
                            e
                        );
                        filtered_tools.push(tool);
                    }
                }
            }

            filtered_tools
        } else {
            // If no database, return all tools
            all_tools
        };

        tracing::info!("Returning {} aggregated tools to client", tools.len());

        Ok(rmcp::model::ListToolsResult {
            next_cursor: None,
            tools,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Extract the tool name and arguments
        let tool_name = request.name.clone();
        let arguments = request.arguments.clone();
        let tool_name_str = tool_name.to_string();

        // Get the tool name mapping
        let tool_name_mapping = self.get_tool_name_mapping().await;

        // Look up the tool in the mapping
        if let Some(mapping) = tool_name_mapping.get(&tool_name_str) {
            // We found the tool in our mapping
            tracing::info!(
                "Found tool '{}' in mapping -> server: '{}', upstream: '{}'",
                tool_name_str,
                mapping.server_name,
                mapping.upstream_tool_name
            );

            // Check if the tool is enabled if database is available
            if let Some(db) = &self.db {
                // Parse the tool name to extract original name
                let (_, original_tool_name) = parse_tool_name(&mapping.upstream_tool_name);

                // Check if the tool is enabled
                match crate::conf::operations::is_tool_enabled(
                    &db.pool,
                    &mapping.server_name,
                    &original_tool_name,
                )
                .await
                {
                    Ok(enabled) => {
                        if !enabled {
                            return Err(McpError::invalid_params(
                                format!("Tool '{}' is disabled", tool_name_str),
                                None,
                            ));
                        }
                    }
                    Err(e) => {
                        // Log the error but allow the tool call to proceed
                        tracing::warn!(
                            "Error checking if tool '{}' is enabled: {}. Allowing by default.",
                            original_tool_name,
                            e
                        );
                    }
                }
            }

            // Lock the connection pool to access the service
            let mut pool = self.connection_pool.lock().await;

            // Get the instance
            let conn_result = pool.get_instance_mut(&mapping.server_name, &mapping.instance_id);

            match conn_result {
                Ok(conn) => {
                    // Check if the connection is ready
                    if !conn.is_connected() {
                        return Err(McpError::internal_error(
                            format!(
                                "Server '{}' instance '{}' is not connected",
                                mapping.server_name, mapping.instance_id
                            ),
                            None,
                        ));
                    }

                    // Check if the service is available
                    if conn.service.is_none() {
                        return Err(McpError::internal_error(
                            format!(
                                "Service for server '{}' instance '{}' is not available",
                                mapping.server_name, mapping.instance_id
                            ),
                            None,
                        ));
                    }

                    // Mark the connection as busy
                    conn.update_busy();

                    // Prepare the request with the upstream tool name
                    let upstream_request = CallToolRequestParam {
                        name: mapping.upstream_tool_name.clone().into(),
                        arguments: arguments.clone(),
                    };

                    tracing::info!(
                        "Calling upstream tool '{}' on server '{}' instance '{}'",
                        mapping.upstream_tool_name,
                        mapping.server_name,
                        mapping.instance_id
                    );

                    // Call the tool on the upstream server
                    let result = match conn
                        .service
                        .as_mut()
                        .unwrap()
                        .call_tool(upstream_request)
                        .await
                    {
                        Ok(result) => {
                            // Mark the connection as ready again
                            conn.status = crate::core::ConnectionStatus::Ready;
                            Ok(result)
                        }
                        Err(e) => {
                            // Mark the connection as ready again
                            conn.status = crate::core::ConnectionStatus::Ready;

                            // Handle different types of errors
                            use rmcp::ServiceError;
                            let error_message = match e {
                                ServiceError::McpError(mcp_err) => {
                                    // This is already a McpError, so we can just pass it through
                                    tracing::error!(
                                        "MCP error calling tool '{}' on server '{}' instance '{}': {}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
                                        mcp_err
                                    );
                                    return Err(mcp_err);
                                }
                                ServiceError::Transport(io_err) => {
                                    // Transport error (network, IO)
                                    tracing::error!(
                                        "Transport error calling tool '{}' on server '{}' instance '{}': {}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
                                        io_err
                                    );

                                    // Update connection status to error
                                    conn.update_failed(format!("Transport error: {}", io_err));

                                    format!("Network or IO error: {}", io_err)
                                }
                                ServiceError::UnexpectedResponse => {
                                    // Unexpected response type
                                    tracing::error!(
                                        "Unexpected response type from tool '{}' on server '{}' instance '{}'",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id
                                    );
                                    "Unexpected response type from upstream server".to_string()
                                }
                                ServiceError::Cancelled { reason } => {
                                    // Request was cancelled
                                    let reason_str = reason.as_deref().unwrap_or("<unknown>");
                                    tracing::error!(
                                        "Request cancelled for tool '{}' on server '{}' instance '{}': {}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
                                        reason_str
                                    );
                                    format!("Request cancelled: {}", reason_str)
                                }
                                ServiceError::Timeout { timeout } => {
                                    // Request timed out
                                    tracing::error!(
                                        "Request timeout for tool '{}' on server '{}' instance '{}' after {:?}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
                                        timeout
                                    );
                                    format!("Request timed out after {:?}", timeout)
                                }
                                // Handle any future error types that might be added
                                _ => {
                                    tracing::error!(
                                        "Unknown error calling tool '{}' on server '{}' instance '{}': {:?}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
                                        e
                                    );
                                    format!("Unknown error: {:?}", e)
                                }
                            };

                            Err(McpError::internal_error(
                                format!(
                                    "Error calling tool '{}': {}",
                                    tool_name_str, error_message
                                ),
                                None,
                            ))
                        }
                    };

                    result
                }
                Err(e) => {
                    tracing::error!("Error getting instance: {}", e);
                    Err(McpError::internal_error(
                        format!("Error getting instance: {}", e),
                        None,
                    ))
                }
            }
        } else {
            // Tool not found in mapping, try the old way as fallback
            tracing::warn!(
                "Tool '{}' not found in mapping, trying fallback method",
                tool_name_str
            );

            // Try to parse the tool name to extract server prefix if present
            let (server_prefix, original_tool_name) = parse_tool_name(&tool_name_str);

            // Call the upstream tool
            match crate::core::tool::call_upstream_tool(
                &self.connection_pool,
                CallToolRequestParam {
                    name: tool_name_str.clone().into(),
                    arguments,
                },
            )
            .await
            {
                Ok(result) => Ok(result),
                Err(e) => {
                    tracing::error!("Error calling tool '{}': {}", tool_name_str, e);

                    // Provide a more helpful error message if we have a server prefix
                    if let Some(server_prefix) = server_prefix {
                        Err(McpError::invalid_params(
                            format!(
                                "Error calling tool '{}' on server '{}': {}",
                                original_tool_name, server_prefix, e
                            ),
                            None,
                        ))
                    } else {
                        Err(McpError::invalid_params(
                            format!(
                                "Tool '{}' not found or error occurred: {}",
                                tool_name_str, e
                            ),
                            None,
                        ))
                    }
                }
            }
        }
    }
}
