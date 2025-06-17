//! Independent ProxyServer implementation for core
//!
//! This module provides a complete reimplementation of the proxy server functionality
//! using only core modules, with zero dependencies on core modules.

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use rmcp::{
    Error as McpError, RoleServer, ServerHandler, Service,
    model::{
        CallToolRequestParam, CallToolResult, GetPromptRequestParam, GetPromptResult,
        ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParam,
        ReadResourceRequestParam, ReadResourceResult, ServerInfo,
    },
    service::RequestContext,
    tool,
};
use tokio::sync::Mutex;
use tracing;

use crate::{
    config::database::Database,
    core::{pool::UpstreamConnectionPool, transport::TransportType},
};

/// Global instance of the proxy server
static GLOBAL_PROXY_SERVER: OnceCell<Arc<Mutex<ProxyServer>>> = OnceCell::new();

/// Independent Proxy Server implementation using core modules
///
/// This server aggregates tools, resources, and prompts from multiple MCP servers
/// and exposes them through various transport protocols.
#[derive(Debug, Clone)]
pub struct ProxyServer {
    /// Connection pool for upstream servers (using core implementation)
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    /// Database connection for configuration persistence
    pub database: Option<Arc<Database>>,
    /// Suit service for configuration management and tool enablement check
    pub suit_service: Option<Arc<crate::core::suit::SuitService>>,
    /// Runtime cache for fast runtime queries (temporary core dependency)
    pub runtime_cache: Arc<crate::runtime::RuntimeCache>,
    /// Paginator for aggregated results
    pub paginator: crate::core::foundation::pagination::ProxyPaginator,
}

/// Configuration for the unified HTTP server
#[derive(Debug, Clone)]
pub struct UnifiedHttpServerConfig {
    /// Address to bind the server to
    pub bind_address: SocketAddr,
    /// Path for the Streamable HTTP endpoint
    pub streamable_http_path: String,
    /// Path for the SSE endpoint
    pub sse_path: String,
    /// Path for the SSE message endpoint
    pub sse_message_path: String,
    /// Keep-alive interval for SSE connections
    pub keep_alive_interval: Option<std::time::Duration>,
    /// Cancellation token for graceful shutdown
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

impl Default for UnifiedHttpServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8000".parse().unwrap(),
            streamable_http_path: "/mcp".to_string(),
            sse_path: "/sse".to_string(),
            sse_message_path: "/message".to_string(),
            keep_alive_interval: Some(std::time::Duration::from_secs(15)),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        }
    }
}

/// Unified HTTP server that supports both Streamable HTTP and SSE
pub struct UnifiedHttpServer {
    /// Server configuration
    pub config: UnifiedHttpServerConfig,
}

impl Default for UnifiedHttpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedHttpServer {
    /// Create a new unified HTTP server with default configuration
    pub fn new() -> Self {
        Self::with_config(UnifiedHttpServerConfig::default())
    }

    /// Create a new unified HTTP server with custom configuration
    pub fn with_config(config: UnifiedHttpServerConfig) -> Self {
        Self { config }
    }

    /// Start the unified HTTP server with both Streamable HTTP and SSE endpoints
    pub async fn start<F, S>(
        &self,
        service_factory: F,
    ) -> Result<()>
    where
        F: Fn() -> S + Clone + Send + Sync + 'static,
        S: Service<RoleServer> + Send + Sync + 'static,
    {
        tracing::info!(
            "Starting unified HTTP server on {} with Streamable HTTP at {} and SSE at {}",
            self.config.bind_address,
            self.config.streamable_http_path,
            self.config.sse_path
        );

        // Create Streamable HTTP server config
        let streamable_http_config = rmcp::transport::StreamableHttpServerConfig {
            sse_keep_alive: self.config.keep_alive_interval,
            stateful_mode: true,
        };

        // Create SSE server config
        let sse_config = rmcp::transport::sse_server::SseServerConfig {
            bind: self.config.bind_address,
            sse_path: self.config.sse_path.clone(),
            post_path: self.config.sse_message_path.clone(),
            ct: self.config.cancellation_token.clone(),
            sse_keep_alive: self.config.keep_alive_interval,
        };

        // Create the StreamableHttpService
        let session_manager = std::sync::Arc::new(
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
        );

        let streamable_http_service = rmcp::transport::StreamableHttpService::new(
            service_factory.clone(),
            session_manager,
            streamable_http_config,
        );

        // Create SSE server
        let (sse_server, sse_router) = rmcp::transport::sse_server::SseServer::new(sse_config);

        // Create the combined router
        let combined_router = axum::Router::new()
            .route_service(&self.config.streamable_http_path, streamable_http_service)
            .merge(sse_router);

        // Start the combined server
        let listener = tokio::net::TcpListener::bind(self.config.bind_address)
            .await
            .context(format!(
                "Failed to bind to address {}",
                self.config.bind_address
            ))?;

        let ct = self.config.cancellation_token.child_token();

        // Start the HTTP server with the combined router
        let server = axum::serve(listener, combined_router).with_graceful_shutdown(async move {
            ct.cancelled().await;
            tracing::info!("Unified HTTP server cancelled");
        });

        // Register the service with SSE server
        tracing::info!("Registering service with SSE server");
        sse_server.with_service(service_factory);

        // Start the server in a background task
        tokio::spawn(async move {
            if let Err(e) = server.await {
                tracing::error!(error = %e, "Unified HTTP server shutdown with error");
            }
        });

        tracing::info!("Unified HTTP server started successfully with the following endpoints:");
        tracing::info!(
            "  - Streamable HTTP: {}{}",
            self.config.bind_address,
            self.config.streamable_http_path
        );
        tracing::info!(
            "  - SSE: {}{}",
            self.config.bind_address,
            self.config.sse_path
        );
        tracing::info!(
            "  - SSE Message: {}{}",
            self.config.bind_address,
            self.config.sse_message_path
        );

        Ok(())
    }
}

