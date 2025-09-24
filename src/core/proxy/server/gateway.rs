use crate::{
    config::database::Database,
    core::{pool::UpstreamConnectionPool, transport::TransportType},
    mcper::builtin::BuiltinServiceRegistry,
};
use anyhow::Context;
use once_cell::sync::OnceCell;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, GetPromptRequestParam, GetPromptResult, InitializeRequestParam,
    ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, ReadResourceRequestParam,
    ReadResourceResult, ResourceUpdatedNotificationParam, ServerInfo, SubscribeRequestParam, UnsubscribeRequestParam,
};
use rmcp::{ServerHandler, service::RequestContext};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

static GLOBAL_PROXY_SERVER: OnceCell<Arc<Mutex<ProxyServer>>> = OnceCell::new();

#[derive(Debug)]
pub struct ProxyServer {
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    pub database: Option<Arc<Database>>,
    pub profile_service: Option<Arc<crate::core::profile::ProfileService>>,
    pub runtime_cache: Arc<crate::runtime::RuntimeCache>,
    pub redb_cache: Arc<crate::core::cache::RedbCacheManager>,
    pub paginator: crate::core::foundation::pagination::ProxyPaginator,
    pub builtin_services: Arc<BuiltinServiceRegistry>,
    pub cancellation_token: tokio_util::sync::CancellationToken,
    pub downstream_clients: Arc<dashmap::DashMap<String, rmcp::service::Peer<rmcp::RoleServer>>>,
    pub resource_subscriptions: Arc<dashmap::DashMap<String, String>>, // unique_uri -> server_id
    pub server_resource_index: Arc<dashmap::DashMap<String, dashmap::DashSet<String>>>, // server_id -> {unique_uri}
}

impl Clone for ProxyServer {
    fn clone(&self) -> Self {
        Self {
            connection_pool: self.connection_pool.clone(),
            database: self.database.clone(),
            profile_service: self.profile_service.clone(),
            runtime_cache: self.runtime_cache.clone(),
            redb_cache: self.redb_cache.clone(),
            paginator: self.paginator.clone(),
            builtin_services: self.builtin_services.clone(),
            cancellation_token: self.cancellation_token.clone(),
            downstream_clients: self.downstream_clients.clone(),
            resource_subscriptions: self.resource_subscriptions.clone(),
            server_resource_index: self.server_resource_index.clone(),
        }
    }
}

impl ProxyServer {
    pub fn set_global(server: Arc<Mutex<ProxyServer>>) {
        let _ = GLOBAL_PROXY_SERVER.set(server);
    }
    pub fn global() -> Option<Arc<Mutex<ProxyServer>>> {
        GLOBAL_PROXY_SERVER.get().cloned()
    }

    fn is_streamable_http(&self, context: &RequestContext<rmcp::RoleServer>) -> bool {
        context.extensions.get::<axum::http::request::Parts>().is_some()
    }

    fn allowed_origin(origin: &str) -> bool {
        // TODO(PR4-followup): When deploying MCPMate remotely (cloud/VM/K8s),
        // replace this hardcoded loopback allowlist with a configurable policy:
        // - sources: env (e.g., MCPMATE_ALLOWED_ORIGINS), DB, or admin API
        // - semantics: exact match, wildcard, or regex; default-deny
        // - integration: shared CORS/Origin guard reused by API and /mcp
        // For now we keep a minimal, safe-by-default loopback allowlist.
        let o = origin.trim().to_ascii_lowercase();
        o == "null"
            || o.starts_with("http://localhost")
            || o.starts_with("https://localhost")
            || o.starts_with("http://127.0.0.1")
            || o.starts_with("https://127.0.0.1")
            || o.starts_with("http://[::1]")
            || o.starts_with("https://[::1]")
    }

