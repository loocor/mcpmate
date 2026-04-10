use super::common::{ClientContext, ManagedClientContextResolver, SessionBoundClientContextResolver};
use crate::{
    audit::AuditService,
    clients::models::FirstContactBehavior,
    clients::service::ClientConfigService,
    config::audit_database::AuditDatabase,
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
use serde_json::{Map, Value};
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

pub struct ProxyServer {
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    pub database: Option<Arc<Database>>,
    pub audit_database: Option<Arc<AuditDatabase>>,
    pub audit_service: Option<Arc<AuditService>>,
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
    /// Used for first-contact governance on MCP `initialize` (unknown clients + policy).
    pub client_config_service: Option<Arc<ClientConfigService>>,
}

impl Clone for ProxyServer {
    fn clone(&self) -> Self {
        Self {
            connection_pool: self.connection_pool.clone(),
            database: self.database.clone(),
            audit_database: self.audit_database.clone(),
            audit_service: self.audit_service.clone(),
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
            client_config_service: self.client_config_service.clone(),
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
        let client = self
            .client_context_resolver
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
        if let Some(ref svc) = self.client_config_service {
            self.enforce_client_governance_for_initialize(svc, &client).await?;
        }

        if client.config_mode.is_none() {
            client.config_mode = Some(self.resolve_effective_config_mode(&client.client_id).await?);
        }

        if client.rules_fingerprint.is_some() {
            return Ok(client);
        }

        let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
            self.database.clone(),
            self.profile_service.clone(),
        );
        let snapshot = vis
            .resolve_snapshot_for_client(&client)
            .await
            .map_err(|error| self.map_client_context_error(error))?;
        client.rules_fingerprint = Some(snapshot.rules_fingerprint);
        Ok(client)
    }

    /// Enforce default client governance on MCP `initialize`: deny / review / allow for unknown clients.
    /// Uses JSON-RPC–style `invalid_request` / `internal_error` per MCP error mapping.
    async fn enforce_client_governance_for_initialize(
        &self,
        svc: &Arc<ClientConfigService>,
        client: &ClientContext,
    ) -> Result<(), rmcp::ErrorData> {
        let policy = svc.get_first_contact_behavior().await.map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to read client governance policy: {e}"), None)
        })?;

        let display_name = client
            .observed_client_info
            .as_ref()
            .map(|o| o.name.as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(client.client_id.as_str());

        let state_opt = svc
            .fetch_state(&client.client_id)
            .await
            .map_err(|e| rmcp::ErrorData::internal_error(format!("Failed to read client state: {e}"), None))?;

        if let Some(ref state) = state_opt {
            return match state.approval_status() {
                "approved" => Ok(()),
                "rejected" => Err(rmcp::ErrorData::invalid_request(
                    "MCPMate rejected this client identifier; connection is not allowed.".to_string(),
                    None,
                )),
                "suspended" => Err(rmcp::ErrorData::invalid_request(
                    "This client is suspended in MCPMate; connection is not allowed.".to_string(),
                    None,
                )),
                "pending" => Err(rmcp::ErrorData::invalid_request(
                    "This client is pending approval in MCPMate. Approve it in the dashboard, then reconnect."
                        .to_string(),
                    None,
                )),
                _ => Ok(()),
            };
        }

        match policy {
            FirstContactBehavior::Allow => Ok(()),
            FirstContactBehavior::Deny => Err(rmcp::ErrorData::invalid_request(
                "Unknown client identifier is denied by MCPMate policy. Register the client before connecting."
                    .to_string(),
                None,
            )),
            FirstContactBehavior::Review => {
                svc.ensure_passive_observed_row(&client.client_id, display_name, None)
                    .await
                    .map_err(|e| {
                        rmcp::ErrorData::internal_error(format!("Failed to register client for review: {e}"), None)
                    })?;
                Err(rmcp::ErrorData::invalid_request(
                    "This client is pending approval in MCPMate. Approve it in the dashboard, then reconnect."
                        .to_string(),
                    None,
                ))
            }
        }
    }

    async fn resolve_effective_config_mode(
        &self,
        client_id: &str,
    ) -> Result<String, rmcp::ErrorData> {
        let db = self
            .database
            .as_ref()
            .ok_or_else(|| rmcp::ErrorData::internal_error("Database not available".to_string(), None))?;
        let explicit_mode: Option<String> = sqlx::query_scalar("SELECT config_mode FROM client WHERE identifier = ?")
            .bind(client_id)
            .fetch_optional(&db.pool)
            .await
            .map_err(|error| self.map_client_context_error(error.into()))?;

        match explicit_mode.filter(|mode| !mode.trim().is_empty()) {
            Some(mode) => Ok(mode),
            None => crate::config::client::init::resolve_default_client_config_mode(&db.pool)
                .await
                .map_err(|error| self.map_client_context_error(error)),
        }
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

    pub async fn refresh_bound_session_runtime_identity(
        &self,
        session_id: &str,
        client_id: &str,
    ) -> Result<(), rmcp::ErrorData> {
        let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
            self.database.clone(),
            self.profile_service.clone(),
        );

        let snapshot = if let Some(binding) = self.client_context_resolver.session_bindings.get(session_id) {
            let client = ClientContext {
                client_id: binding.client_id.clone(),
                session_id: Some(session_id.to_string()),
                profile_id: binding.profile_id.clone(),
                config_mode: binding.config_mode.clone(),
                unify_workspace: binding.unify_workspace.clone(),
                rules_fingerprint: binding.rules_fingerprint.clone(),
                transport: crate::core::proxy::server::common::ClientTransport::StreamableHttp,
                source: crate::core::proxy::server::common::ClientIdentitySource::SessionBinding,
                observed_client_info: binding.observed_client_info.clone(),
            };
            vis.resolve_snapshot_for_client(&client)
                .await
                .map_err(|error| self.map_client_context_error(error))?
        } else {
            vis.resolve_snapshot(client_id, None)
                .await
                .map_err(|error| self.map_client_context_error(error))?
        };

        self.client_context_resolver
            .refresh_session_rules_fingerprint(session_id, snapshot.rules_fingerprint)
            .await
            .map_err(|error| self.map_client_context_error(error))
    }

    pub async fn update_unify_session_workspace(
        &self,
        session_id: &str,
        client_id: &str,
        workspace: crate::clients::models::ClientCapabilityConfig,
    ) -> Result<(), rmcp::ErrorData> {
        self.client_context_resolver
            .set_unify_workspace(session_id, Some(workspace))
            .await
            .map_err(|error| self.map_client_context_error(error))?;

        self.refresh_bound_session_runtime_identity(session_id, client_id).await
    }

    /// Remove all state associated with a downstream session.
    ///
    /// ## Invariants
    ///
    /// This method maintains consistency across all session-related data structures:
    ///
    /// - `downstream_clients`: The peer is removed.
    /// - `resource_subscriptions`: All entries for this session are removed.
    /// - `server_resource_index`: For each subscription, the reverse index entry is removed.
    /// - `call_sessions_by_token` / `call_sessions_by_request`: In-flight call mappings are cleared.
    /// - `session_bindings`: The client context resolver's binding is removed.
    ///
    /// ## Trigger Points
    ///
    /// Cleanup is triggered reactively when session usage fails:
    /// - `notify_resource_updated_for_session` fails to send
    /// - `broadcast_notify` fails for a session
    /// - `forward_upstream_progress` / `forward_upstream_cancelled` / `forward_upstream_log` fail
    ///
    /// The MCP protocol lacks an explicit "session closed" notification, so stale sessions
    /// are only detected when subsequent operations fail.
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

    fn protocol_version_from_context(
        &self,
        context: &RequestContext<rmcp::RoleServer>,
    ) -> Option<String> {
        context
            .extensions
            .get::<axum::http::request::Parts>()
            .and_then(|parts| parts.headers.get("MCP-Protocol-Version"))
            .and_then(|value| value.to_str().ok())
            .map(ToString::to_string)
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
            audit_database: None,
            audit_service: None,
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
            client_config_service: None,
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
        let client_config_service = Arc::new(ClientConfigService::bootstrap(Arc::new(db_arc.pool.clone())).await?);
        self.client_config_service = Some(client_config_service.clone());
        self.builtin_services = Arc::new(BuiltinServiceRegistry::new().with_mcpmate_services(
            db_arc.clone(),
            self.connection_pool.clone(),
            client_config_service,
        ));
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

    pub fn set_audit_service(
        &mut self,
        audit_database: Arc<AuditDatabase>,
        audit_service: Arc<AuditService>,
    ) {
        self.audit_database = Some(audit_database);
        self.audit_service = Some(audit_service);
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
        let server_config = rmcp::transport::StreamableHttpServerConfig::default()
            .with_sse_keep_alive(Some(std::time::Duration::from_secs(15)))
            .with_sse_retry(Some(std::time::Duration::from_secs(3)))
            .with_stateful_mode(true)
            .with_json_response(false)
            .with_cancellation_token(self.cancellation_token.clone());
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
        let param = ResourceUpdatedNotificationParam { uri: uri.to_string() };
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

    fn build_base_event_data(route: &DownstreamRoute) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert("client_id".to_string(), Value::String(route.client_id.clone()));
        data.insert("session_id".to_string(), Value::String(route.session_id.clone()));
        data
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
        let mut data = Self::build_base_event_data(&route);
        data.insert("progress".to_string(), Value::from(param.progress));
        if let Some(total) = param.total {
            data.insert("total".to_string(), Value::from(total));
        }
        if let Some(message) = param.message.clone() {
            data.insert("message".to_string(), Value::String(message));
        }
        crate::audit::interceptor::emit_event(
            self.audit_service.as_ref(),
            crate::audit::interceptor::build_mcp_event(
                crate::audit::AuditAction::NotificationProgress,
                crate::audit::AuditStatus::Success,
                None,
                None,
                None,
                None,
                Some(data),
                None,
            ),
        )
        .await;
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
        let mut data = Self::build_base_event_data(&route);
        data.insert("request_id".to_string(), Value::String(param.request_id.to_string()));
        crate::audit::interceptor::emit_event(
            self.audit_service.as_ref(),
            crate::audit::interceptor::build_mcp_event(
                crate::audit::AuditAction::NotificationCancelled,
                crate::audit::AuditStatus::Cancelled,
                None,
                None,
                None,
                None,
                Some(data),
                param.reason.clone().map(|reason| reason.to_string()),
            ),
        )
        .await;
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
        let mut data = Self::build_base_event_data(&route);
        data.insert(
            "level".to_string(),
            Value::String(
                match param.level {
                    rmcp::model::LoggingLevel::Debug => "debug",
                    rmcp::model::LoggingLevel::Info => "info",
                    rmcp::model::LoggingLevel::Notice => "notice",
                    rmcp::model::LoggingLevel::Warning => "warning",
                    rmcp::model::LoggingLevel::Error => "error",
                    rmcp::model::LoggingLevel::Critical => "critical",
                    rmcp::model::LoggingLevel::Alert => "alert",
                    rmcp::model::LoggingLevel::Emergency => "emergency",
                }
                .to_string(),
            ),
        );
        if let Some(logger) = param.logger.clone() {
            data.insert("logger".to_string(), Value::String(logger.to_string()));
        }
        data.insert("data".to_string(), param.data.clone());
        crate::audit::interceptor::emit_event(
            self.audit_service.as_ref(),
            crate::audit::interceptor::build_mcp_event(
                crate::audit::AuditAction::NotificationMessage,
                crate::audit::AuditStatus::Success,
                None,
                None,
                None,
                None,
                Some(data),
                None,
            ),
        )
        .await;
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
        let started_at = std::time::Instant::now();
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

        let mut data = Map::new();
        data.insert(
            "client_name".to_string(),
            Value::String(request.client_info.name.clone()),
        );
        data.insert(
            "client_version".to_string(),
            Value::String(request.client_info.version.clone()),
        );
        data.insert(
            "has_roots".to_string(),
            Value::Bool(request.capabilities.roots.is_some()),
        );
        data.insert(
            "has_sampling".to_string(),
            Value::Bool(request.capabilities.sampling.is_some()),
        );
        data.insert(
            "has_elicitation".to_string(),
            Value::Bool(request.capabilities.elicitation.is_some()),
        );
        crate::audit::interceptor::emit_event(
            self.audit_service.as_ref(),
            crate::audit::interceptor::build_mcp_event(
                crate::audit::AuditAction::Initialize,
                crate::audit::AuditStatus::Success,
                Some(&client),
                Some(request.protocol_version.to_string()),
                None,
                Some(started_at.elapsed().as_millis() as u64),
                Some(data),
                None,
            ),
        )
        .await;

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
        let audit_client = self.resolve_bound_client_context(&_context).await.ok();
        let started_at = std::time::Instant::now();
        let request_data = _request.as_ref().map(paginated_request_data);
        let protocol_version = self.protocol_version_from_context(&_context);
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let result = super::tools::list_tools(self, _request, _context).await;
        emit_mcp_result(
            self,
            crate::audit::AuditAction::ToolsList,
            audit_client.as_ref(),
            None,
            started_at.elapsed().as_millis() as u64,
            McpAuditExtras {
                data: request_data,
                protocol_version,
                request_id: None,
                progress_token: None,
                detail: None,
            },
            result.as_ref().err().map(ToString::to_string),
        )
        .await;
        result
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let audit_client = self.resolve_bound_client_context(&_context).await.ok();
        let started_at = std::time::Instant::now();
        let target = Some(request.name.to_string());
        let protocol_version = self.protocol_version_from_context(&_context);
        let mut request_data = Map::new();
        request_data.insert("tool_name".to_string(), Value::String(request.name.to_string()));
        if let Some(arguments) = request.arguments.clone() {
            request_data.insert("arguments".to_string(), Value::Object(arguments));
        }
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let result = super::tools::call_tool(self, request, _context).await;
        emit_mcp_result(
            self,
            crate::audit::AuditAction::ToolsCall,
            audit_client.as_ref(),
            target,
            started_at.elapsed().as_millis() as u64,
            McpAuditExtras {
                data: Some(request_data),
                protocol_version,
                request_id: None,
                progress_token: None,
                detail: Some("Called MCP tool".to_string()),
            },
            result.as_ref().err().map(ToString::to_string),
        )
        .await;
        result
    }

    async fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        let audit_client = self.resolve_bound_client_context(&_context).await.ok();
        let started_at = std::time::Instant::now();
        let request_data = _request.as_ref().map(paginated_request_data);
        let protocol_version = self.protocol_version_from_context(&_context);
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let result = super::resources::list_resources(self, _request, _context).await;
        emit_mcp_result(
            self,
            crate::audit::AuditAction::ResourcesList,
            audit_client.as_ref(),
            None,
            started_at.elapsed().as_millis() as u64,
            McpAuditExtras {
                data: request_data,
                protocol_version,
                request_id: None,
                progress_token: None,
                detail: None,
            },
            result.as_ref().err().map(ToString::to_string),
        )
        .await;
        result
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
        let audit_client = self.resolve_bound_client_context(&_context).await.ok();
        let started_at = std::time::Instant::now();
        let target = Some(request.uri.to_string());
        let protocol_version = self.protocol_version_from_context(&_context);
        let mut request_data = Map::new();
        request_data.insert("resource_uri".to_string(), Value::String(request.uri.to_string()));
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let result = super::resources::read_resource(self, request, _context).await;
        emit_mcp_result(
            self,
            crate::audit::AuditAction::ResourcesRead,
            audit_client.as_ref(),
            target,
            started_at.elapsed().as_millis() as u64,
            McpAuditExtras {
                data: Some(request_data),
                protocol_version,
                request_id: None,
                progress_token: None,
                detail: Some("Read MCP resource".to_string()),
            },
            result.as_ref().err().map(ToString::to_string),
        )
        .await;
        result
    }

    async fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<ListPromptsResult, rmcp::ErrorData> {
        let audit_client = self.resolve_bound_client_context(&_context).await.ok();
        let started_at = std::time::Instant::now();
        let request_data = _request.as_ref().map(paginated_request_data);
        let protocol_version = self.protocol_version_from_context(&_context);
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let result = super::prompts::list_prompts(self, _request, _context).await;
        emit_mcp_result(
            self,
            crate::audit::AuditAction::PromptsList,
            audit_client.as_ref(),
            None,
            started_at.elapsed().as_millis() as u64,
            McpAuditExtras {
                data: request_data,
                protocol_version,
                request_id: None,
                progress_token: None,
                detail: None,
            },
            result.as_ref().err().map(ToString::to_string),
        )
        .await;
        result
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<GetPromptResult, rmcp::ErrorData> {
        let audit_client = self.resolve_bound_client_context(&_context).await.ok();
        let started_at = std::time::Instant::now();
        let target = Some(request.name.to_string());
        let protocol_version = self.protocol_version_from_context(&_context);
        let mut request_data = Map::new();
        request_data.insert("prompt_name".to_string(), Value::String(request.name.to_string()));
        if let Some(arguments) = request.arguments.clone() {
            request_data.insert("arguments".to_string(), Value::Object(arguments));
        }
        self.enforce_mcp_protocol_header(&_context)?;
        self.enforce_origin_if_present(&_context)?;
        let result = super::prompts::get_prompt(self, request, _context).await;
        emit_mcp_result(
            self,
            crate::audit::AuditAction::PromptsGet,
            audit_client.as_ref(),
            target,
            started_at.elapsed().as_millis() as u64,
            McpAuditExtras {
                data: Some(request_data),
                protocol_version,
                request_id: None,
                progress_token: None,
                detail: Some("Get MCP prompt".to_string()),
            },
            result.as_ref().err().map(ToString::to_string),
        )
        .await;
        result
    }
}

struct McpAuditExtras {
    data: Option<Map<String, Value>>,
    protocol_version: Option<String>,
    request_id: Option<String>,
    progress_token: Option<String>,
    detail: Option<String>,
}

async fn emit_mcp_result(
    server: &ProxyServer,
    action: crate::audit::AuditAction,
    client: Option<&ClientContext>,
    target: Option<String>,
    duration_ms: u64,
    extras: McpAuditExtras,
    error_message: Option<String>,
) {
    let status = if error_message.is_some() {
        crate::audit::AuditStatus::Failed
    } else {
        crate::audit::AuditStatus::Success
    };
    let mut event = crate::audit::AuditEvent::new(action, status)
        .with_mcp_method(crate::audit::interceptor::mcp_method_name(action))
        .with_direction("client_to_server")
        .with_duration_ms(duration_ms);
    if let Some(client) = client {
        event = apply_client_audit_context(event, client);
    }
    if let Some(target) = target {
        event = event.with_target(target);
    }
    if let Some(protocol_version) = extras.protocol_version {
        event = event.with_protocol_version(protocol_version);
    }
    if let Some(data) = extras.data {
        event = event.with_mcp_data(data);
    }
    if let Some(request_id) = extras.request_id {
        event = event.with_request_id(request_id);
    }
    if let Some(progress_token) = extras.progress_token {
        event = event.with_task_metadata(None, None, Some(progress_token));
    }
    if let Some(detail) = extras.detail {
        event = event.with_detail(detail);
    }
    if let Some(error_message) = error_message {
        event = event.with_error(None::<String>, error_message);
    }
    crate::audit::interceptor::emit_event(server.audit_service.as_ref(), event.build()).await;
}

fn paginated_request_data(request: &rmcp::model::PaginatedRequestParams) -> Map<String, Value> {
    let mut data = Map::new();
    if let Some(cursor) = request.cursor.clone() {
        data.insert("cursor".to_string(), Value::String(cursor));
    }
    data
}

fn apply_client_audit_context(
    mut event: crate::audit::AuditEvent,
    client: &ClientContext,
) -> crate::audit::AuditEvent {
    event = event.with_client_id(client.client_id.clone());
    if let Some(profile_id) = &client.profile_id {
        event = event.with_profile_id(profile_id.clone());
    }
    if let Some(session_id) = &client.session_id {
        event = event.with_session_id(session_id.clone());
    }
    event
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::client::init::initialize_client_table;
    use crate::config::database::Database;
    use crate::core::models::Config;
    use crate::core::proxy::server::common::{ClientIdentitySource, ClientTransport};
    use sqlx::sqlite::SqlitePoolOptions;
    use tempfile::TempDir;

    struct TestServerState {
        downstream_clients: Arc<dashmap::DashMap<String, rmcp::service::Peer<rmcp::RoleServer>>>,
        resource_subscriptions: Arc<dashmap::DashMap<(String, String), String>>,
        server_resource_index: Arc<dashmap::DashMap<String, dashmap::DashSet<(String, String)>>>,
        session_bindings: Arc<SessionBoundClientContextResolver>,
    }

    fn create_test_server_state() -> TestServerState {
        TestServerState {
            downstream_clients: Arc::new(dashmap::DashMap::new()),
            resource_subscriptions: Arc::new(dashmap::DashMap::new()),
            server_resource_index: Arc::new(dashmap::DashMap::new()),
            session_bindings: Arc::new(SessionBoundClientContextResolver::new()),
        }
    }

    async fn create_mode_resolution_test_server() -> (TempDir, sqlx::SqlitePool, ProxyServer) {
        let temp_dir = TempDir::new().expect("temp dir");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable foreign keys");
        initialize_client_table(&pool).await.expect("init client table");

        let database = Arc::new(Database {
            pool: pool.clone(),
            path: temp_dir.path().join("test.db"),
        });

        let mut server = ProxyServer::new(Arc::new(Config::default()));
        server.database = Some(database);

        (temp_dir, pool, server)
    }

    async fn cleanup_session_state(
        session_id: &str,
        state: &TestServerState,
    ) {
        state.downstream_clients.remove(session_id);

        let subscription_keys: Vec<((String, String), String)> = state
            .resource_subscriptions
            .iter()
            .filter(|entry| entry.key().0 == session_id)
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect();
        for ((subscription_session, unique_uri), server_id) in subscription_keys {
            state
                .resource_subscriptions
                .remove(&(subscription_session.clone(), unique_uri.clone()));
            if !server_id.is_empty() {
                if let Some(index) = state.server_resource_index.get(&server_id) {
                    index.remove(&(subscription_session, unique_uri));
                }
            }
        }

        let _ = state.session_bindings.unbind_session(session_id).await;
    }

    #[tokio::test]
    async fn resource_subscriptions_cleanup_removes_session_entries() {
        let state = create_test_server_state();
        let session_id = "test-session";

        state.resource_subscriptions.insert(
            (session_id.to_string(), "resource://a".to_string()),
            "srv-1".to_string(),
        );
        state.resource_subscriptions.insert(
            (session_id.to_string(), "resource://b".to_string()),
            "srv-1".to_string(),
        );
        state
            .resource_subscriptions
            .insert(("other".to_string(), "resource://c".to_string()), "srv-1".to_string());

        {
            let idx = state.server_resource_index.entry("srv-1".to_string()).or_default();
            idx.insert((session_id.to_string(), "resource://a".to_string()));
            idx.insert((session_id.to_string(), "resource://b".to_string()));
            idx.insert(("other".to_string(), "resource://c".to_string()));
        }

        assert_eq!(state.resource_subscriptions.len(), 3);

        cleanup_session_state(session_id, &state).await;

        assert!(
            !state
                .resource_subscriptions
                .contains_key(&(session_id.to_string(), "resource://a".to_string()))
        );
        assert!(
            !state
                .resource_subscriptions
                .contains_key(&(session_id.to_string(), "resource://b".to_string()))
        );
        assert!(
            state
                .resource_subscriptions
                .contains_key(&("other".to_string(), "resource://c".to_string()))
        );

        let idx = state.server_resource_index.get("srv-1").expect("index should exist");
        assert!(!idx.contains(&(session_id.to_string(), "resource://a".to_string())));
        assert!(idx.contains(&("other".to_string(), "resource://c".to_string())));
    }

    #[tokio::test]
    async fn cleanup_is_idempotent() {
        let state = create_test_server_state();
        let session_id = "test-idempotent";

        cleanup_session_state(session_id, &state).await;
        cleanup_session_state(session_id, &state).await;
    }

    #[tokio::test]
    async fn session_binding_cleanup_via_resolver() {
        let resolver = Arc::new(SessionBoundClientContextResolver::new());
        let session_id = "test-binding-cleanup";

        let context = ClientContext {
            client_id: "client-1".to_string(),
            session_id: Some(session_id.to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };

        resolver
            .bind_session(session_id, &context)
            .await
            .expect("bind should succeed");
        assert!(resolver.session_bindings.contains_key(session_id));

        resolver
            .unbind_session(session_id)
            .await
            .expect("unbind should succeed");
        assert!(!resolver.session_bindings.contains_key(session_id));

        resolver
            .unbind_session(session_id)
            .await
            .expect("unbind should be idempotent");
    }

    #[tokio::test]
    async fn multiple_sessions_cleanup_isolation() {
        let state = create_test_server_state();
        let session_a = "session-a";
        let session_b = "session-b";

        state
            .resource_subscriptions
            .insert((session_a.to_string(), "resource://1".to_string()), "srv".to_string());
        state
            .resource_subscriptions
            .insert((session_b.to_string(), "resource://2".to_string()), "srv".to_string());

        {
            let idx = state.server_resource_index.entry("srv".to_string()).or_default();
            idx.insert((session_a.to_string(), "resource://1".to_string()));
            idx.insert((session_b.to_string(), "resource://2".to_string()));
        }

        cleanup_session_state(session_a, &state).await;

        assert!(
            !state
                .resource_subscriptions
                .contains_key(&(session_a.to_string(), "resource://1".to_string()))
        );
        assert!(
            state
                .resource_subscriptions
                .contains_key(&(session_b.to_string(), "resource://2".to_string()))
        );
    }

    #[test]
    fn client_context_without_session() {
        let context_no_session = ClientContext {
            client_id: "no-session".to_string(),
            session_id: None,
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };

        assert!(context_no_session.session_id.is_none());
    }

    #[test]
    fn client_context_with_session() {
        let context_with_session = ClientContext {
            client_id: "with-session".to_string(),
            session_id: Some("sess-123".to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };

        assert_eq!(context_with_session.session_id.as_deref(), Some("sess-123"));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn resolve_effective_config_mode_prefers_explicit_client_mode() {
        let (_temp_dir, pool, server) = create_mode_resolution_test_server().await;

        sqlx::query("UPDATE client_runtime_settings SET value = ? WHERE key = ?")
            .bind("transparent")
            .bind("default_config_mode")
            .execute(&pool)
            .await
            .expect("set default mode");

        sqlx::query(
            r#"
            INSERT INTO client (id, name, identifier, managed, config_mode, backup_policy, backup_limit)
            VALUES (?, ?, ?, 1, ?, 'keep_n', 30)
            "#,
        )
        .bind(crate::generate_id!("clnt"))
        .bind("Recognized Client")
        .bind("recognized-client")
        .bind("unify")
        .execute(&pool)
        .await
        .expect("insert client row");

        let mode = server
            .resolve_effective_config_mode("recognized-client")
            .await
            .expect("resolve config mode");

        assert_eq!(mode, "unify");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn resolve_effective_config_mode_uses_settings_default_for_recognized_client_without_explicit_mode() {
        let (_temp_dir, pool, server) = create_mode_resolution_test_server().await;

        sqlx::query("UPDATE client_runtime_settings SET value = ? WHERE key = ?")
            .bind("transparent")
            .bind("default_config_mode")
            .execute(&pool)
            .await
            .expect("set default mode");

        sqlx::query(
            r#"
            INSERT INTO client (id, name, identifier, managed, config_mode, backup_policy, backup_limit)
            VALUES (?, ?, ?, 1, ?, 'keep_n', 30)
            "#,
        )
        .bind(crate::generate_id!("clnt"))
        .bind("Recognized Client Without Explicit Mode")
        .bind("recognized-client-with-default")
        .bind(Option::<String>::None)
        .execute(&pool)
        .await
        .expect("insert client row with null mode");

        let mode = server
            .resolve_effective_config_mode("recognized-client-with-default")
            .await
            .expect("resolve config mode");

        assert_eq!(mode, "transparent");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn resolve_effective_config_mode_uses_settings_default_for_unrecognized_client() {
        let (_temp_dir, pool, server) = create_mode_resolution_test_server().await;

        sqlx::query("UPDATE client_runtime_settings SET value = ? WHERE key = ?")
            .bind("transparent")
            .bind("default_config_mode")
            .execute(&pool)
            .await
            .expect("set default mode");

        let mode = server
            .resolve_effective_config_mode("manual-unrecognized-client")
            .await
            .expect("resolve config mode");

        assert_eq!(mode, "transparent");
    }
}
