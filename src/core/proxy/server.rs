//! Independent ProxyServer implementation for core
//!
//! This module provides a complete reimplementation of the proxy server functionality
//! using only core modules, with zero dependencies on core modules.

use crate::{
    config::database::Database,
    core::{pool::UpstreamConnectionPool, transport::TransportType},
    mcper::builtin::BuiltinServiceRegistry,
};
use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, Service,
    model::{
        CallToolRequestParam, CallToolResult, GetPromptRequestParam, GetPromptResult, ListPromptsResult,
        ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParam, ReadResourceRequestParam,
        ReadResourceResult, ServerInfo,
    },
    service::RequestContext,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tracing;

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
    /// Profile service for configuration management and tool enablement check
    pub profile_service: Option<Arc<crate::core::profile::ProfileService>>,
    /// Runtime cache for fast runtime queries (temporary core dependency)
    pub runtime_cache: Arc<crate::runtime::RuntimeCache>,
    /// Paginator for aggregated results
    pub paginator: crate::core::foundation::pagination::ProxyPaginator,
    /// Built-in services registry for MCPMate internal tools
    pub builtin_services: Arc<BuiltinServiceRegistry>,
    /// Cancellation token for graceful shutdown
    pub cancellation_token: tokio_util::sync::CancellationToken,
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
        use crate::common::constants::ports;
        Self {
            bind_address: format!("127.0.0.1:{}", ports::MCP_PORT).parse().unwrap(),
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

        let service_factory_clone = service_factory.clone();
        let streamable_http_service = rmcp::transport::StreamableHttpService::new(
            move || Ok(service_factory_clone()),
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
            .context(format!("Failed to bind to address {}", self.config.bind_address))?;

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
        tracing::info!("  - SSE: {}{}", self.config.bind_address, self.config.sse_path);
        tracing::info!(
            "  - SSE Message: {}{}",
            self.config.bind_address,
            self.config.sse_message_path
        );

        Ok(())
    }
}

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

        // Create builtin services registry (will be initialized when database is set)
        let builtin_services = Arc::new(BuiltinServiceRegistry::new());

        Self {
            connection_pool,
            database: None,        // Database will be initialized separately
            profile_service: None, // Will be initialized when database is set
            runtime_cache: Arc::new(crate::runtime::RuntimeCache::new()),
            paginator,
            builtin_services,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
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

        // Initialize Profile service
        self.profile_service = Some(Arc::new(crate::core::profile::ProfileService::new(db_arc.clone())));

        // Initialize builtin services registry with MCPMate services
        self.builtin_services =
            Arc::new(BuiltinServiceRegistry::new().with_mcpmate_services(db_arc.clone(), self.connection_pool.clone()));

        // Initialize global resolver with database
        if let Err(e) = crate::core::protocol::resolver::init(db_arc.clone()) {
            tracing::warn!("Failed to initialize global resolver: {}", e);
        } else {
            tracing::info!("Global server resolver initialized");
        }

        // Update connection pool with database reference and runtime cache
        {
            let mut pool = self.connection_pool.lock().await;
            pool.set_database(Some(db_arc));
            pool.set_runtime_cache(Some(self.runtime_cache.clone()));
        }

        // Setup event handlers with server manager callback
        self.setup_event_handlers().await?;

        tracing::debug!(
            "Database connection, builtin services, server manager, and event handlers set for proxy server"
        );
        Ok(())
    }

    /// Setup event handlers with simplified direct integration
    async fn setup_event_handlers(&self) -> Result<()> {
        let mut handlers = crate::core::events::EventHandlers::new();

        // Set profile service for cache invalidation
        if let Some(profile_service) = &self.profile_service {
            handlers.set_profile_service(profile_service.clone());
        }

        // Set connection pool for server management
        handlers.set_connection_pool(self.connection_pool.clone());

        // Set lightweight event-driven capability manager for server capability sync
        if let Some(database) = &self.database {
            let event_capability_manager = Arc::new(crate::core::events::EventDrivenCapabilityManager::new(
                Arc::new(database.pool.clone()),
                self.connection_pool.clone(),
            ));

            handlers.set_event_capability_manager(event_capability_manager);
        } else {
            tracing::warn!("No database available for event-driven capability manager in event handlers");
        }

        // Initialize the event handlers
        crate::core::events::init_with_handlers(handlers)?;

        tracing::info!("Event handlers initialized with direct integration");
        Ok(())
    }

    /// Start the proxy server with the specified transport type
    pub async fn start(
        &self,
        transport_type: TransportType,
        bind_address: SocketAddr,
    ) -> Result<()> {
        tracing::info!("Starting proxy server with transport type: {:?}", transport_type);

        match transport_type {
            TransportType::Sse => self.start_sse_server(bind_address, "/sse").await,
            TransportType::StreamableHttp => self.start_streamable_http_server(bind_address, "/mcp").await,
            TransportType::Stdio => Err(anyhow::anyhow!("Stdio transport not supported for proxy server")),
        }
    }

    /// Start the proxy server with unified transport (both SSE and Streamable HTTP)
    /// Returns a JoinHandle that can be awaited for graceful shutdown
    pub async fn start_unified(
        &self,
        bind_address: SocketAddr,
    ) -> Result<tokio::task::JoinHandle<Result<(), anyhow::Error>>> {
        tracing::info!("Starting unified proxy server on {}", bind_address);

        // Create a service factory function
        let server_clone = self.clone();
        let factory = move || server_clone.clone();

        // Create unified server config using proxy server's cancellation token
        let config = UnifiedHttpServerConfig {
            bind_address,
            streamable_http_path: "/mcp".to_string(),
            sse_path: "/sse".to_string(),
            sse_message_path: "/message".to_string(),
            keep_alive_interval: Some(std::time::Duration::from_secs(15)),
            cancellation_token: self.cancellation_token.clone(),
        };

        // Create and start the unified server
        let server = UnifiedHttpServer::with_config(config);
        let server_handle = tokio::spawn(async move { server.start(factory).await });

        // Publish server ready events
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerTransportReady {
            transport_type: TransportType::StreamableHttp,
            ready: true,
        });

        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerTransportReady {
            transport_type: TransportType::Sse,
            ready: true,
        });

        tracing::info!("Unified proxy server started successfully");
        Ok(server_handle)
    }

    /// Initiate graceful shutdown of the proxy server
    /// This sends the shutdown signal but doesn't wait for completion
    pub async fn initiate_shutdown(&self) -> Result<()> {
        tracing::info!("Initiating proxy server shutdown...");

        // Cancel the server's main operations (this will trigger graceful shutdown)
        self.cancellation_token.cancel();

        tracing::info!("Shutdown signal sent to proxy server");
        Ok(())
    }

    /// Complete the shutdown process by cleaning up connections
    pub async fn complete_shutdown(&self) -> Result<()> {
        tracing::info!("Completing proxy server shutdown...");

        // Disconnect all connections in the pool
        {
            let mut pool = self.connection_pool.lock().await;
            pool.disconnect_all().await?;
        }

        tracing::info!("Proxy server shutdown completed");
        Ok(())
    }

    /// Start SSE server
    async fn start_sse_server(
        &self,
        bind_address: SocketAddr,
        sse_path: &str,
    ) -> Result<()> {
        tracing::info!("Starting SSE server on {} at path {}", bind_address, sse_path);

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
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerTransportReady {
            transport_type: TransportType::Sse,
            ready: true,
        });

        tracing::info!("SSE server started successfully");
        Ok(())
    }

    /// Start Streamable HTTP server
    async fn start_streamable_http_server(
        &self,
        bind_address: SocketAddr,
        path: &str,
    ) -> Result<()> {
        tracing::info!("Starting Streamable HTTP server on {} at path {}", bind_address, path);

        // Create a factory function
        let server_clone = self.clone();
        let factory = move || Ok(server_clone.clone());

        // Create Streamable HTTP server config
        let server_config = rmcp::transport::StreamableHttpServerConfig {
            sse_keep_alive: Some(std::time::Duration::from_secs(15)),
            stateful_mode: true,
        };

        // Create the StreamableHttpService
        let session_manager = std::sync::Arc::new(
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
        );

        let streamable_service = rmcp::transport::StreamableHttpService::new(factory, session_manager, server_config);

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
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerTransportReady {
            transport_type: TransportType::StreamableHttp,
            ready: true,
        });

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
                title: Some("MCPMate".to_string()),
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
        // Early return if no database available
        let Some(db) = &self.database else {
            tracing::warn!("Database not available for tool filtering; returning only builtin tools");
            let builtin_tools = self.builtin_services.tools();
            return Ok(rmcp::model::ListToolsResult {
                tools: builtin_tools,
                next_cursor: None,
            });
        };

        // Get enabled tools from database configuration
        let db_service = crate::core::protocol::DatabaseToolService::new(db.clone(), self.connection_pool.clone());
        let mut tools = match db_service.get_enabled_tools().await {
            Ok(tools) => {
                tracing::info!("Proxy listed {} tools from database", tools.len());
                tools
            }
            Err(e) => {
                tracing::error!("Failed to get tools using database service: {}", e);
                // Return empty list on error - no fallback to runtime aggregation
                Vec::new()
            }
        };

        // Always include builtin service tools (these are system tools, not user-configurable)
        let builtin_tools = self.builtin_services.tools();
        tracing::debug!("Including {} builtin service tools", builtin_tools.len());
        tools.extend(builtin_tools);

        tracing::info!("Proxy listed {} total tools (including builtin services)", tools.len());

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

        // First check if this is a builtin service tool
        if let Some(result) = self.builtin_services.call_tool(&request).await {
            tracing::debug!("Tool '{}' handled by builtin service", request.name);
            return match result {
                Ok(call_result) => Ok(call_result),
                Err(e) => {
                    tracing::error!("Builtin service tool '{}' failed: {}", request.name, e);
                    Err(McpError::internal_error(e.to_string(), None))
                }
            };
        }

        // If not a builtin tool, use database service for authoritative tool calling
        match &self.database {
            Some(db) => {
                // Create database tool service
                let db_service =
                    crate::core::protocol::DatabaseToolService::new(db.clone(), self.connection_pool.clone());

                // Use new database-driven tool calling
                match crate::core::protocol::call_upstream_tool(&db_service, request).await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        tracing::error!("Failed to call tool using database service: {}", e);
                        Err(McpError::internal_error(e.to_string(), None))
                    }
                }
            }
            None => {
                tracing::error!("Database not available for tool calling");
                Err(McpError::internal_error(
                    "Database not available for tool calling".to_string(),
                    None,
                ))
            }
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        // Early return if no database available
        let Some(db) = &self.database else {
            tracing::warn!("Database not available for resource filtering; returning empty list");
            return Ok(ListResourcesResult {
                resources: Vec::new(),
                next_cursor: None,
            });
        };

        // Get enabled resources from database configuration
        let db_service =
            crate::core::protocol::resource::DatabaseResourceService::new(db.clone(), self.connection_pool.clone());
        let resources = match db_service.get_enabled_resources().await {
            Ok(resources) => {
                tracing::info!("Proxy listed {} total resources from database", resources.len());
                resources
            }
            Err(e) => {
                tracing::error!("Failed to get resources using database service: {}", e);
                // Return empty list on error - no fallback to runtime aggregation
                Vec::new()
            }
        };

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
        // Early return if no database available
        let Some(db) = &self.database else {
            tracing::warn!("Database not available for server filtering; returning empty list");
            return Ok(ListResourceTemplatesResult {
                resource_templates: Vec::new(),
                next_cursor: None,
            });
        };

        // Get resource templates from enabled servers only
        let db_service =
            crate::core::protocol::resource::DatabaseResourceService::new(db.clone(), self.connection_pool.clone());
        let resource_templates = match db_service.get_enabled_resource_templates().await {
            Ok(templates) => {
                tracing::info!(
                    "Proxy listed {} resource templates from enabled servers",
                    templates.len()
                );
                templates
            }
            Err(e) => {
                tracing::error!("Failed to get resource templates using database service: {}", e);
                // Return empty list on error - no fallback to runtime aggregation
                Vec::new()
            }
        };

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
        let resource_mapping =
            crate::core::protocol::resource::build_resource_mapping(&self.connection_pool, self.database.as_ref())
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
        // Early return if no database available
        let Some(db) = &self.database else {
            tracing::warn!("Database not available for prompt filtering; returning empty list");
            return Ok(ListPromptsResult {
                prompts: Vec::new(),
                next_cursor: None,
            });
        };

        // Get enabled prompts from database configuration
        let db_service =
            crate::core::protocol::prompt::DatabasePromptService::new(db.clone(), self.connection_pool.clone());
        let prompts = match db_service.get_enabled_prompts().await {
            Ok(prompts) => {
                tracing::info!("Proxy listed {} total prompts from database", prompts.len());
                prompts
            }
            Err(e) => {
                tracing::error!("Failed to get prompts using database service: {}", e);
                // Return empty list on error - no fallback to runtime aggregation
                Vec::new()
            }
        };

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
        let prompt_mapping = crate::core::protocol::prompt::build_prompt_mapping(&self.connection_pool).await;

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