#[tool(tool_box)]
impl ProxyServer {
    /// Set the global instance of the proxy server
    pub fn set_global(server: Arc<Mutex<ProxyServer>>) {
        if GLOBAL_PROXY_SERVER.set(server).is_err() {
            tracing::warn!("Global proxy server instance already set, ignoring");
        } else {
            tracing::info!("Global proxy server instance set");
        }
    }

    /// Get the global instance of the proxy server
    pub fn global() -> Option<Arc<Mutex<ProxyServer>>> {
        GLOBAL_PROXY_SERVER.get().cloned()
    }

    /// Create a new proxy server
    pub fn new(config: Arc<crate::core::models::Config>) -> Self {
        // Create connection pool with no database reference initially
        let mut pool = UpstreamConnectionPool::new(config.clone(), None);

        // Initialize the pool
        pool.initialize();

        let connection_pool = Arc::new(Mutex::new(pool));

        // Start health check task
        UpstreamConnectionPool::start_health_check(connection_pool.clone());

        // Create paginator with default config
        let paginator = crate::core::foundation::pagination::ProxyPaginator::new();

        Self {
            connection_pool,
            database: None,     // Database will be initialized separately
            suit_service: None, // Will be initialized when database is set
            runtime_cache: Arc::new(crate::runtime::RuntimeCache::new()),
            paginator,
        }
    }

    /// Set the database connection
    pub async fn set_database(
        &mut self,
        db: Database,
    ) -> Result<()> {
        // Create Arc for the database
        let db_arc = Arc::new(db);

        // Store the database connection
        self.database = Some(db_arc.clone());

        // Initialize Suit service
        self.suit_service = Some(Arc::new(crate::core::suit::SuitService::new(
            db_arc.clone(),
        )));

        // Update connection pool with database reference and runtime cache
        {
            let mut pool = self.connection_pool.lock().await;
            pool.set_database(Some(db_arc));
            pool.set_runtime_cache(Some(self.runtime_cache.clone()));
        }

        tracing::info!(
            "Database connection and runtime cache set for proxy server and connection pool"
        );
        Ok(())
    }

    /// Start the proxy server with the specified transport type
    pub async fn start(
        &self,
        transport_type: TransportType,
        bind_address: SocketAddr,
    ) -> Result<()> {
        tracing::info!(
            "Starting proxy server with transport type: {:?}",
            transport_type
        );

        match transport_type {
            TransportType::Sse => self.start_sse_server(bind_address, "/sse").await,
            TransportType::StreamableHttp => {
                self.start_streamable_http_server(bind_address, "/mcp")
                    .await
            }
            TransportType::Stdio => Err(anyhow::anyhow!(
                "Stdio transport not supported for proxy server"
            )),
        }
    }

