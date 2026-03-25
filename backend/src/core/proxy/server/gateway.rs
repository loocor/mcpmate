use super::common::{ClientContext, ManagedClientContextResolver, SessionBoundClientContextResolver};
use crate::{
    config::database::Database,
    core::{pool::UpstreamConnectionPool, transport::TransportType},
    mcper::builtin::BuiltinServiceRegistry,
};
use anyhow::Context;
use once_cell::sync::OnceCell;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult, InitializeRequestParams,
    ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, ReadResourceRequestParams,
    ReadResourceResult, RequestId, ResourceUpdatedNotificationParam, ServerInfo, SubscribeRequestParams,
    UnsubscribeRequestParams,
};
use rmcp::{ServerHandler, service::RequestContext};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

static GLOBAL_PROXY_SERVER: OnceCell<Arc<Mutex<ProxyServer>>> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct DownstreamRoute {
    pub session_id: String,
    pub client_id: String,
    pub rules_fingerprint: Option<String>,
    pub peer: rmcp::service::Peer<rmcp::RoleServer>,
}

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
    pub client_context_resolver: Arc<SessionBoundClientContextResolver>,
    pub downstream_clients: Arc<dashmap::DashMap<String, rmcp::service::Peer<rmcp::RoleServer>>>,
    pub resource_subscriptions: Arc<dashmap::DashMap<(String, String), String>>, // (session_id, unique_uri) -> server_id
    pub server_resource_index: Arc<dashmap::DashMap<String, dashmap::DashSet<(String, String)>>>, // server_id -> {(session_id, unique_uri)}
    pub call_sessions_by_token: Arc<dashmap::DashMap<rmcp::model::ProgressToken, DownstreamRoute>>,
    pub call_sessions_by_request: Arc<dashmap::DashMap<RequestId, DownstreamRoute>>,
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
            client_context_resolver: self.client_context_resolver.clone(),
            downstream_clients: self.downstream_clients.clone(),
            resource_subscriptions: self.resource_subscriptions.clone(),
            server_resource_index: self.server_resource_index.clone(),
            call_sessions_by_token: self.call_sessions_by_token.clone(),
            call_sessions_by_request: self.call_sessions_by_request.clone(),
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

    fn is_streamable_http(
        &self,
        context: &RequestContext<rmcp::RoleServer>,
    ) -> bool {
        context.extensions.get::<axum::http::request::Parts>().is_some()
    }

    fn map_client_context_error(
        &self,
        error: anyhow::Error,
    ) -> rmcp::ErrorData {
        tracing::warn!(error = %error, "Managed client context resolution failed");
        rmcp::ErrorData::invalid_request(error.to_string(), None)
    }

    pub async fn resolve_initialize_client_context(
        &self,
        context: &RequestContext<rmcp::RoleServer>,
        initialize: &InitializeRequestParams,
    ) -> Result<ClientContext, rmcp::ErrorData> {
        let client = self.client_context_resolver
            .resolve_initialize_context(initialize, context)
            .await
            .map_err(|error| self.map_client_context_error(error))?;
        self.attach_runtime_identity(client).await
    }

    pub async fn resolve_bound_client_context(
        &self,
        context: &RequestContext<rmcp::RoleServer>,
    ) -> Result<ClientContext, rmcp::ErrorData> {
        let client = self
            .client_context_resolver
            .resolve_request_context(context)
            .await
            .map_err(|error| self.map_client_context_error(error))?;
        let client = self.attach_runtime_identity(client).await?;

        if let Some(session_id) = client.session_id.clone() {
            if !self.downstream_clients.contains_key(&session_id) {
                self.register_downstream_client(&client, context.peer.clone()).await?;
            }
        }

        Ok(client)
    }

    fn require_session_id(
        &self,
        client: &ClientContext,
    ) -> Result<String, rmcp::ErrorData> {
        client.session_id.clone().ok_or_else(|| {
            rmcp::ErrorData::invalid_request(
                "Managed downstream session is required for this request".to_string(),
                None,
            )
        })
    }

    pub fn build_downstream_route(
        &self,
        client: &ClientContext,
        peer: rmcp::service::Peer<rmcp::RoleServer>,
    ) -> Result<DownstreamRoute, rmcp::ErrorData> {
        Ok(DownstreamRoute {
            session_id: self.require_session_id(client)?,
            client_id: client.client_id.clone(),
            rules_fingerprint: client.rules_fingerprint.clone(),
            peer,
        })
    }

    async fn attach_runtime_identity(
        &self,
        mut client: ClientContext,
    ) -> Result<ClientContext, rmcp::ErrorData> {
        if client.rules_fingerprint.is_some() {
            return Ok(client);
        }

        let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
            self.database.clone(),
            self.profile_service.clone(),
        );
        let snapshot = vis
            .resolve_snapshot(&client.client_id)
            .await
            .map_err(|error| self.map_client_context_error(error))?;
        client.rules_fingerprint = Some(snapshot.rules_fingerprint);
        Ok(client)
    }

    pub async fn register_downstream_client(
        &self,
        client: &ClientContext,
        peer: rmcp::service::Peer<rmcp::RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
        let session_id = self.require_session_id(client)?;
        self.client_context_resolver
            .bind_session(&session_id, client)
            .await
            .map_err(|error| self.map_client_context_error(error))?;
        self.downstream_clients.insert(session_id.clone(), peer);

        tracing::debug!(
            session_id = %session_id,
            client_id = %client.client_id,
            profile_id = ?client.profile_id,
            source = ?client.source,
            transport = ?client.transport,
            total_clients = %self.downstream_clients.len(),
            "downstream client registered"
        );
        Ok(())
    }

    pub async fn remove_downstream_session(
        &self,
        session_id: &str,
    ) {
        self.downstream_clients.remove(session_id);

        let subscription_keys: Vec<((String, String), String)> = self
            .resource_subscriptions
            .iter()
            .filter(|entry| entry.key().0 == session_id)
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        for ((subscription_session, unique_uri), server_id) in subscription_keys {
            self.resource_subscriptions
                .remove(&(subscription_session.clone(), unique_uri.clone()));
            if !server_id.is_empty() {
                if let Some(index) = self.server_resource_index.get(&server_id) {
                    index.remove(&(subscription_session, unique_uri));
                }
            }
        }

        let progress_tokens: Vec<rmcp::model::ProgressToken> = self
            .call_sessions_by_token
            .iter()
            .filter(|entry| entry.value().session_id == session_id)
            .map(|entry| entry.key().clone())
            .collect();
        for progress_token in progress_tokens {
            self.call_sessions_by_token.remove(&progress_token);
        }

        let request_ids: Vec<RequestId> = self
            .call_sessions_by_request
            .iter()
            .filter(|entry| entry.value().session_id == session_id)
            .map(|entry| entry.key().clone())
            .collect();
        for request_id in request_ids {
            self.call_sessions_by_request.remove(&request_id);
        }

        if let Err(error) = self.client_context_resolver.unbind_session(session_id).await {
            tracing::warn!(session_id = %session_id, error = %error, "Failed to unbind downstream session");
        }
    }

    fn allowed_origin(origin: &str) -> bool {
        crate::common::env::is_allowed_origin(origin)
    }

    fn enforce_origin_if_present(
        &self,
        context: &RequestContext<rmcp::RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
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

    fn enforce_mcp_protocol_header(
        &self,
        context: &RequestContext<rmcp::RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
        if !self.is_streamable_http(context) {
            return Ok(());
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
            client_context_resolver: Arc::new(SessionBoundClientContextResolver::new()),
            downstream_clients: Arc::new(dashmap::DashMap::new()),
            resource_subscriptions: Arc::new(dashmap::DashMap::new()),
            server_resource_index: Arc::new(dashmap::DashMap::new()),
            call_sessions_by_token: Arc::new(dashmap::DashMap::new()),
            call_sessions_by_request: Arc::new(dashmap::DashMap::new()),
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
            keep_alive_interval: Some(std::time::Duration::from_secs(15)),
            cancellation_token: self.cancellation_token.clone(),
        };
        let server = super::common::UnifiedHttpServer::with_config(config);
        let server_handle = tokio::spawn(async move { server.start(factory).await });
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerTransportReady {
            transport_type: TransportType::StreamableHttp,
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
            sse_retry: Some(std::time::Duration::from_secs(3)),
            stateful_mode: true,
            json_response: false,
            cancellation_token: self.cancellation_token.clone(),
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

    pub async fn notify_tool_list_changed(&self) -> usize {
        self.broadcast_notify(|peer| Box::pin(async move { peer.notify_tool_list_changed().await }))
            .await
    }

    pub async fn notify_prompt_list_changed(&self) -> usize {
        self.broadcast_notify(|peer| Box::pin(async move { peer.notify_prompt_list_changed().await }))
            .await
    }

    pub async fn notify_resource_list_changed(&self) -> usize {
        self.broadcast_notify(|peer| Box::pin(async move { peer.notify_resource_list_changed().await }))
            .await
    }

    pub async fn notify_all_list_changed(&self) -> (usize, usize, usize) {
        let t = self.notify_tool_list_changed().await;
        let p = self.notify_prompt_list_changed().await;
        let r = self.notify_resource_list_changed().await;
        (t, p, r)
    }

    async fn notify_resource_updated_for_session(
        &self,
        session_id: &str,
        uri: &str,
    ) -> bool {
        let Some(peer_ref) = self.downstream_clients.get(session_id) else {
            return false;
        };
        let peer = peer_ref.clone();
        drop(peer_ref);
        let param = ResourceUpdatedNotificationParam {
            uri: uri.to_string(),
        };
        match peer.notify_resource_updated(param).await {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(session_id = %session_id, uri = %uri, error = %error, "notify resources/updated failed, removing stale session");
                self.remove_downstream_session(session_id).await;
                false
            }
        }
    }

    pub async fn notify_resource_updated(
        &self,
        uri: &str,
    ) -> usize {
        let routes: Vec<String> = self
            .resource_subscriptions
            .iter()
            .filter(|entry| entry.key().1 == uri)
            .map(|entry| entry.key().0.clone())
            .collect();
        let mut ok = 0usize;
        for session_id in routes {
            if self.notify_resource_updated_for_session(&session_id, uri).await {
                ok += 1;
            }
        }
        ok
    }

    pub async fn notify_resource_updates_for_server(
        &self,
        server_id: &str,
    ) -> usize {
        let Some(subscriptions) = self.server_resource_index.get(server_id) else {
            return 0;
        };

        let routes: Vec<(String, String)> = subscriptions.iter().map(|entry| entry.key().clone()).collect();
        drop(subscriptions);

        let mut total = 0usize;
        for (session_id, uri) in routes {
            if self.notify_resource_updated_for_session(&session_id, &uri).await {
                total += 1;
            }
        }
        total
    }

    async fn broadcast_notify<F, Fut>(
        &self,
        make_call: F,
    ) -> usize
    where
        F: Fn(rmcp::service::Peer<rmcp::RoleServer>) -> Fut,
        Fut: std::future::Future<Output = Result<(), rmcp::service::ServiceError>>,
    {
        let recipients: Vec<(String, rmcp::service::Peer<rmcp::RoleServer>)> = self
            .downstream_clients
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();

        let mut ok = 0usize;
        let mut stale_sessions: Vec<String> = Vec::new();
        for (session_id, peer) in recipients {
            match make_call(peer).await {
                Ok(()) => ok += 1,
                Err(error) => {
                    tracing::warn!(session_id = %session_id, error = %error, "notify downstream failed, marking stale session");
                    stale_sessions.push(session_id);
                }
            }
        }
        for session_id in stale_sessions {
            self.remove_downstream_session(&session_id).await;
        }
        ok
    }

    pub fn register_call_session(
        &self,
        progress_token: rmcp::model::ProgressToken,
        request_id: RequestId,
        route: DownstreamRoute,
    ) {
        tracing::debug!(
            progress_token = ?progress_token,
            request_id = ?request_id,
            session_id = %route.session_id,
            client_id = %route.client_id,
            "Registered call session for downstream mapping"
        );
        self.call_sessions_by_token.insert(progress_token, route.clone());
        self.call_sessions_by_request.insert(request_id, route);
    }

    pub fn unregister_call_session(
        &self,
        progress_token: &rmcp::model::ProgressToken,
        request_id: &RequestId,
    ) {
        self.call_sessions_by_token.remove(progress_token);
        self.call_sessions_by_request.remove(request_id);
    }

    pub async fn forward_upstream_progress(
        &self,
        _server_id: &str,
        param: rmcp::model::ProgressNotificationParam,
        _meta_token: Option<rmcp::model::ProgressToken>,
    ) -> bool {
        let Some(route_ref) = self.call_sessions_by_token.get(&param.progress_token) else {
            return false;
        };
        let route = route_ref.clone();
        drop(route_ref);

        tracing::trace!(
            progress_token = ?param.progress_token,
            session_id = %route.session_id,
            client_id = %route.client_id,
            progress = ?param.progress,
            "Forwarded progress to downstream"
        );
        match route.peer.notify_progress(param.clone()).await {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(session_id = %route.session_id, client_id = %route.client_id, error = %error, "Failed to forward progress; removing stale session");
                self.call_sessions_by_token.remove(&param.progress_token);
                self.remove_downstream_session(&route.session_id).await;
                false
            }
        }
    }

    pub async fn forward_upstream_cancelled(
        &self,
        _server_id: &str,
        param: rmcp::model::CancelledNotificationParam,
    ) -> bool {
        let Some(route_ref) = self.call_sessions_by_request.get(&param.request_id) else {
            return false;
        };
        let route = route_ref.clone();
        drop(route_ref);

        tracing::trace!(
            request_id = ?param.request_id,
            session_id = %route.session_id,
            client_id = %route.client_id,
            reason = ?param.reason,
            "Forwarded cancellation to downstream"
        );
        match route.peer.notify_cancelled(param.clone()).await {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(session_id = %route.session_id, client_id = %route.client_id, error = %error, "Failed to forward cancellation; removing stale session");
                self.call_sessions_by_request.remove(&param.request_id);
                self.remove_downstream_session(&route.session_id).await;
                false
            }
        }
    }

    pub async fn forward_upstream_log(
        &self,
        _server_id: &str,
        param: rmcp::model::LoggingMessageNotificationParam,
        meta_token: Option<rmcp::model::ProgressToken>,
    ) -> bool {
        let Some(token) = meta_token else {
            return false;
        };
        let Some(route_ref) = self.call_sessions_by_token.get(&token) else {
            return false;
        };
        let route = route_ref.clone();
        drop(route_ref);

        tracing::trace!(
            progress_token = ?token,
            session_id = %route.session_id,
            client_id = %route.client_id,
            level = ?param.level,
            "Forwarded log message to downstream"
        );
        match route.peer.notify_logging_message(param.clone()).await {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(session_id = %route.session_id, client_id = %route.client_id, error = %error, "Failed to forward log message; removing stale session");
                self.call_sessions_by_token.remove(&token);
                self.remove_downstream_session(&route.session_id).await;
                false
            }
        }
    }
}

