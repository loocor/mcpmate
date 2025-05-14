// Core implementation of the HTTP proxy server

use anyhow::{Context, Result};
use rmcp::{
    model::{CallToolRequestParam, CallToolResult, ServerInfo},
    service::RequestContext,
    tool, Error as McpError, ServerHandler,
};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    conf::Database, core::models::Config, core::TransportType, core::UpstreamConnectionPool,
};

use super::{handler, start_sse, start_streamable_http, start_unified};

/// HTTP Proxy Server that aggregates tools from multiple MCP servers
#[derive(Debug, Clone)]
pub struct HttpProxyServer {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    /// Tool name mapping cache
    pub(crate) tool_name_mapping_cache:
        Arc<Mutex<Option<HashMap<String, crate::core::tool::ToolNameMapping>>>>,
    /// Last time the tool name mapping was updated
    pub(crate) last_tool_mapping_update: Arc<Mutex<std::time::Instant>>,
    /// Last connection state hash, used to detect changes in the connection pool
    pub(crate) last_connection_state_hash: Arc<Mutex<u64>>,
    /// Database connection for tool configuration persistence
    pub database: Option<Arc<Database>>,
    /// Config Suit merge service for tool enablement check
    pub config_suit_merge_service: Option<Arc<crate::core::suit::ConfigSuitMergeService>>,
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

    /// Create a new HTTP proxy server
    pub fn new(config: Arc<Config>) -> Self {
        // Create connection pool
        let mut pool = UpstreamConnectionPool::new(config);

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
            database: None,                  // Database will be initialized separately
            config_suit_merge_service: None, // Will be initialized after database
        }
    }

    /// Initialize the database connection
    pub async fn init_database(&mut self) -> Result<()> {
        // Create database connection
        let db = Database::new().await?;

        // Initialize default values
        db.initialize_defaults().await?;

        // Create Arc for the database
        let db_arc = Arc::new(db);

        // Store the database connection
        self.database = Some(db_arc.clone());

        // Initialize Config Suit merge service
        let merge_service = Arc::new(crate::core::suit::ConfigSuitMergeService::new(db_arc));

        // Update the cache
        if let Err(e) = merge_service.update_cache().await {
            tracing::error!(
                "Failed to initialize Config Suit merge service cache: {}",
                e
            );
        } else {
            tracing::info!("Config Suit merge service cache initialized successfully");
        }

        // Store the Config Suit merge service
        let merge_service_arc = Arc::clone(&merge_service);
        self.config_suit_merge_service = Some(merge_service);

        // Start background update task
        crate::core::suit::ConfigSuitMergeService::start_background_update(merge_service_arc);

        tracing::info!("Database and Config Suit merge service initialized successfully");
        Ok(())
    }

    /// Set the database connection
    pub async fn set_database(&mut self, db: Database) -> Result<()> {
        // Initialize default values
        db.initialize_defaults().await?;

        // Create Arc for the database
        let db_arc = Arc::new(db);

        // Store the database connection
        self.database = Some(db_arc.clone());

        // Initialize Config Suit merge service
        let merge_service = Arc::new(crate::core::suit::ConfigSuitMergeService::new(db_arc));

        // Update the cache
        if let Err(e) = merge_service.update_cache().await {
            tracing::error!(
                "Failed to initialize Config Suit merge service cache: {}",
                e
            );
        } else {
            tracing::info!("Config Suit merge service cache initialized successfully");
        }

        // Store the Config Suit merge service
        let merge_service_arc = Arc::clone(&merge_service);
        self.config_suit_merge_service = Some(merge_service);

        // Start background update task
        crate::core::suit::ConfigSuitMergeService::start_background_update(merge_service_arc);

        tracing::info!("Database connection and Config Suit merge service set successfully");
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
                start_sse(self, bind_address, path)
                    .await
                    .context(format!("Failed to start SSE server on {}", bind_address))
            }
            TransportType::StreamableHttp => {
                tracing::info!(
                    "Using Streamable HTTP transport mode (2025-03-26 MCP specification)"
                );
                start_streamable_http(self, bind_address, path)
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
        start_unified(self, bind_address).await
    }
}

// Implement ServerHandler for HttpProxyServer by delegating to the server_handler module
impl ServerHandler for HttpProxyServer {
    fn get_info(&self) -> ServerInfo {
        handler::get_info(self)
    }

    async fn list_tools(
        &self,
        request: Option<rmcp::model::PaginatedRequestParam>,
        context: RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        handler::list_tools(self, request, context).await
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        handler::call_tool(self, request, context).await
    }
}