    fn enforce_origin_if_present(&self, context: &RequestContext<rmcp::RoleServer>) -> Result<(), rmcp::ErrorData> {
        if let Some(parts) = context.extensions.get::<axum::http::request::Parts>() {
            if let Some(val) = parts.headers.get(axum::http::header::ORIGIN) {
                if let Ok(s) = val.to_str() {
                    if !Self::allowed_origin(s) {
                        tracing::warn!(origin = %s, "Rejected request due to disallowed Origin");
                        return Err(rmcp::ErrorData::invalid_request(
                            format!("Disallowed Origin: {}", s),
                            None,
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn enforce_mcp_protocol_header(&self, context: &RequestContext<rmcp::RoleServer>) -> Result<(), rmcp::ErrorData> {
        if !self.is_streamable_http(context) {
            return Ok(()); // SSE/stdio path: no HTTP parts
        }
        let parts = match context.extensions.get::<axum::http::request::Parts>() {
            Some(p) => p,
            None => return Ok(()),
        };
        let required = [
            rmcp::model::ProtocolVersion::V_2025_06_18.to_string(),
            rmcp::model::ProtocolVersion::V_2025_03_26.to_string(),
        ];
        match parts.headers.get("MCP-Protocol-Version").and_then(|h| h.to_str().ok()) {
            Some(v) if required.iter().any(|r| r == v) => Ok(()),
            Some(v) => Err(rmcp::ErrorData::invalid_request(
                format!("Unsupported MCP-Protocol-Version: {}", v),
                None,
            )),
            None => Err(rmcp::ErrorData::invalid_request(
                "Missing MCP-Protocol-Version header".to_string(),
                None,
            )),
        }
    }

    pub fn new(config: Arc<crate::core::models::Config>) -> Self {
        let mut pool = UpstreamConnectionPool::new(config.clone(), None);
        pool.initialize();
        let connection_pool = Arc::new(Mutex::new(pool));
        UpstreamConnectionPool::start_health_check(connection_pool.clone());

        let paginator = crate::core::foundation::pagination::ProxyPaginator::new();
        let builtin_services = Arc::new(BuiltinServiceRegistry::new());
        let redb_cache = crate::core::cache::RedbCacheManager::global().unwrap_or_else(|e| {
            tracing::error!("Failed to initialize REDB cache manager: {}", e);
            panic!("REDB cache manager is required for ProxyServer")
        });

        Self {
            connection_pool,
            database: None,
            profile_service: None,
            runtime_cache: Arc::new(crate::runtime::RuntimeCache::new()),
            redb_cache,
            paginator,
            builtin_services,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            downstream_clients: Arc::new(dashmap::DashMap::new()),
            resource_subscriptions: Arc::new(dashmap::DashMap::new()),
            server_resource_index: Arc::new(dashmap::DashMap::new()),
        }
    }

    pub async fn set_database(
        &mut self,
        db: Database,
    ) -> anyhow::Result<()> {
        let db_arc = Arc::new(db);
        self.database = Some(db_arc.clone());
        crate::core::capability::naming::initialize(db_arc.pool.clone());
        self.profile_service = Some(Arc::new(crate::core::profile::ProfileService::new(db_arc.clone())));
        self.builtin_services =
            Arc::new(BuiltinServiceRegistry::new().with_mcpmate_services(db_arc.clone(), self.connection_pool.clone()));
        if let Err(e) = crate::core::capability::resolver::init(db_arc.clone()).await {
            tracing::warn!("Failed to initialize global resolver: {}", e);
        } else {
            tracing::info!("Global server resolver initialized");
        }
        // server_mapping manager removed; resolver provides in-memory mapping
        {
            let mut pool = self.connection_pool.lock().await;
            pool.set_database(Some(db_arc));
            pool.set_runtime_cache(Some(self.runtime_cache.clone()));
        }
        self.setup_event_handlers().await?;
        tracing::debug!(
            "Database connection, builtin services, server manager, and event handlers set for proxy server"
        );
        Ok(())
    }

    async fn setup_event_handlers(&self) -> anyhow::Result<()> {
        let mut handlers = crate::core::events::EventHandlers::new();
        if let Some(profile_service) = &self.profile_service {
            handlers.set_profile_service(profile_service.clone());
        }
        handlers.set_connection_pool(self.connection_pool.clone());
        if let Some(database) = &self.database {
            let event_capability_manager = Arc::new(crate::core::events::EventDrivenCapabilityManager::new(
                Arc::new(database.pool.clone()),
                self.connection_pool.clone(),
            ));
            handlers.set_event_capability_manager(event_capability_manager);
        } else {
            tracing::warn!("No database available for event-driven capability manager in event handlers");
        }
        crate::core::events::init_with_handlers(handlers)?;
        tracing::info!("Event handlers initialized with direct integration");
        Ok(())
    }

    pub async fn start(
        &self,
        transport_type: TransportType,
        bind_address: SocketAddr,
    ) -> anyhow::Result<()> {
        tracing::info!("Starting proxy server with transport type: {:?}", transport_type);
        match transport_type {
            TransportType::Sse => self.start_sse_server(bind_address, "/sse").await,
            TransportType::StreamableHttp => self.start_streamable_http_server(bind_address, "/mcp").await,
            TransportType::Stdio => Err(anyhow::anyhow!("Stdio transport not supported for proxy server")),
        }
    }

    pub async fn start_unified(
        &self,
        bind_address: SocketAddr,
    ) -> anyhow::Result<tokio::task::JoinHandle<anyhow::Result<()>>> {
        tracing::info!("Starting unified proxy server on {}", bind_address);
        let server_clone = self.clone();
        let factory = move || server_clone.clone();
        let config = super::common::UnifiedHttpServerConfig {
            bind_address,
            streamable_http_path: "/mcp".to_string(),
            sse_path: "/sse".to_string(),
            sse_message_path: "/message".to_string(),
            keep_alive_interval: Some(std::time::Duration::from_secs(15)),
            cancellation_token: self.cancellation_token.clone(),
        };
        let server = super::common::UnifiedHttpServer::with_config(config);
        let server_handle = tokio::spawn(async move { server.start(factory).await });
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

    pub async fn initiate_shutdown(&self) -> anyhow::Result<()> {
        self.cancellation_token.cancel();
        Ok(())
    }
    pub async fn complete_shutdown(&self) -> anyhow::Result<()> {
        let mut pool = self.connection_pool.lock().await;
        pool.disconnect_all().await?;
        Ok(())
    }

    async fn start_sse_server(
        &self,
        bind_address: SocketAddr,
        sse_path: &str,
    ) -> anyhow::Result<()> {
        tracing::info!("Starting SSE server on {} at path {}", bind_address, sse_path);
        let server_config = rmcp::transport::sse_server::SseServerConfig {
            bind: bind_address,
            sse_path: sse_path.to_string(),
            post_path: "/message".to_string(),
            ct: tokio_util::sync::CancellationToken::new(),
            sse_keep_alive: Some(std::time::Duration::from_secs(15)),
        };
        let server_clone = self.clone();
        let factory = move || server_clone.clone();
        let server = rmcp::transport::sse_server::SseServer::serve_with_config(server_config)
            .await
            .context("Failed to start SSE server")?;
        server.with_service(factory);
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerTransportReady {
            transport_type: TransportType::Sse,
            ready: true,
        });
        tracing::info!("SSE server started successfully");
        Ok(())
    }

    async fn start_streamable_http_server(
        &self,
        bind_address: SocketAddr,
        path: &str,
    ) -> anyhow::Result<()> {
        tracing::info!("Starting Streamable HTTP server on {} at path {}", bind_address, path);
        let server_clone = self.clone();
        let factory = move || Ok(server_clone.clone());
        let server_config = rmcp::transport::StreamableHttpServerConfig {
            sse_keep_alive: Some(std::time::Duration::from_secs(15)),
            stateful_mode: true,
        };
        let session_manager = std::sync::Arc::new(
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default(),
        );
        let streamable_service = rmcp::transport::StreamableHttpService::new(factory, session_manager, server_config);
        let app = axum::Router::new().route_service(path, streamable_service);
        let listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .context("Failed to bind Streamable HTTP server")?;
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("Streamable HTTP server error: {}", e);
            }
        });
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerTransportReady {
            transport_type: TransportType::StreamableHttp,
            ready: true,
        });
        tracing::info!("Streamable HTTP server started successfully");
        Ok(())
    }

    /// Broadcast tools listChanged to all downstream MCP clients.
    pub async fn notify_tool_list_changed(&self) -> usize {
        self.broadcast_notify(|peer| Box::pin(async move { peer.notify_tool_list_changed().await }))
            .await
    }

    /// Broadcast prompts listChanged to all downstream MCP clients.
    pub async fn notify_prompt_list_changed(&self) -> usize {
        self.broadcast_notify(|peer| Box::pin(async move { peer.notify_prompt_list_changed().await }))
            .await
    }

    /// Broadcast resources listChanged to all downstream MCP clients.
    pub async fn notify_resource_list_changed(&self) -> usize {
        self.broadcast_notify(|peer| Box::pin(async move { peer.notify_resource_list_changed().await }))
            .await
    }

    /// Notify tools/prompts/resources listChanged and return counts per type (t, p, r)
    pub async fn notify_all_list_changed(&self) -> (usize, usize, usize) {
        let t = self.notify_tool_list_changed().await;
        let p = self.notify_prompt_list_changed().await;
        let r = self.notify_resource_list_changed().await;
        (t, p, r)
    }

    /// Notify downstream clients that a specific resource URI was updated.
    pub async fn notify_resource_updated(&self, uri: &str) -> usize {
        let uri = uri.to_string();
        let mut ok = 0usize;
        let mut stale: Vec<String> = Vec::new();
        for entry in self.downstream_clients.iter() {
            let key = entry.key().clone();
            let peer = entry.value().clone();
            let param = ResourceUpdatedNotificationParam { uri: uri.clone() };
            match peer.notify_resource_updated(param).await {
                Ok(()) => ok += 1,
                Err(e) => {
                    tracing::warn!(client = %key, error = %e, "notify resources/updated failed, marking stale");
                    stale.push(key);
                }
            }
        }
        for k in stale { let _ = self.downstream_clients.remove(&k); }
        ok
    }

    /// For a given server, notify resources/updated for all subscribed unique URIs.
    pub async fn notify_resource_updates_for_server(&self, server_id: &str) -> usize {
        if let Some(set) = self.server_resource_index.get(server_id) {
            let mut total = 0usize;
            for uri in set.iter() {
                total += self.notify_resource_updated(uri.key()).await;
            }
            total
        } else { 0 }
    }

    async fn broadcast_notify<F, Fut>(
        &self,
        make_call: F,
    ) -> usize
    where
        F: Fn(rmcp::service::Peer<rmcp::RoleServer>) -> Fut,
        Fut: std::future::Future<Output = Result<(), rmcp::service::ServiceError>>,
    {
        let mut ok = 0usize;
        let mut stale: Vec<String> = Vec::new();
        for entry in self.downstream_clients.iter() {
            let key = entry.key().clone();
            let peer = entry.value().clone();
            match make_call(peer).await {
                Ok(()) => {
                    ok += 1;
                }
                Err(e) => {
                    tracing::warn!(client = %key, error = %e, "notify downstream failed, marking stale");
                    stale.push(key);
                }
            }
        }
        // clean up stale peers
        for k in stale {
            let _ = self.downstream_clients.remove(&k);
        }
        ok
    }
}

impl ServerHandler for ProxyServer {
    async fn initialize(
        &self,
        request: InitializeRequestParam,
        context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ServerInfo, rmcp::ErrorData> {
        // Origin check (if header present)
        self.enforce_origin_if_present(&context)?;
        // Log client-declared protocol version and capabilities
        tracing::info!(
            client_protocol = %request.protocol_version,
            has_roots = %request.capabilities.roots.is_some(),
            has_sampling = %request.capabilities.sampling.is_some(),
            has_elicitation = %request.capabilities.elicitation.is_some(),
            client_name = %request.client_info.name,
            client_version = %request.client_info.version,
            "MCP client initialize"
        );

        // Best-effort: also log raw HTTP headers if available (Streamable HTTP)
        if let Some(parts) = context.extensions.get::<axum::http::request::Parts>() {
            if let Some(v) = parts.headers.get("MCP-Protocol-Version").and_then(|h| h.to_str().ok()) {
                tracing::debug!(header_mcp_protocol_version = %v, "HTTP header: MCP-Protocol-Version");
            }
            if let Some(v) = parts
                .headers
                .get(axum::http::header::ORIGIN)
                .and_then(|h| h.to_str().ok())
            {
                tracing::debug!(header_origin = %v, "HTTP header: Origin");
            }
        }

        // Preserve peer info for later use (keeps default behavior)
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request);
        }

        // Register downstream peer for notifications
        let client_id = crate::generate_id!("dcli");
        self.downstream_clients.insert(client_id.clone(), context.peer.clone());
        tracing::debug!(client_id = %client_id, total_clients = %self.downstream_clients.len(), "downstream client registered");

        Ok(self.get_info())
    }

    async fn subscribe(
        &self,
        request: SubscribeRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let unique_uri = request.uri;
        let server_id_opt = if let Ok((server_name, _)) =
            crate::core::capability::naming::resolve_unique_name(
                crate::core::capability::naming::NamingKind::Resource,
                &unique_uri,
            )
            .await
        {
            crate::core::capability::resolver::to_id(&server_name).await.ok().flatten()
        } else { None };
        if let Some(server_id) = server_id_opt {
            self.resource_subscriptions.insert(unique_uri.clone(), server_id.clone());
            let entry = self
                .server_resource_index
                .entry(server_id.clone())
                .or_default();
            entry.insert(unique_uri.clone());
            tracing::info!(server_id = %server_id, uri = %unique_uri, "Subscribed resource updates");
        } else {
            // still accept to be tolerant; updates may be broadcast-only
            self.resource_subscriptions.insert(unique_uri.clone(), String::new());
            tracing::warn!(uri = %unique_uri, "Subscribed without resolvable server id");
        }
        Ok(())
    }

    async fn unsubscribe(
        &self,
        request: UnsubscribeRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let unique_uri = request.uri;
        if let Some((_, server_id)) = self.resource_subscriptions.remove(&unique_uri) {
            if let Some(set) = self.server_resource_index.get(&server_id) {
                set.remove(&unique_uri);
            }
            tracing::info!(server_id = %server_id, uri = %unique_uri, "Unsubscribed resource updates");
        }
        Ok(())
    }

    fn get_info(&self) -> ServerInfo {
        let capabilities = rmcp::model::ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .enable_prompts()
            .enable_tool_list_changed()
            .enable_prompts_list_changed()
            .enable_resources_list_changed()
            .enable_resources_subscribe()
            .build();
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::LATEST,
            capabilities,
            server_info: crate::common::constants::branding::create_implementation(),
            instructions: Some(crate::common::constants::branding::DESCRIPTION.to_string()),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::tools::list_tools(self, _request, _context)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::tools::call_tool(self, request, _context)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::resources::list_resources(self, _request, _context)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourceTemplatesResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::resources::list_resource_templates(self, _request, _context)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::resources::read_resource(self, request, _context)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    async fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListPromptsResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::prompts::list_prompts(self, _request, _context)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<GetPromptResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::prompts::get_prompt(self, request, _context)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }
}
