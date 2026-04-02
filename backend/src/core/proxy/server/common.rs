use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use dashmap::DashMap;
use rmcp::{
    RoleServer, Service,
    model::InitializeRequestParams,
    service::RequestContext,
    transport::{
        StreamableHttpServerConfig, StreamableHttpService, streamable_http_server::session::local::LocalSessionManager,
    },
};
use std::sync::Arc;

use crate::clients::models::ClientCapabilityConfig;

const MANAGED_CLIENT_ID_HEADER: &str = "x-mcpmate-client-id";
const MANAGED_PROFILE_ID_HEADER: &str = "x-mcpmate-profile-id";

/// Determine whether a server declares a given capability token
pub fn supports_capability(
    capabilities: Option<&str>,
    kind: crate::core::capability::CapabilityType,
) -> bool {
    let token = match kind {
        crate::core::capability::CapabilityType::Tools => crate::common::capability::CapabilityToken::Tools.as_str(),
        crate::core::capability::CapabilityType::Prompts => {
            crate::common::capability::CapabilityToken::Prompts.as_str()
        }
        crate::core::capability::CapabilityType::Resources
        | crate::core::capability::CapabilityType::ResourceTemplates => {
            crate::common::capability::CapabilityToken::Resources.as_str()
        }
    };

    crate::core::capability::facade::capability_declared(capabilities, token)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedClientInfo {
    pub name: String,
    pub version: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientContext {
    pub client_id: String,
    pub session_id: Option<String>,
    pub profile_id: Option<String>,
    pub config_mode: Option<String>,
    pub unify_workspace: Option<ClientCapabilityConfig>,
    pub rules_fingerprint: Option<String>,
    pub transport: ClientTransport,
    pub source: ClientIdentitySource,
    pub observed_client_info: Option<ObservedClientInfo>,
}

impl ClientContext {
    pub fn runtime_identity(&self) -> Option<crate::core::capability::RuntimeIdentity> {
        self.rules_fingerprint
            .clone()
            .map(|rules_fingerprint| crate::core::capability::RuntimeIdentity {
                client_id: self.client_id.clone(),
                profile_id: self.profile_id.clone(),
                rules_fingerprint,
            })
    }

    pub fn connection_mode(&self) -> crate::core::capability::ConnectionMode {
        match self.transport {
            ClientTransport::StreamableHttp => crate::core::capability::ConnectionMode::shareable(),
            ClientTransport::Other => {
                if let Some(session_id) = self.session_id.clone() {
                    crate::core::capability::ConnectionMode::per_session(session_id)
                } else {
                    crate::core::capability::ConnectionMode::per_client(self.client_id.clone())
                }
            }
        }
    }

    pub fn connection_selection(
        &self,
        server_id: impl Into<String>,
    ) -> Option<crate::core::capability::ConnectionSelection> {
        self.runtime_identity()
            .map(|identity| crate::core::capability::ConnectionSelection {
                server_id: server_id.into(),
                affinity_key: self.connection_mode().affinity_key,
                routing_fingerprint: Some(identity.rules_fingerprint),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientTransport {
    StreamableHttp,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientIdentitySource {
    ManagedQuery,
    ManagedHeader,
    SessionBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionBinding {
    pub session_id: String,
    pub client_id: String,
    pub profile_id: Option<String>,
    pub config_mode: Option<String>,
    pub unify_workspace: Option<ClientCapabilityConfig>,
    pub rules_fingerprint: Option<String>,
    pub source: ClientIdentitySource,
    pub observed_client_info: Option<ObservedClientInfo>,
}

/// Resolver for managed client contexts that maintains session bindings.
///
/// ## Storage and Lifecycle
///
/// This resolver maintains three concurrent maps:
///
/// - `session_bindings`: Active session-to-client mappings. Cleared on explicit unbind.
/// - `observed_clients`: Client info cache. Grows unbounded; entries persist across sessions.
/// - `pending_initializations`: Transient contexts awaiting session binding. Cleared on bind.
///
/// ## Cleanup Semantics
///
/// `unbind_session` removes the session binding but intentionally leaves `observed_clients`
/// and `pending_initializations` untouched:
///
/// - `observed_clients`: Serves as a cache for reconnections; retained for future sessions.
/// - `pending_initializations`: Keyed by peer pointer; stale entries are harmless but consume
///   memory until the peer is dropped. A future TTL-based cleanup could be added if needed.
#[derive(Debug, Clone, Default)]
pub struct SessionBoundClientContextResolver {
    pub session_bindings: Arc<DashMap<String, SessionBinding>>,
    pub observed_clients: Arc<DashMap<String, ObservedClientInfo>>,
    pub pending_initializations: Arc<DashMap<usize, ClientContext>>,
}

#[async_trait]
pub trait ManagedClientContextResolver: Send + Sync {
    async fn resolve_initialize_context(
        &self,
        request: &InitializeRequestParams,
        context: &RequestContext<RoleServer>,
    ) -> Result<ClientContext>;

    async fn bind_session(
        &self,
        session_id: &str,
        client_context: &ClientContext,
    ) -> Result<()>;

    async fn resolve_request_context(
        &self,
        context: &RequestContext<RoleServer>,
    ) -> Result<ClientContext>;

    async fn unbind_session(
        &self,
        session_id: &str,
    ) -> Result<()>;

    async fn refresh_session_rules_fingerprint(
        &self,
        session_id: &str,
        rules_fingerprint: String,
    ) -> Result<()>;

    async fn set_unify_workspace(
        &self,
        session_id: &str,
        workspace: Option<ClientCapabilityConfig>,
    ) -> Result<()>;
}

impl SessionBoundClientContextResolver {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ManagedClientContextResolver for SessionBoundClientContextResolver {
    async fn resolve_initialize_context(
        &self,
        request: &InitializeRequestParams,
        context: &RequestContext<RoleServer>,
    ) -> Result<ClientContext> {
        let client_context =
            resolve_initialize_context_parts(context.extensions.get::<axum::http::request::Parts>(), request)?;

        if let Some(observed) = client_context.observed_client_info.clone() {
            self.observed_clients.insert(client_context.client_id.clone(), observed);
        }

        if let Some(session_id) = client_context.session_id.clone() {
            self.bind_session(&session_id, &client_context).await?;
        } else {
            let peer_key = peer_context_key(context)?;
            self.pending_initializations.insert(peer_key, client_context.clone());
        }

        Ok(client_context)
    }

    async fn bind_session(
        &self,
        session_id: &str,
        client_context: &ClientContext,
    ) -> Result<()> {
        let observed_client_info = client_context.observed_client_info.clone().or_else(|| {
            self.observed_clients
                .get(&client_context.client_id)
                .map(|entry| entry.clone())
        });

        if let Some(existing) = self.session_bindings.get(session_id) {
            if existing.client_id != client_context.client_id {
                return Err(anyhow!(
                    "Managed session '{}' already bound to client_id '{}' instead of '{}'",
                    session_id,
                    existing.client_id,
                    client_context.client_id
                ));
            }
            if existing.profile_id != client_context.profile_id {
                return Err(anyhow!(
                    "Managed session '{}' already bound to profile_id {:?} instead of {:?}",
                    session_id,
                    existing.profile_id,
                    client_context.profile_id
                ));
            }
            // Allow upgrading from None to Some(fingerprint) during initialize flow,
            // but reject real mismatches (both Some but different values).
            match (&existing.rules_fingerprint, &client_context.rules_fingerprint) {
                (Some(existing_fp), Some(new_fp)) if existing_fp != new_fp => {
                    return Err(anyhow!(
                        "Managed session '{}' already bound to rules_fingerprint {:?} instead of {:?}",
                        session_id,
                        existing.rules_fingerprint,
                        client_context.rules_fingerprint
                    ));
                }
                (Some(_), None) => {
                    return Err(anyhow!(
                        "Managed session '{}' already bound to rules_fingerprint {:?}, cannot downgrade to None",
                        session_id,
                        existing.rules_fingerprint
                    ));
                }
                _ => {}
            }
        }

        self.session_bindings.insert(
            session_id.to_string(),
            SessionBinding {
                session_id: session_id.to_string(),
                client_id: client_context.client_id.clone(),
                profile_id: client_context.profile_id.clone(),
                config_mode: client_context.config_mode.clone(),
            unify_workspace: client_context.unify_workspace.clone(),
                rules_fingerprint: client_context.rules_fingerprint.clone(),
                source: client_context.source,
                observed_client_info,
            },
        );

        Ok(())
    }

    async fn resolve_request_context(
        &self,
        context: &RequestContext<RoleServer>,
    ) -> Result<ClientContext> {
        let parts = context
            .extensions
            .get::<axum::http::request::Parts>()
            .ok_or_else(|| anyhow!("Managed downstream request is missing HTTP request parts"))?;

        if let Some(bound) = resolve_bound_request_context_parts(parts, &self.session_bindings)? {
            return Ok(bound);
        }

        let session_id = extract_session_id(parts)
            .ok_or_else(|| anyhow!("Managed downstream session is required before request context resolution"))?;
        let peer_key = peer_context_key(context)?;
        let pending_client_context = self
            .pending_initializations
            .get(&peer_key)
            .map(|entry| entry.clone())
            .ok_or_else(|| {
                anyhow!(
                    "Managed downstream session '{}' has no initialize-bound identity",
                    session_id
                )
            })?;
        let client_context = resolve_pending_session_context_parts(parts, &pending_client_context)?;
        self.bind_session(&session_id, &client_context).await?;
        self.pending_initializations.remove(&peer_key);
        resolve_bound_request_context_parts(parts, &self.session_bindings)?.ok_or_else(|| {
            anyhow!(
                "Managed downstream session '{}' failed to resolve after binding",
                session_id
            )
        })
    }

    async fn unbind_session(
        &self,
        session_id: &str,
    ) -> Result<()> {
        self.session_bindings.remove(session_id);
        Ok(())
    }

    async fn refresh_session_rules_fingerprint(
        &self,
        session_id: &str,
        rules_fingerprint: String,
    ) -> Result<()> {
        let mut binding = self
            .session_bindings
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Managed session '{}' is not bound", session_id))?;
        binding.rules_fingerprint = Some(rules_fingerprint);
        Ok(())
    }

    async fn set_unify_workspace(
        &self,
        session_id: &str,
        workspace: Option<ClientCapabilityConfig>,
    ) -> Result<()> {
        let mut binding = self
            .session_bindings
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("Managed session '{}' is not bound", session_id))?;
        binding.unify_workspace = workspace;
        Ok(())
    }
}

pub fn resolve_initialize_context_parts(
    parts: Option<&axum::http::request::Parts>,
    initialize: &InitializeRequestParams,
) -> Result<ClientContext> {
    resolve_managed_context(parts, Some(initialize))
}

pub fn resolve_request_context_parts(parts: &axum::http::request::Parts) -> Result<ClientContext> {
    resolve_managed_context(Some(parts), None)
}

fn resolve_bound_request_context_parts(
    parts: &axum::http::request::Parts,
    session_bindings: &DashMap<String, SessionBinding>,
) -> Result<Option<ClientContext>> {
    let Some(session_id) = extract_session_id(parts) else {
        return Ok(None);
    };

    let Some(binding) = session_bindings.get(&session_id) else {
        return Ok(None);
    };

    validate_request_identity_matches_context(parts, &binding.client_id, binding.profile_id.as_deref())?;

    Ok(Some(ClientContext {
        client_id: binding.client_id.clone(),
        session_id: Some(session_id),
        profile_id: binding.profile_id.clone(),
        config_mode: binding.config_mode.clone(),
            unify_workspace: binding.unify_workspace.clone(),
        rules_fingerprint: binding.rules_fingerprint.clone(),
        transport: transport_from_parts(Some(parts)),
        source: ClientIdentitySource::SessionBinding,
        observed_client_info: binding.observed_client_info.clone(),
    }))
}

fn resolve_pending_session_context_parts(
    parts: &axum::http::request::Parts,
    initialize_context: &ClientContext,
) -> Result<ClientContext> {
    let session_id = extract_session_id(parts)
        .ok_or_else(|| anyhow!("Managed downstream session is required before request context resolution"))?;
    validate_request_identity_matches_context(
        parts,
        &initialize_context.client_id,
        initialize_context.profile_id.as_deref(),
    )?;

    Ok(ClientContext {
        client_id: initialize_context.client_id.clone(),
        session_id: Some(session_id),
        profile_id: initialize_context.profile_id.clone(),
        config_mode: initialize_context.config_mode.clone(),
            unify_workspace: initialize_context.unify_workspace.clone(),
        rules_fingerprint: initialize_context.rules_fingerprint.clone(),
        transport: transport_from_parts(Some(parts)),
        source: ClientIdentitySource::SessionBinding,
        observed_client_info: initialize_context.observed_client_info.clone(),
    })
}

fn validate_request_identity_matches_context(
    parts: &axum::http::request::Parts,
    client_id: &str,
    profile_id: Option<&str>,
) -> Result<()> {
    if let Some((request_client_id, _)) = resolve_optional_managed_client_id(parts)? {
        if request_client_id != client_id {
            return Err(anyhow!(
                "Managed client_id '{}' does not match session binding '{}'",
                request_client_id,
                client_id
            ));
        }
    }

    let request_profile_id = resolve_managed_profile_id(parts)?;
    match (profile_id, request_profile_id) {
        (Some(bound_profile_id), Some(request_profile_id)) if bound_profile_id != request_profile_id => Err(anyhow!(
            "Managed profile_id '{}' does not match session binding '{}'",
            request_profile_id,
            bound_profile_id
        )),
        (Some(_), _) => Ok(()),
        (None, Some(request_profile_id)) => Err(anyhow!(
            "Managed profile_id '{}' does not match session binding without profile_id",
            request_profile_id
        )),
        (None, None) => Ok(()),
    }
}

fn peer_context_key(context: &RequestContext<RoleServer>) -> Result<usize> {
    let peer_info: &InitializeRequestParams = context
        .peer
        .peer_info()
        .ok_or_else(|| anyhow!("Managed downstream peer is missing initialize peer info"))?;
    Ok(std::ptr::from_ref(peer_info) as usize)
}

fn resolve_managed_context(
    parts: Option<&axum::http::request::Parts>,
    initialize: Option<&InitializeRequestParams>,
) -> Result<ClientContext> {
    let parts =
        parts.ok_or_else(|| anyhow!("Managed downstream requests require MCPMate-managed HTTP side-band metadata"))?;
    let session_id = extract_session_id(parts);
    let profile_id = resolve_managed_profile_id(parts)?;
    let observed_client_info = initialize.map(observe_client_info);
    let (client_id, source) = resolve_managed_client_id(parts)?;

    if let Some(bridge_client_id) = initialize.and_then(extract_bridge_client_id) {
        if bridge_client_id != client_id {
            return Err(anyhow!(
                "Managed client_id '{}' does not match bridge APPID '{}'",
                client_id,
                bridge_client_id
            ));
        }
    }

    Ok(ClientContext {
        client_id,
        session_id,
        profile_id,
        config_mode: None,
            unify_workspace: None,
        rules_fingerprint: None,
        transport: transport_from_parts(Some(parts)),
        source,
        observed_client_info,
    })
}

fn resolve_managed_client_id(parts: &axum::http::request::Parts) -> Result<(String, ClientIdentitySource)> {
    resolve_optional_managed_client_id(parts)?
        .ok_or_else(|| anyhow!("Managed client_id side-band is required; clientInfo is observation-only"))
}

fn resolve_optional_managed_client_id(
    parts: &axum::http::request::Parts
) -> Result<Option<(String, ClientIdentitySource)>> {
    let query_client_id = extract_query_client_id(parts);
    let header_client_id = extract_header_client_id(parts);

    match (query_client_id, header_client_id) {
        (Some(query_client_id), Some(header_client_id)) if query_client_id != header_client_id => Err(anyhow!(
            "Managed client identity mismatch between query '{}' and header '{}'",
            query_client_id,
            header_client_id
        )),
        (Some(query_client_id), _) => Ok(Some((query_client_id, ClientIdentitySource::ManagedQuery))),
        (None, Some(header_client_id)) => Ok(Some((header_client_id, ClientIdentitySource::ManagedHeader))),
        (None, None) => Ok(None),
    }
}

fn resolve_managed_profile_id(parts: &axum::http::request::Parts) -> Result<Option<String>> {
    let query_profile_id = extract_query_profile_id(parts);
    let header_profile_id = extract_header_profile_id(parts);

    match (query_profile_id, header_profile_id) {
        (Some(query_profile_id), Some(header_profile_id)) if query_profile_id != header_profile_id => Err(anyhow!(
            "Managed profile identity mismatch between query '{}' and header '{}'",
            query_profile_id,
            header_profile_id
        )),
        (Some(query_profile_id), _) => Ok(Some(query_profile_id)),
        (None, Some(header_profile_id)) => Ok(Some(header_profile_id)),
        (None, None) => Ok(None),
    }
}

fn observe_client_info(initialize: &InitializeRequestParams) -> ObservedClientInfo {
    ObservedClientInfo {
        name: initialize.client_info.name.clone(),
        version: initialize.client_info.version.clone(),
        title: initialize.client_info.title.clone(),
    }
}

fn transport_from_parts(parts: Option<&axum::http::request::Parts>) -> ClientTransport {
    if parts.is_some() {
        ClientTransport::StreamableHttp
    } else {
        ClientTransport::Other
    }
}

fn extract_session_id(parts: &axum::http::request::Parts) -> Option<String> {
    parts
        .headers
        .get("mcp-session-id")
        .or_else(|| parts.headers.get("MCP-Session-Id"))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_query_profile_id(parts: &axum::http::request::Parts) -> Option<String> {
    parts.uri.query().and_then(|query| query_value(query, "profile_id"))
}

fn extract_query_client_id(parts: &axum::http::request::Parts) -> Option<String> {
    parts.uri.query().and_then(|query| query_value(query, "client_id"))
}

fn extract_header_profile_id(parts: &axum::http::request::Parts) -> Option<String> {
    extract_header_value(parts, MANAGED_PROFILE_ID_HEADER)
}

fn extract_header_client_id(parts: &axum::http::request::Parts) -> Option<String> {
    extract_header_value(parts, MANAGED_CLIENT_ID_HEADER)
}

fn extract_header_value(
    parts: &axum::http::request::Parts,
    header_name: &str,
) -> Option<String> {
    parts
        .headers
        .get(header_name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_bridge_client_id(initialize: &InitializeRequestParams) -> Option<String> {
    let name = initialize.client_info.name.trim();
    name.strip_prefix(crate::common::constants::branding::bridge::CLIENT_NAME_PREFIX)
        .and_then(|suffix| suffix.strip_prefix("::"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn query_value(
    query: &str,
    key: &str,
) -> Option<String> {
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value.into_owned())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Clone)]
pub struct UnifiedHttpServerConfig {
    pub bind_address: std::net::SocketAddr,
    pub streamable_http_path: String,
    pub keep_alive_interval: Option<std::time::Duration>,
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

impl Default for UnifiedHttpServerConfig {
    fn default() -> Self {
        use crate::common::constants::ports;
        Self {
            bind_address: format!("127.0.0.1:{}", ports::MCP_PORT).parse().unwrap(),
            streamable_http_path: "/mcp".to_string(),
            keep_alive_interval: Some(std::time::Duration::from_secs(15)),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        }
    }
}

/// Unified HTTP server that exposes only the streamable HTTP endpoint
pub struct UnifiedHttpServer {
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

    /// Start the unified HTTP server with the streamable HTTP endpoint
    pub async fn start<F, S>(
        &self,
        service_factory: F,
    ) -> Result<()>
    where
        F: Fn() -> S + Clone + Send + Sync + 'static,
        S: Service<RoleServer> + Send + Sync + 'static,
    {
        tracing::info!(
            "Starting unified HTTP server on {} with Streamable HTTP at {}",
            self.config.bind_address,
            self.config.streamable_http_path,
        );

        let streamable_http_config = StreamableHttpServerConfig {
            sse_keep_alive: self.config.keep_alive_interval,
            sse_retry: Some(std::time::Duration::from_secs(3)),
            stateful_mode: true,
            json_response: false,
            cancellation_token: self.config.cancellation_token.clone(),
        };

        let session_manager = std::sync::Arc::new(LocalSessionManager::default());

        let service_factory_clone = service_factory.clone();
        let streamable_http_service = StreamableHttpService::new(
            move || Ok(service_factory_clone()),
            session_manager,
            streamable_http_config,
        );

        let combined_router =
            axum::Router::new().route_service(&self.config.streamable_http_path, streamable_http_service);

        let listener = tokio::net::TcpListener::bind(self.config.bind_address)
            .await
            .context(format!("Failed to bind to address {}", self.config.bind_address))?;

        let ct = self.config.cancellation_token.child_token();

        let server = axum::serve(listener, combined_router).with_graceful_shutdown(async move {
            ct.cancelled().await;
            tracing::info!("Unified HTTP server cancelled");
        });

        let _ = service_factory;

        tokio::spawn(async move {
            if let Err(e) = server.await {
                tracing::error!(error = %e, "Unified HTTP server shutdown with error");
            }
        });

        tracing::info!("Unified HTTP server started successfully with the following endpoint:");
        tracing::info!(
            "  - Streamable HTTP: {}{}",
            self.config.bind_address,
            self.config.streamable_http_path
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, header::HeaderValue};
    use rmcp::model::{ClientCapabilities, Implementation, ProtocolVersion};

    fn build_initialize(name: &str) -> InitializeRequestParams {
        InitializeRequestParams::new(ClientCapabilities::default(), Implementation::new(name, "1.0.0"))
            .with_protocol_version(ProtocolVersion::LATEST)
    }

    fn build_parts(
        uri: &str,
        session_id: Option<&str>,
        managed_client_id: Option<&str>,
        managed_profile_id: Option<&str>,
    ) -> axum::http::request::Parts {
        let mut request = Request::builder().uri(uri).body(()).unwrap();
        if let Some(session_id) = session_id {
            request
                .headers_mut()
                .insert("mcp-session-id", HeaderValue::from_str(session_id).unwrap());
        }
        if let Some(managed_client_id) = managed_client_id {
            request.headers_mut().insert(
                MANAGED_CLIENT_ID_HEADER,
                HeaderValue::from_str(managed_client_id).unwrap(),
            );
        }
        if let Some(managed_profile_id) = managed_profile_id {
            request.headers_mut().insert(
                MANAGED_PROFILE_ID_HEADER,
                HeaderValue::from_str(managed_profile_id).unwrap(),
            );
        }
        request.into_parts().0
    }

    #[test]
    fn resolves_query_client_id_first() {
        let parts = build_parts(
            "/mcp?client_id=claude-desktop&profile_id=profile-a",
            Some("sess-123"),
            None,
            None,
        );
        let initialize = build_initialize("curl");

        let context = resolve_initialize_context_parts(Some(&parts), &initialize).expect("managed context");

        assert_eq!(context.client_id, "claude-desktop");
        assert_eq!(context.session_id.as_deref(), Some("sess-123"));
        assert_eq!(context.profile_id.as_deref(), Some("profile-a"));
        assert_eq!(context.source, ClientIdentitySource::ManagedQuery);
        assert_eq!(context.transport, ClientTransport::StreamableHttp);
        assert_eq!(
            context.observed_client_info.as_ref().map(|info| info.name.as_str()),
            Some("curl")
        );
    }

    #[test]
    fn resolves_header_client_id_when_query_missing() {
        let parts = build_parts("/mcp", Some("sess-123"), Some("claude-code"), Some("profile-b"));
        let initialize = build_initialize("curl");

        let context = resolve_initialize_context_parts(Some(&parts), &initialize).expect("managed context");

        assert_eq!(context.client_id, "claude-code");
        assert_eq!(context.profile_id.as_deref(), Some("profile-b"));
        assert_eq!(context.source, ClientIdentitySource::ManagedHeader);
    }

    #[test]
    fn rejects_initialize_without_managed_side_band() {
        let parts = build_parts("/mcp", None, None, None);
        let initialize = build_initialize("custom-client");

        let err =
            resolve_initialize_context_parts(Some(&parts), &initialize).expect_err("missing client id should fail");
        assert!(err.to_string().contains("Managed client_id side-band is required"));
    }

    #[test]
    fn rejects_bridge_identity_mismatch() {
        let parts = build_parts("/mcp?client_id=alpha", None, None, None);
        let initialize = build_initialize("mcpmate-bridge::beta");

        let err = resolve_initialize_context_parts(Some(&parts), &initialize).expect_err("bridge mismatch should fail");
        assert!(err.to_string().contains("does not match bridge APPID"));
    }

    #[test]
    fn resolves_bound_session_context() {
        let session_bindings = DashMap::new();
        session_bindings.insert(
            "sess-123".to_string(),
            SessionBinding {
                session_id: "sess-123".to_string(),
                client_id: "claude-code".to_string(),
                profile_id: Some("profile-a".to_string()),
                config_mode: Some("hosted".to_string()),
            unify_workspace: None,
                rules_fingerprint: Some("fp-123".to_string()),
                source: ClientIdentitySource::ManagedQuery,
                observed_client_info: Some(ObservedClientInfo {
                    name: "bridge".to_string(),
                    version: "1.0.0".to_string(),
                    title: None,
                }),
            },
        );

        let parts = build_parts("/mcp", Some("sess-123"), None, None);
        let context = resolve_bound_request_context_parts(&parts, &session_bindings)
            .expect("binding lookup")
            .expect("bound context");

        assert_eq!(context.client_id, "claude-code");
        assert_eq!(context.session_id.as_deref(), Some("sess-123"));
        assert_eq!(context.profile_id.as_deref(), Some("profile-a"));
        assert_eq!(context.rules_fingerprint.as_deref(), Some("fp-123"));
        assert_eq!(context.source, ClientIdentitySource::SessionBinding);
        assert_eq!(
            context.observed_client_info.as_ref().map(|info| info.name.as_str()),
            Some("bridge")
        );
    }

    #[test]
    fn rejects_bound_session_client_id_mismatch() {
        let session_bindings = DashMap::new();
        session_bindings.insert(
            "sess-123".to_string(),
            SessionBinding {
                session_id: "sess-123".to_string(),
                client_id: "claude-code".to_string(),
                profile_id: Some("profile-a".to_string()),
                config_mode: Some("hosted".to_string()),
            unify_workspace: None,
                rules_fingerprint: Some("fp-123".to_string()),
                source: ClientIdentitySource::ManagedQuery,
                observed_client_info: None,
            },
        );

        let parts = build_parts("/mcp?client_id=cursor", Some("sess-123"), None, None);
        let err =
            resolve_bound_request_context_parts(&parts, &session_bindings).expect_err("client mismatch should fail");
        assert!(err.to_string().contains("does not match session binding"));
    }

    #[test]
    fn rejects_bound_session_profile_id_mismatch() {
        let session_bindings = DashMap::new();
        session_bindings.insert(
            "sess-123".to_string(),
            SessionBinding {
                session_id: "sess-123".to_string(),
                client_id: "claude-code".to_string(),
                profile_id: Some("profile-a".to_string()),
                config_mode: Some("hosted".to_string()),
            unify_workspace: None,
                rules_fingerprint: Some("fp-123".to_string()),
                source: ClientIdentitySource::ManagedQuery,
                observed_client_info: None,
            },
        );

        let parts = build_parts("/mcp?profile_id=profile-b", Some("sess-123"), None, None);
        let err =
            resolve_bound_request_context_parts(&parts, &session_bindings).expect_err("profile mismatch should fail");
        assert!(err.to_string().contains("does not match session binding"));
    }

    #[test]
    fn resolves_pending_session_context_from_initialize_identity() {
        let initialize_context = ClientContext {
            client_id: "claude-code".to_string(),
            session_id: None,
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-123".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: Some(ObservedClientInfo {
                name: "bridge".to_string(),
                version: "1.0.0".to_string(),
                title: None,
            }),
        };

        let parts = build_parts("/mcp", Some("sess-123"), None, None);
        let context = resolve_pending_session_context_parts(&parts, &initialize_context).expect("pending context");

        assert_eq!(context.client_id, "claude-code");
        assert_eq!(context.session_id.as_deref(), Some("sess-123"));
        assert_eq!(context.profile_id.as_deref(), Some("profile-a"));
        assert_eq!(context.rules_fingerprint.as_deref(), Some("fp-123"));
        assert_eq!(context.source, ClientIdentitySource::SessionBinding);
        assert_eq!(
            context.observed_client_info.as_ref().map(|info| info.name.as_str()),
            Some("bridge")
        );
    }

    #[test]
    fn rejects_pending_session_context_client_id_mismatch() {
        let initialize_context = ClientContext {
            client_id: "claude-code".to_string(),
            session_id: None,
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-123".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };

        let parts = build_parts("/mcp?client_id=cursor", Some("sess-123"), None, None);
        let err = resolve_pending_session_context_parts(&parts, &initialize_context)
            .expect_err("client mismatch should fail");
        assert!(err.to_string().contains("does not match session binding"));
    }

    #[test]
    fn rejects_pending_session_context_profile_id_mismatch() {
        let initialize_context = ClientContext {
            client_id: "claude-code".to_string(),
            session_id: None,
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-123".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };

        let parts = build_parts("/mcp?profile_id=profile-b", Some("sess-123"), None, None);
        let err = resolve_pending_session_context_parts(&parts, &initialize_context)
            .expect_err("profile mismatch should fail");
        assert!(err.to_string().contains("does not match session binding"));
    }

    #[tokio::test]
    async fn rejects_rebinding_same_session_to_different_client() {
        let resolver = SessionBoundClientContextResolver::new();
        let first = ClientContext {
            client_id: "claude-code".to_string(),
            session_id: Some("sess-123".to_string()),
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-123".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };
        resolver.bind_session("sess-123", &first).await.expect("first binding");

        let second = ClientContext {
            client_id: "cursor".to_string(),
            session_id: Some("sess-123".to_string()),
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-123".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };
        let err = resolver
            .bind_session("sess-123", &second)
            .await
            .expect_err("rebinding should fail");
        assert!(err.to_string().contains("already bound to client_id"));
    }

    #[test]
    fn runtime_identity_includes_rules_fingerprint() {
        let context = ClientContext {
            client_id: "claude-code".to_string(),
            session_id: Some("sess-123".to_string()),
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-123".to_string()),
            transport: ClientTransport::Other,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };

        let identity = context.runtime_identity().expect("runtime identity");
        assert_eq!(identity.client_id, "claude-code");
        assert_eq!(identity.profile_id.as_deref(), Some("profile-a"));
        assert_eq!(identity.rules_fingerprint, "fp-123");
    }

    #[test]
    fn connection_selection_uses_session_affinity_for_non_http_clients() {
        let context = ClientContext {
            client_id: "claude-code".to_string(),
            session_id: Some("sess-123".to_string()),
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-123".to_string()),
            transport: ClientTransport::Other,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };

        let selection = context.connection_selection("srv_1").expect("selection");
        assert_eq!(selection.server_id, "srv_1");
        assert_eq!(selection.routing_fingerprint.as_deref(), Some("fp-123"));
        assert!(
            matches!(selection.affinity_key, crate::core::capability::AffinityKey::PerSession(ref id) if id == "sess-123")
        );
    }

    // ========================================
    // Session Binding Lifecycle Invariants
    // ========================================

    #[tokio::test]
    async fn unbind_session_removes_binding() {
        let resolver = SessionBoundClientContextResolver::new();
        let context = ClientContext {
            client_id: "test-client".to_string(),
            session_id: Some("sess-remove-test".to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };

        resolver
            .bind_session("sess-remove-test", &context)
            .await
            .expect("bind should succeed");
        assert!(
            resolver.session_bindings.contains_key("sess-remove-test"),
            "binding should exist after bind"
        );

        resolver
            .unbind_session("sess-remove-test")
            .await
            .expect("unbind should succeed");
        assert!(
            !resolver.session_bindings.contains_key("sess-remove-test"),
            "binding should be removed after unbind"
        );
    }

    #[tokio::test]
    async fn unbind_nonexistent_session_is_safe() {
        let resolver = SessionBoundClientContextResolver::new();
        // Unbinding a session that never existed should succeed without error
        let result = resolver.unbind_session("nonexistent-session").await;
        assert!(result.is_ok(), "unbinding nonexistent session should be safe");
    }

    #[tokio::test]
    async fn rebind_same_session_with_identical_context_succeeds() {
        let resolver = SessionBoundClientContextResolver::new();
        let context = ClientContext {
            client_id: "test-client".to_string(),
            session_id: Some("sess-rebind".to_string()),
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-123".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };

        resolver
            .bind_session("sess-rebind", &context)
            .await
            .expect("first bind should succeed");
        // Rebinding with identical context should succeed (idempotent)
        resolver
            .bind_session("sess-rebind", &context)
            .await
            .expect("rebind with identical context should succeed");
        assert!(resolver.session_bindings.contains_key("sess-rebind"));
    }

    #[tokio::test]
    async fn observed_clients_persists_across_bindings() {
        let resolver = SessionBoundClientContextResolver::new();
        let observed = ObservedClientInfo {
            name: "test-client".to_string(),
            version: "1.0.0".to_string(),
            title: Some("Test Client".to_string()),
        };
        let context = ClientContext {
            client_id: "client-obs".to_string(),
            session_id: Some("sess-obs".to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: Some(observed.clone()),
        };

        resolver
            .bind_session("sess-obs", &context)
            .await
            .expect("bind should succeed");
        let binding = resolver.session_bindings.get("sess-obs").expect("binding should exist");
        assert_eq!(
            binding.observed_client_info.as_ref().map(|o| o.name.as_str()),
            Some("test-client")
        );
    }

    #[tokio::test]
    async fn observed_clients_reused_for_subsequent_binding_without_observed_info() {
        let resolver = SessionBoundClientContextResolver::new();

        // First, store observed client info via the resolver's internal map
        let observed = ObservedClientInfo {
            name: "cached-client".to_string(),
            version: "2.0.0".to_string(),
            title: None,
        };
        resolver
            .observed_clients
            .insert("client-cached".to_string(), observed.clone());

        // Bind a session WITHOUT observed_client_info - should pick up from cached
        let context = ClientContext {
            client_id: "client-cached".to_string(),
            session_id: Some("sess-cached".to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };
        resolver
            .bind_session("sess-cached", &context)
            .await
            .expect("bind should succeed");

        let binding = resolver
            .session_bindings
            .get("sess-cached")
            .expect("binding should exist");
        assert_eq!(
            binding.observed_client_info.as_ref().map(|o| o.name.as_str()),
            Some("cached-client"),
            "binding should use cached observed_client_info"
        );
    }

    #[test]
    fn pending_initializations_stores_context_without_session_id() {
        let resolver = SessionBoundClientContextResolver::new();
        let context = ClientContext {
            client_id: "pending-client".to_string(),
            session_id: None, // No session ID
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::Other,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };

        // Use a fake peer_key (in real code this comes from peer pointer)
        let peer_key = 0x1234_usize;
        resolver.pending_initializations.insert(peer_key, context.clone());

        assert!(
            resolver.pending_initializations.contains_key(&peer_key),
            "pending context should be stored"
        );
        let stored = resolver.pending_initializations.get(&peer_key).expect("should exist");
        assert_eq!(stored.client_id, "pending-client");
    }

    #[tokio::test]
    async fn session_binding_rejects_profile_id_mismatch() {
        let resolver = SessionBoundClientContextResolver::new();
        let first = ClientContext {
            client_id: "same-client".to_string(),
            session_id: Some("sess-profile".to_string()),
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };
        resolver
            .bind_session("sess-profile", &first)
            .await
            .expect("first bind should succeed");

        let second = ClientContext {
            client_id: "same-client".to_string(),
            session_id: Some("sess-profile".to_string()),
            profile_id: Some("profile-b".to_string()), // Different profile
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };
        let err = resolver
            .bind_session("sess-profile", &second)
            .await
            .expect_err("should reject profile mismatch");
        assert!(err.to_string().contains("already bound to profile_id"));
    }

    #[tokio::test]
    async fn session_binding_rejects_rules_fingerprint_mismatch() {
        let resolver = SessionBoundClientContextResolver::new();
        let first = ClientContext {
            client_id: "same-client".to_string(),
            session_id: Some("sess-fp".to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-alpha".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };
        resolver
            .bind_session("sess-fp", &first)
            .await
            .expect("first bind should succeed");

        let second = ClientContext {
            client_id: "same-client".to_string(),
            session_id: Some("sess-fp".to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-beta".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedHeader,
            observed_client_info: None,
        };
        let err = resolver
            .bind_session("sess-fp", &second)
            .await
            .expect_err("should reject fingerprint mismatch");
        assert!(err.to_string().contains("already bound to rules_fingerprint"));
    }

    #[tokio::test]
    async fn session_binding_allows_fingerprint_upgrade_from_none() {
        let resolver = SessionBoundClientContextResolver::new();
        let initial = ClientContext {
            client_id: "cursor".to_string(),
            session_id: Some("sess-upgrade".to_string()),
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };
        resolver
            .bind_session("sess-upgrade", &initial)
            .await
            .expect("initial bind should succeed");

        let upgraded = ClientContext {
            client_id: "cursor".to_string(),
            session_id: Some("sess-upgrade".to_string()),
            profile_id: Some("profile-a".to_string()),
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-123".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };
        resolver
            .bind_session("sess-upgrade", &upgraded)
            .await
            .expect("upgrade bind should succeed");

        let binding = resolver
            .session_bindings
            .get("sess-upgrade")
            .expect("binding should exist");
        assert_eq!(binding.rules_fingerprint.as_deref(), Some("fp-123"));
    }

    #[tokio::test]
    async fn session_binding_rejects_fingerprint_downgrade_from_some() {
        let resolver = SessionBoundClientContextResolver::new();
        let initial = ClientContext {
            client_id: "cursor".to_string(),
            session_id: Some("sess-downgrade".to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-original".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };
        resolver
            .bind_session("sess-downgrade", &initial)
            .await
            .expect("initial bind should succeed");

        let downgrade = ClientContext {
            client_id: "cursor".to_string(),
            session_id: Some("sess-downgrade".to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: None,
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };
        let err = resolver
            .bind_session("sess-downgrade", &downgrade)
            .await
            .expect_err("should reject fingerprint downgrade");
        assert!(err.to_string().contains("already bound to rules_fingerprint"));
    }

    #[tokio::test]
    async fn session_binding_refreshes_rules_fingerprint() {
        let resolver = SessionBoundClientContextResolver::new();
        let context = ClientContext {
            client_id: "cursor".to_string(),
            session_id: Some("sess-refresh".to_string()),
            profile_id: None,
            config_mode: Some("hosted".to_string()),
            unify_workspace: None,
            rules_fingerprint: Some("fp-old".to_string()),
            transport: ClientTransport::StreamableHttp,
            source: ClientIdentitySource::ManagedQuery,
            observed_client_info: None,
        };

        resolver
            .bind_session("sess-refresh", &context)
            .await
            .expect("bind should succeed");
        resolver
            .refresh_session_rules_fingerprint("sess-refresh", "fp-new".to_string())
            .await
            .expect("refresh should succeed");

        let binding = resolver
            .session_bindings
            .get("sess-refresh")
            .expect("binding should exist");
        assert_eq!(binding.rules_fingerprint.as_deref(), Some("fp-new"));
    }
}