impl ServerHandler for ProxyServer {
    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ServerInfo, rmcp::ErrorData> {
        self.enforce_origin_if_present(&context)?;
        tracing::info!(
            client_protocol = %request.protocol_version,
            has_roots = %request.capabilities.roots.is_some(),
            has_sampling = %request.capabilities.sampling.is_some(),
            has_elicitation = %request.capabilities.elicitation.is_some(),
            client_name = %request.client_info.name,
            client_version = %request.client_info.version,
            "MCP client initialize"
        );

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

        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request.clone());
        }

        let client = self.resolve_initialize_client_context(&context, &request).await?;
        if client.session_id.is_some() {
            self.register_downstream_client(&client, context.peer.clone()).await?;
        }

        Ok(self.get_info())
    }

    async fn subscribe(
        &self,
        request: SubscribeRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let client = self.resolve_bound_client_context(&_context).await?;
        let session_id = self.require_session_id(&client)?;
        let unique_uri = request.uri;
        let server_id_opt = if let Ok((server_name, _)) = crate::core::capability::naming::resolve_unique_name(
            crate::core::capability::naming::NamingKind::Resource,
            &unique_uri,
        )
        .await
        {
            crate::core::capability::resolver::to_id(&server_name)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        if let Some(server_id) = server_id_opt {
            self.resource_subscriptions
                .insert((session_id.clone(), unique_uri.clone()), server_id.clone());
            let entry = self.server_resource_index.entry(server_id.clone()).or_default();
            entry.insert((session_id.clone(), unique_uri.clone()));
            tracing::info!(
                server_id = %server_id,
                session_id = %session_id,
                client_id = %client.client_id,
                uri = %unique_uri,
                "Subscribed resource updates"
            );
        } else {
            self.resource_subscriptions
                .insert((session_id.clone(), unique_uri.clone()), String::new());
            tracing::warn!(
                session_id = %session_id,
                client_id = %client.client_id,
                uri = %unique_uri,
                "Subscribed without resolvable server id"
            );
        }
        Ok(())
    }

    async fn unsubscribe(
        &self,
        request: UnsubscribeRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<(), rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let client = self.resolve_bound_client_context(&_context).await?;
        let session_id = self.require_session_id(&client)?;
        let unique_uri = request.uri;
        if let Some((_, server_id)) = self
            .resource_subscriptions
            .remove(&(session_id.clone(), unique_uri.clone()))
        {
            if !server_id.is_empty() {
                if let Some(set) = self.server_resource_index.get(&server_id) {
                    set.remove(&(session_id.clone(), unique_uri.clone()));
                }
            }
            tracing::info!(
                server_id = %server_id,
                session_id = %session_id,
                client_id = %client.client_id,
                uri = %unique_uri,
                "Unsubscribed resource updates"
            );
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
        ServerInfo::new(capabilities)
            .with_protocol_version(rmcp::model::ProtocolVersion::LATEST)
            .with_server_info(crate::common::constants::branding::create_implementation())
            .with_instructions(crate::common::constants::branding::DESCRIPTION.to_string())
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::tools::list_tools(self, _request, _context).await
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::tools::call_tool(self, request, _context).await
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::resources::list_resources(self, _request, _context).await
    }

    async fn list_resource_templates(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourceTemplatesResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::resources::list_resource_templates(self, _request, _context).await
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::resources::read_resource(self, request, _context).await
    }

    async fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListPromptsResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::prompts::list_prompts(self, _request, _context).await
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<GetPromptResult, rmcp::ErrorData> {
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        super::prompts::get_prompt(self, request, _context).await
    }
}