    /// Start the proxy server with unified transport (both SSE and Streamable HTTP)
    pub async fn start_unified(
        &self,
        bind_address: SocketAddr,
    ) -> Result<()> {
        tracing::info!("Starting unified proxy server on {}", bind_address);

        // Create a service factory function
        let server_clone = self.clone();
        let factory = move || server_clone.clone();

        // Create unified server config
        let config = UnifiedHttpServerConfig {
            bind_address,
            streamable_http_path: "/mcp".to_string(),
            sse_path: "/sse".to_string(),
            sse_message_path: "/message".to_string(),
            keep_alive_interval: Some(std::time::Duration::from_secs(15)),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        };

        // Create and start the unified server
        let server = UnifiedHttpServer::with_config(config);
        server.start(factory).await?;

        // Publish server ready events
        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ServerTransportReady {
                transport_type: TransportType::StreamableHttp,
                ready: true,
            },
        );

        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ServerTransportReady {
                transport_type: TransportType::Sse,
                ready: true,
            },
        );

        tracing::info!("Unified proxy server started successfully");
        Ok(())
    }

    /// Start SSE server
    async fn start_sse_server(
        &self,
        bind_address: SocketAddr,
        sse_path: &str,
    ) -> Result<()> {
        tracing::info!(
            "Starting SSE server on {} at path {}",
            bind_address,
            sse_path
        );

        // Create SSE server config
        let server_config = rmcp::transport::sse_server::SseServerConfig {
            bind: bind_address,
            sse_path: sse_path.to_string(),
            post_path: "/message".to_string(),
            ct: tokio_util::sync::CancellationToken::new(),
            sse_keep_alive: Some(std::time::Duration::from_secs(15)),
        };

        // Create a factory function
        let server_clone = self.clone();
        let factory = move || server_clone.clone();

        // Start the SSE server
        let server = rmcp::transport::sse_server::SseServer::serve_with_config(server_config)
            .await
            .context("Failed to start SSE server")?;

        // Register our service with the server
        server.with_service(factory);

        // Publish server ready event
        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ServerTransportReady {
                transport_type: TransportType::Sse,
                ready: true,
            },
        );

        tracing::info!("SSE server started successfully");
        Ok(())
    }

    /// Start Streamable HTTP server
    async fn start_streamable_http_server(
        &self,
        bind_address: SocketAddr,
        path: &str,
    ) -> Result<()> {
        tracing::info!(
            "Starting Streamable HTTP server on {} at path {}",
            bind_address,
            path
        );

        // Create a factory function
        let server_clone = self.clone();
        let factory = move || server_clone.clone();

        // Create Streamable HTTP server config
        let server_config = rmcp::transport::StreamableHttpServerConfig {
            sse_keep_alive: Some(std::time::Duration::from_secs(15)),
            stateful_mode: true,
        };

        // Create the StreamableHttpService
        let session_manager = std::sync::Arc::new(
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
        );

        let streamable_service =
            rmcp::transport::StreamableHttpService::new(factory, session_manager, server_config);

        // Create an Axum router and mount the service
        let app = axum::Router::new().route_service(path, streamable_service);

        // Start the server
        let listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .context("Failed to bind Streamable HTTP server")?;

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("Streamable HTTP server error: {}", e);
            }
        });

        // Publish server ready event
        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ServerTransportReady {
                transport_type: TransportType::StreamableHttp,
                ready: true,
            },
        );

        tracing::info!("Streamable HTTP server started successfully");
        Ok(())
    }
}

impl ServerHandler for ProxyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::LATEST,
            capabilities: rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            server_info: rmcp::model::Implementation {
                name: "mcpmate".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "MCPMate - Aggregates tools, resources, and prompts from multiple upstream MCP servers. Connect to access all configured MCP services through a single endpoint.".to_string()
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        tracing::debug!("Listing tools from proxy server");

        // Use core protocol implementation
        let tools = crate::core::protocol::get_all_tools(&self.connection_pool).await;

        Ok(rmcp::model::ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        tracing::debug!("Calling tool: {}", request.name);

        // Check if suit service is available
        if let Some(suit_service) = &self.suit_service {
            // Use core protocol implementation
            match crate::core::protocol::tool::call_upstream_tool(
                &self.connection_pool,
                request,
                suit_service,
            )
            .await
            {
                Ok(result) => Ok(result),
                Err(e) => {
                    tracing::error!("Failed to call tool: {}", e);
                    Err(McpError::internal_error(e.to_string(), None))
                }
            }
        } else {
            tracing::error!("Suit service not available for tool calling");
            Err(McpError::internal_error(
                "Configuration suit service not available".to_string(),
                None,
            ))
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        tracing::debug!("Listing resources from proxy server");

        // Use core protocol implementation
        let resources = crate::core::protocol::get_all_resources(&self.connection_pool).await;

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        tracing::debug!("Listing resource templates from proxy server");

        // Use core protocol implementation
        let resource_templates =
            crate::core::protocol::get_all_resource_templates(&self.connection_pool).await;

        Ok(ListResourceTemplatesResult {
            resource_templates,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        tracing::debug!("Reading resource: {}", request.uri);

        // Build resource mapping on-demand (can be optimized later with caching)
        let resource_mapping = crate::core::protocol::resource::build_resource_mapping(
            &self.connection_pool,
            self.database.as_ref(),
        )
        .await;

        // Use core protocol implementation
        match crate::core::protocol::resource::read_upstream_resource(
            &self.connection_pool,
            &resource_mapping,
            &request.uri,
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::error!("Failed to read resource '{}': {}", request.uri, e);
                Err(McpError::internal_error(e.to_string(), None))
            }
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        tracing::debug!("Listing prompts from proxy server");

        // Use core protocol implementation
        let prompts = crate::core::protocol::get_all_prompts(&self.connection_pool).await;

        Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<GetPromptResult, McpError> {
        tracing::debug!("Getting prompt: {}", request.name);

        // Build prompt mapping on-demand (can be optimized later with caching)
        let prompt_mapping =
            crate::core::protocol::prompt::build_prompt_mapping(&self.connection_pool).await;

        // Use core protocol implementation
        match crate::core::protocol::prompt::get_upstream_prompt(
            &self.connection_pool,
            &prompt_mapping,
            &request.name,
            request.arguments,
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::error!("Failed to get prompt '{}': {}", request.name, e);
                Err(McpError::internal_error(e.to_string(), None))
            }
        }
    }
}
