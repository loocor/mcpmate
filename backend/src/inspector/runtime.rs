use std::collections::HashSet;
use std::time::Duration;

use rmcp::model::{
    ClientRequest, GetPromptRequestParams, GetPromptResult, ListTasksRequest, PaginatedRequestParams,
    ReadResourceRequestParams, ReadResourceResult, Resource, ResourceTemplate, ServerResult, Task, Tool,
};
use rmcp::service::{Peer, RoleClient};
use serde_json::Value;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::api::handlers::ApiError;
use crate::clients::models::{
    UnifyDirectExposureConfig, UnifyDirectPromptSurface, UnifyDirectResourceSurface, UnifyDirectTemplateSurface,
    UnifyDirectToolSurface, UnifyRouteMode,
};
use crate::common::server::ServerType;
use crate::config::database::Database;
use crate::core::models::MCPServerConfig;
use crate::core::profile::visibility::RuntimeSurfaceOverride;
use crate::core::secrets::resolve_runtime_server_config_with_optional_resolver;
use crate::core::secrets::store::LocalSecretStore;
use crate::core::transport::ClientService;
use crate::inspector::contract::{InspectorProxyMode, InspectorProxyScope};
use crate::inspector::target::{InspectorNativeTarget, InspectorProxyTarget, InspectorTarget};
use crate::inspector::workspace::InspectorWorkspace;

pub struct InspectorRuntimeOwner {
    service: ClientService,
    runtime_surface_guard: Option<RuntimeSurfaceGuard>,
}

#[derive(Clone, Copy)]
pub struct InspectorRuntimeAdapter<'a> {
    pub database: Option<&'a Database>,
    pub proxy_surface_available: bool,
    pub inspector_workspace: &'a InspectorWorkspace,
    pub secret_store: &'a tokio::sync::RwLock<Option<std::sync::Arc<LocalSecretStore>>>,
}

pub struct InspectorRuntimeEnvironment<'a> {
    adapter: InspectorRuntimeAdapter<'a>,
}

struct RuntimeSurfaceGuard {
    client_id: String,
}

impl RuntimeSurfaceGuard {
    fn install(
        client_id: String,
        override_config: RuntimeSurfaceOverride,
    ) -> Self {
        crate::core::profile::visibility::upsert_runtime_surface_override(client_id.clone(), override_config);
        Self { client_id }
    }
}

impl Drop for RuntimeSurfaceGuard {
    fn drop(&mut self) {
        crate::core::profile::visibility::remove_runtime_surface_override(&self.client_id);
    }
}

impl InspectorRuntimeOwner {
    fn native(service: ClientService) -> Self {
        Self {
            service,
            runtime_surface_guard: None,
        }
    }

    fn proxy(
        service: ClientService,
        runtime_surface_guard: RuntimeSurfaceGuard,
    ) -> Self {
        Self {
            service,
            runtime_surface_guard: Some(runtime_surface_guard),
        }
    }

    pub async fn cancel(self) {
        let releases_runtime_surface = self.runtime_surface_guard.is_some();

        match timeout(Duration::from_secs(3), self.service.cancel()).await {
            Ok(Ok(reason)) => tracing::debug!(?reason, "Inspector runtime cancelled"),
            Ok(Err(error)) => tracing::warn!(%error, "Failed to cancel Inspector runtime"),
            Err(_) => tracing::warn!("Timed out cancelling Inspector runtime"),
        }

        if releases_runtime_surface {
            tracing::debug!("Inspector runtime surface override released");
        }
    }

    pub async fn cancel_optional(owner: Option<Self>) {
        if let Some(owner) = owner {
            owner.cancel().await;
        }
    }

    pub async fn cancel_taken(owner: &mut Option<Self>) {
        if let Some(owner) = owner.take() {
            owner.cancel().await;
        }
    }
}

impl<'a> InspectorRuntimeEnvironment<'a> {
    pub fn new(adapter: InspectorRuntimeAdapter<'a>) -> Self {
        Self { adapter }
    }

    pub async fn connect_target(
        &self,
        target: &InspectorTarget,
    ) -> Result<InspectorConnectedRuntime, ApiError> {
        match target {
            InspectorTarget::Native(native_target) => self.connect_native_target(native_target).await,
            InspectorTarget::Proxy(proxy_target) => self.connect_proxy_target(proxy_target).await,
        }
    }

    pub async fn acquire_target_peer(
        &self,
        target: &InspectorTarget,
        session_peer: Option<Peer<RoleClient>>,
    ) -> Result<InspectorAcquiredPeer, ApiError> {
        if session_peer.is_some() {
            return InspectorAcquiredPeer::session_for_target(target, session_peer);
        }

        self.connect_target(target).await.map(Into::into)
    }

    pub async fn connect_native_target(
        &self,
        native_target: &InspectorNativeTarget,
    ) -> Result<InspectorConnectedRuntime, ApiError> {
        let runtime = DirectNativeRuntime::connect_target(self, native_target).await?;
        Ok(InspectorConnectedRuntime {
            peer: runtime.peer,
            owner: runtime.owner,
        })
    }

    pub async fn connect_proxy_target(
        &self,
        proxy_target: &InspectorProxyTarget,
    ) -> Result<InspectorConnectedRuntime, ApiError> {
        let runtime = DirectProxyRuntime::connect(self, proxy_target.to_surface()).await?;
        Ok(InspectorConnectedRuntime {
            peer: runtime.peer,
            owner: runtime.owner,
        })
    }

    fn database(&self) -> Result<&crate::config::database::Database, ApiError> {
        self.adapter
            .database
            .ok_or(ApiError::InternalError("Database not available".into()))
    }

    fn proxy_surface_available(&self) -> bool {
        self.adapter.proxy_surface_available
    }

    async fn resolve_runtime_config(
        &self,
        raw_config: &MCPServerConfig,
    ) -> Result<MCPServerConfig, ApiError> {
        let secret_store = self.adapter.secret_store.read().await;
        let resolver = secret_store
            .as_deref()
            .map(|store| store as &dyn mcpmate_secrets::SecretResolver);
        resolve_runtime_server_config_with_optional_resolver(raw_config, resolver)
            .map_err(|err| ApiError::InternalError(format!("Failed to resolve runtime secrets: {}", err)))
    }

    async fn resolve_managed_target_config(
        &self,
        server_id: &str,
    ) -> Result<ManagedTargetConfig, ApiError> {
        let database = self.database()?;
        let server = crate::config::server::get_server_by_id(&database.pool, server_id)
            .await
            .map_err(map_anyhow)?
            .ok_or_else(|| ApiError::NotFound(format!("Server '{}' not found", server_id)))?;

        let config = crate::core::secrets::mcp_config_from_server(&database.pool, server_id, &server)
            .await
            .map_err(map_anyhow)?;

        Ok(ManagedTargetConfig {
            server_name: server.name,
            config,
        })
    }

    async fn resolve_managed_runtime_config(
        &self,
        server_id: &str,
    ) -> Result<ManagedRuntimeConfig, ApiError> {
        let target_config = self.resolve_managed_target_config(server_id).await?;
        let runtime_config = self.resolve_runtime_config(&target_config.config).await?;

        Ok(ManagedRuntimeConfig {
            server_id: server_id.to_string(),
            server_name: target_config.server_name,
            runtime_config,
        })
    }

    pub async fn native_target_config(
        &self,
        native_target: &InspectorNativeTarget,
    ) -> Result<InspectorNativeTargetConfig, ApiError> {
        let (config, source) = match native_target {
            InspectorNativeTarget::Managed { server_id } => (
                self.resolve_managed_target_config(server_id).await?.config,
                "managed_registry",
            ),
            InspectorNativeTarget::Scratch { record_id } => {
                let record = self
                    .inspector_workspace()
                    .get_server_record(record_id)
                    .map_err(map_anyhow)?
                    .ok_or_else(|| ApiError::NotFound(format!("Inspector scratch server '{}' not found", record_id)))?;
                (record.config, "scratch_workspace")
            }
        };

        Ok(InspectorNativeTargetConfig { config, source })
    }

    fn inspector_workspace(&self) -> &crate::inspector::workspace::InspectorWorkspace {
        self.adapter.inspector_workspace
    }

    async fn load_unify_direct_exposure_rows(&self) -> Result<UnifyDirectExposureRows, ApiError> {
        let database = self.database()?;
        let server_rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT sc.id
            FROM server_config sc
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id
            "#,
        )
        .fetch_all(&database.pool)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
        let tool_rows: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT sc.id, st.tool_name
            FROM server_config sc
            INNER JOIN server_tools st ON st.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, st.tool_name
            "#,
        )
        .fetch_all(&database.pool)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
        let prompt_rows: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT sc.id, sp.prompt_name
            FROM server_config sc
            INNER JOIN server_prompts sp ON sp.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, sp.prompt_name
            "#,
        )
        .fetch_all(&database.pool)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
        let resource_rows: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT sc.id, sr.resource_uri
            FROM server_config sc
            INNER JOIN server_resources sr ON sr.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, sr.resource_uri
            "#,
        )
        .fetch_all(&database.pool)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
        let template_rows: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT sc.id, srt.uri_template
            FROM server_config sc
            INNER JOIN server_resource_templates srt ON srt.server_id = sc.id
            WHERE sc.enabled = 1 AND sc.unify_direct_exposure_eligible = 1
            ORDER BY sc.id, srt.uri_template
            "#,
        )
        .fetch_all(&database.pool)
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;

        Ok(UnifyDirectExposureRows {
            server_ids: server_rows.into_iter().map(|(server_id,)| server_id).collect(),
            tool_rows,
            prompt_rows,
            resource_rows,
            template_rows,
        })
    }
}

struct DirectNativeRuntime {
    peer: Peer<RoleClient>,
    owner: InspectorRuntimeOwner,
}

struct DirectProxyRuntime {
    peer: Peer<RoleClient>,
    owner: InspectorRuntimeOwner,
}

struct ManagedRuntimeConfig {
    server_id: String,
    server_name: String,
    runtime_config: MCPServerConfig,
}

struct ManagedTargetConfig {
    server_name: String,
    config: MCPServerConfig,
}

pub struct InspectorConnectedRuntime {
    pub peer: Peer<RoleClient>,
    pub owner: InspectorRuntimeOwner,
}

pub struct InspectorNativeTargetConfig {
    pub config: MCPServerConfig,
    pub source: &'static str,
}

pub struct InspectorAcquiredPeer {
    pub peer: Peer<RoleClient>,
    pub runtime_owner: Option<InspectorRuntimeOwner>,
}

impl InspectorAcquiredPeer {
    pub fn session(peer: Peer<RoleClient>) -> Self {
        Self {
            peer,
            runtime_owner: None,
        }
    }

    pub fn session_for_target(
        target: &InspectorTarget,
        peer: Option<Peer<RoleClient>>,
    ) -> Result<Self, ApiError> {
        peer.map(Self::session)
            .ok_or_else(|| ApiError::InternalError(session_peer_missing_message(target).into()))
    }

    pub fn peer(&self) -> &Peer<RoleClient> {
        &self.peer
    }

    pub fn into_parts(self) -> (Peer<RoleClient>, Option<InspectorRuntimeOwner>) {
        (self.peer, self.runtime_owner)
    }

    pub async fn cancel_runtime(self) {
        InspectorRuntimeOwner::cancel_optional(self.runtime_owner).await;
    }
}

fn session_peer_missing_message(target: &InspectorTarget) -> &'static str {
    match target {
        InspectorTarget::Native(_) => "Native Inspector session peer is not available",
        InspectorTarget::Proxy(_) => "Inspector session peer is not available",
    }
}

impl From<InspectorConnectedRuntime> for InspectorAcquiredPeer {
    fn from(runtime: InspectorConnectedRuntime) -> Self {
        Self {
            peer: runtime.peer,
            runtime_owner: Some(runtime.owner),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProxyRuntimeSurface {
    pub proxy_mode: InspectorProxyMode,
    pub proxy_scope: InspectorProxyScope,
    pub target_server_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
struct UnifyDirectExposureRows {
    server_ids: Vec<String>,
    tool_rows: Vec<(String, String)>,
    prompt_rows: Vec<(String, String)>,
    resource_rows: Vec<(String, String)>,
    template_rows: Vec<(String, String)>,
}

impl ProxyRuntimeSurface {
    fn config_mode(&self) -> &'static str {
        match self.proxy_mode {
            InspectorProxyMode::Hosted => "hosted",
            InspectorProxyMode::Unify => "unify",
        }
    }

    fn override_server_ids(&self) -> Option<Vec<String>> {
        match self.proxy_scope {
            InspectorProxyScope::Isolated => self.target_server_ids.clone(),
            InspectorProxyScope::ActiveCatalog => None,
        }
    }

    fn to_runtime_override(
        &self,
        unify_workspace: Option<UnifyDirectExposureConfig>,
    ) -> RuntimeSurfaceOverride {
        RuntimeSurfaceOverride {
            config_mode: Some(self.config_mode().to_string()),
            server_ids: self.override_server_ids(),
            unify_workspace,
        }
    }
}

impl DirectNativeRuntime {
    pub async fn connect_target(
        env: &InspectorRuntimeEnvironment<'_>,
        target: &InspectorNativeTarget,
    ) -> Result<Self, ApiError> {
        match target {
            InspectorNativeTarget::Managed { server_id } => Self::connect(env, server_id).await,
            InspectorNativeTarget::Scratch { record_id } => Self::connect_scratch(env, record_id).await,
        }
    }

    pub async fn connect(
        env: &InspectorRuntimeEnvironment<'_>,
        server_id: &str,
    ) -> Result<Self, ApiError> {
        let config = env.resolve_managed_runtime_config(server_id).await?;
        let database = env.database()?;

        connect_runtime(
            &config.server_id,
            &config.server_name,
            &config.runtime_config,
            Some(&database.pool),
        )
        .await
    }

    pub async fn connect_scratch(
        env: &InspectorRuntimeEnvironment<'_>,
        record_id: &str,
    ) -> Result<Self, ApiError> {
        let record = env
            .inspector_workspace()
            .get_server_record(record_id)
            .map_err(map_anyhow)?
            .ok_or_else(|| ApiError::NotFound(format!("Inspector scratch server '{}' not found", record_id)))?;
        let runtime_config = env.resolve_runtime_config(&record.config).await?;

        connect_runtime(&record.id, &record.name, &runtime_config, None).await
    }
}

impl DirectProxyRuntime {
    pub async fn connect(
        env: &InspectorRuntimeEnvironment<'_>,
        surface: ProxyRuntimeSurface,
    ) -> Result<Self, ApiError> {
        if !env.proxy_surface_available() {
            return Err(ApiError::Conflict(
                "MCPMate proxy surface is not running; Proxy inspector mode requires the local /mcp endpoint".into(),
            ));
        }
        let client_id = crate::generate_id!("insppxy");
        let unify_workspace = match surface.proxy_mode {
            InspectorProxyMode::Hosted => None,
            InspectorProxyMode::Unify => Some(build_unify_workspace(env, &surface).await?),
        };
        let runtime_surface_guard =
            RuntimeSurfaceGuard::install(client_id.clone(), surface.to_runtime_override(unify_workspace));

        let runtime_config = proxy_runtime_config(&client_id)?;
        match connect_proxy_runtime(&client_id, &runtime_config, runtime_surface_guard).await {
            Ok(runtime) => Ok(runtime),
            Err(error) => Err(error),
        }
    }
}

pub async fn list_tools(peer: &Peer<RoleClient>) -> Result<Vec<Tool>, ApiError> {
    let mut cursor = None;
    let mut tools = Vec::new();
    loop {
        let result = peer
            .list_tools(Some(PaginatedRequestParams::default().with_cursor(cursor)))
            .await
            .map_err(map_service)?;
        tools.extend(result.tools);
        cursor = result.next_cursor;
        if cursor.is_none() {
            return Ok(tools);
        }
    }
}

pub async fn list_prompts(peer: &Peer<RoleClient>) -> Result<Vec<rmcp::model::Prompt>, ApiError> {
    let mut cursor = None;
    let mut prompts = Vec::new();
    loop {
        let result = peer
            .list_prompts(Some(PaginatedRequestParams::default().with_cursor(cursor)))
            .await
            .map_err(map_service)?;
        prompts.extend(result.prompts);
        cursor = result.next_cursor;
        if cursor.is_none() {
            return Ok(prompts);
        }
    }
}

pub async fn list_resources(peer: &Peer<RoleClient>) -> Result<Vec<Resource>, ApiError> {
    let mut cursor = None;
    let mut resources = Vec::new();
    loop {
        let result = peer
            .list_resources(Some(PaginatedRequestParams::default().with_cursor(cursor)))
            .await
            .map_err(map_service)?;
        resources.extend(result.resources);
        cursor = result.next_cursor;
        if cursor.is_none() {
            return Ok(resources);
        }
    }
}

pub async fn list_resource_templates(peer: &Peer<RoleClient>) -> Result<Vec<ResourceTemplate>, ApiError> {
    let mut cursor = None;
    let mut templates = Vec::new();
    loop {
        let result = peer
            .list_resource_templates(Some(PaginatedRequestParams::default().with_cursor(cursor)))
            .await
            .map_err(map_service)?;
        templates.extend(result.resource_templates);
        cursor = result.next_cursor;
        if cursor.is_none() {
            return Ok(templates);
        }
    }
}

pub async fn list_tasks(peer: &Peer<RoleClient>) -> Result<Vec<Task>, ApiError> {
    let mut cursor = None;
    let mut tasks = Vec::new();
    loop {
        let result = peer
            .send_request(ClientRequest::ListTasksRequest(ListTasksRequest {
                method: Default::default(),
                params: Some(PaginatedRequestParams::default().with_cursor(cursor)),
                extensions: Default::default(),
            }))
            .await
            .map_err(map_service)?;
        let ServerResult::ListTasksResult(result) = result else {
            return Err(ApiError::InternalError(
                "Unexpected response for Inspector tasks/list request".into(),
            ));
        };
        tasks.extend(result.tasks);
        cursor = result.next_cursor;
        if cursor.is_none() {
            return Ok(tasks);
        }
    }
}

pub async fn get_prompt(
    peer: &Peer<RoleClient>,
    name: &str,
    arguments: Option<serde_json::Map<String, Value>>,
) -> Result<GetPromptResult, ApiError> {
    let mut params = GetPromptRequestParams::new(name.to_string());
    if let Some(arguments) = arguments {
        params = params.with_arguments(arguments);
    }
    peer.get_prompt(params).await.map_err(map_service)
}

pub async fn read_resource(
    peer: &Peer<RoleClient>,
    uri: &str,
) -> Result<ReadResourceResult, ApiError> {
    peer.read_resource(ReadResourceRequestParams::new(uri.to_string()))
        .await
        .map_err(map_service)
}

pub async fn cancel_runtime_owner(owner: InspectorRuntimeOwner) {
    owner.cancel().await;
}

async fn connect_runtime(
    server_id: &str,
    server_name: &str,
    runtime_config: &MCPServerConfig,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
) -> Result<DirectNativeRuntime, ApiError> {
    let transport_type = runtime_config.get_transport_type();
    let label = format!("{} ({})", server_name, server_id);
    let service = match runtime_config.kind {
        ServerType::Stdio => {
            let (service, _capabilities, _pid) = crate::core::transport::stdio::connect_stdio_server_no_probe(
                &label,
                runtime_config,
                CancellationToken::new(),
                database_pool,
            )
            .await
            .map_err(map_anyhow)?;
            service
        }
        ServerType::Sse | ServerType::StreamableHttp => {
            let (service, _capabilities) =
                crate::core::transport::connect_http_server_no_probe(&label, runtime_config, transport_type)
                    .await
                    .map_err(map_anyhow)?;
            service
        }
    };
    let peer = service.peer().clone();
    Ok(DirectNativeRuntime {
        peer,
        owner: InspectorRuntimeOwner::native(service),
    })
}

fn proxy_runtime_config(client_id: &str) -> Result<MCPServerConfig, ApiError> {
    let mut url = url::Url::parse(&crate::system::config::get_runtime_port_config().mcp_http_url())
        .map_err(|error| ApiError::InternalError(format!("Invalid local MCP proxy URL: {}", error)))?;
    url.query_pairs_mut().append_pair("client_id", client_id);
    Ok(MCPServerConfig {
        kind: ServerType::StreamableHttp,
        command: None,
        args: None,
        url: Some(url.to_string()),
        env: None,
        headers: None,
    })
}

async fn connect_proxy_runtime(
    client_id: &str,
    runtime_config: &MCPServerConfig,
    runtime_surface_guard: RuntimeSurfaceGuard,
) -> Result<DirectProxyRuntime, ApiError> {
    let transport_type = runtime_config.get_transport_type();
    let label = format!("MCPMate Inspector Proxy ({})", client_id);
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|error| ApiError::InternalError(format!("Failed to build local proxy HTTP client: {}", error)))?;
    let (service, _capabilities) = crate::core::transport::connect_http_server_with_client_no_probe(
        &label,
        runtime_config,
        client,
        transport_type,
    )
    .await
    .map_err(map_anyhow)?;
    let peer = service.peer().clone();
    Ok(DirectProxyRuntime {
        peer,
        owner: InspectorRuntimeOwner::proxy(service, runtime_surface_guard),
    })
}

async fn build_unify_workspace(
    env: &InspectorRuntimeEnvironment<'_>,
    surface: &ProxyRuntimeSurface,
) -> Result<UnifyDirectExposureConfig, ApiError> {
    let rows = env.load_unify_direct_exposure_rows().await?;
    build_unify_workspace_from_rows(surface, rows)
}

fn build_unify_workspace_from_rows(
    surface: &ProxyRuntimeSurface,
    rows: UnifyDirectExposureRows,
) -> Result<UnifyDirectExposureConfig, ApiError> {
    let target_filter = match surface.proxy_scope {
        InspectorProxyScope::Isolated => surface
            .target_server_ids
            .as_ref()
            .map(|server_ids| server_ids.iter().cloned().collect::<HashSet<_>>()),
        InspectorProxyScope::ActiveCatalog => None,
    };

    let selected_server_ids = filter_targeted_rows(rows.server_ids.into_iter(), &target_filter);

    if matches!(surface.proxy_scope, InspectorProxyScope::Isolated) && selected_server_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "proxy_mode=unify with proxy_scope=isolated requires an enabled direct-exposure eligible server".into(),
        ));
    }

    Ok(UnifyDirectExposureConfig {
        route_mode: UnifyRouteMode::ServerLevel,
        selected_server_ids,
        selected_tool_surfaces: filter_targeted_pairs(rows.tool_rows, &target_filter)
            .into_iter()
            .map(|(server_id, tool_name)| UnifyDirectToolSurface { server_id, tool_name })
            .collect(),
        selected_prompt_surfaces: filter_targeted_pairs(rows.prompt_rows, &target_filter)
            .into_iter()
            .map(|(server_id, prompt_name)| UnifyDirectPromptSurface { server_id, prompt_name })
            .collect(),
        selected_resource_surfaces: filter_targeted_pairs(rows.resource_rows, &target_filter)
            .into_iter()
            .map(|(server_id, resource_uri)| UnifyDirectResourceSurface {
                server_id,
                resource_uri,
            })
            .collect(),
        selected_template_surfaces: filter_targeted_pairs(rows.template_rows, &target_filter)
            .into_iter()
            .map(|(server_id, uri_template)| UnifyDirectTemplateSurface {
                server_id,
                uri_template,
            })
            .collect(),
    })
}

fn filter_targeted_rows(
    rows: impl Iterator<Item = String>,
    target_filter: &Option<HashSet<String>>,
) -> Vec<String> {
    rows.filter(|server_id| target_filter.as_ref().is_none_or(|targets| targets.contains(server_id)))
        .collect()
}

fn filter_targeted_pairs(
    rows: Vec<(String, String)>,
    target_filter: &Option<HashSet<String>>,
) -> Vec<(String, String)> {
    rows.into_iter()
        .filter(|(server_id, _)| target_filter.as_ref().is_none_or(|targets| targets.contains(server_id)))
        .collect()
}

fn map_service(error: rmcp::service::ServiceError) -> ApiError {
    ApiError::InternalError(error.to_string())
}

fn map_anyhow(error: anyhow::Error) -> ApiError {
    ApiError::InternalError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_surface_builds_hosted_isolated_override() {
        let surface = ProxyRuntimeSurface {
            proxy_mode: InspectorProxyMode::Hosted,
            proxy_scope: InspectorProxyScope::Isolated,
            target_server_ids: Some(vec!["server-b".to_string(), "server-a".to_string()]),
        };

        let override_config = surface.to_runtime_override(None);

        assert_eq!(override_config.config_mode.as_deref(), Some("hosted"));
        assert_eq!(
            override_config.server_ids,
            Some(vec!["server-b".to_string(), "server-a".to_string()])
        );
        assert!(override_config.unify_workspace.is_none());
    }

    #[test]
    fn proxy_surface_drops_active_catalog_server_filter() {
        let surface = ProxyRuntimeSurface {
            proxy_mode: InspectorProxyMode::Unify,
            proxy_scope: InspectorProxyScope::ActiveCatalog,
            target_server_ids: Some(vec!["server-a".to_string()]),
        };
        let unify_workspace = UnifyDirectExposureConfig {
            route_mode: UnifyRouteMode::ServerLevel,
            selected_server_ids: vec!["server-a".to_string()],
            selected_tool_surfaces: Vec::new(),
            selected_prompt_surfaces: Vec::new(),
            selected_resource_surfaces: Vec::new(),
            selected_template_surfaces: Vec::new(),
        };

        let override_config = surface.to_runtime_override(Some(unify_workspace.clone()));

        assert_eq!(override_config.config_mode.as_deref(), Some("unify"));
        assert_eq!(override_config.server_ids, None);
        assert_eq!(override_config.unify_workspace, Some(unify_workspace));
    }

    #[test]
    fn unify_workspace_filters_isolated_target_rows() {
        let surface = ProxyRuntimeSurface {
            proxy_mode: InspectorProxyMode::Unify,
            proxy_scope: InspectorProxyScope::Isolated,
            target_server_ids: Some(vec!["server-a".to_string()]),
        };
        let rows = UnifyDirectExposureRows {
            server_ids: vec!["server-a".to_string(), "server-b".to_string()],
            tool_rows: vec![
                ("server-a".to_string(), "tool_a".to_string()),
                ("server-b".to_string(), "tool_b".to_string()),
            ],
            prompt_rows: vec![("server-a".to_string(), "prompt_a".to_string())],
            resource_rows: vec![("server-b".to_string(), "resource_b".to_string())],
            template_rows: vec![("server-a".to_string(), "template_a".to_string())],
        };

        let config = build_unify_workspace_from_rows(&surface, rows).expect("workspace config");

        assert_eq!(config.selected_server_ids, vec!["server-a".to_string()]);
        assert_eq!(config.selected_tool_surfaces.len(), 1);
        assert_eq!(config.selected_tool_surfaces[0].server_id, "server-a");
        assert_eq!(config.selected_prompt_surfaces.len(), 1);
        assert!(config.selected_resource_surfaces.is_empty());
        assert_eq!(config.selected_template_surfaces.len(), 1);
    }

    #[test]
    fn unify_workspace_rejects_empty_isolated_target_rows() {
        let surface = ProxyRuntimeSurface {
            proxy_mode: InspectorProxyMode::Unify,
            proxy_scope: InspectorProxyScope::Isolated,
            target_server_ids: Some(vec!["server-missing".to_string()]),
        };
        let rows = UnifyDirectExposureRows {
            server_ids: vec!["server-a".to_string()],
            ..Default::default()
        };

        let result = build_unify_workspace_from_rows(&surface, rows);

        match result {
            Err(ApiError::BadRequest(message)) => {
                assert_eq!(
                    message,
                    "proxy_mode=unify with proxy_scope=isolated requires an enabled direct-exposure eligible server"
                );
            }
            Err(error) => panic!("unexpected error: {:?}", error),
            Ok(_) => panic!("missing isolated target should fail"),
        }
    }

    #[test]
    fn runtime_surface_guard_releases_override_on_drop() {
        let client_id = crate::generate_id!("testpxy");
        crate::core::profile::visibility::remove_runtime_surface_override(&client_id);

        let guard = RuntimeSurfaceGuard::install(
            client_id.clone(),
            crate::core::profile::visibility::RuntimeSurfaceOverride {
                config_mode: Some("hosted".to_string()),
                server_ids: Some(vec!["server-b".to_string(), "server-a".to_string()]),
                unify_workspace: None,
            },
        );

        let override_config = crate::core::profile::visibility::runtime_surface_override(&client_id)
            .expect("override should exist while guard is alive");
        assert_eq!(override_config.config_mode.as_deref(), Some("hosted"));
        assert_eq!(
            override_config.server_ids,
            Some(vec!["server-a".to_string(), "server-b".to_string()])
        );

        drop(guard);

        assert!(
            crate::core::profile::visibility::runtime_surface_override(&client_id).is_none(),
            "override should be removed when guard drops"
        );
    }

    #[test]
    fn session_peer_guard_reports_missing_native_peer() {
        let target = InspectorTarget::native("server-a".to_string());
        let result = InspectorAcquiredPeer::session_for_target(&target, None);

        match result {
            Err(ApiError::InternalError(message)) => {
                assert_eq!(message, "Native Inspector session peer is not available");
            }
            Err(error) => panic!("unexpected error: {:?}", error),
            Ok(_) => panic!("missing peer should fail"),
        }
    }

    #[test]
    fn session_peer_guard_reports_missing_proxy_peer() {
        let target = InspectorTarget::proxy(
            InspectorProxyTarget::from_parts(None, None, None).expect("active catalog proxy target"),
        );
        let result = InspectorAcquiredPeer::session_for_target(&target, None);

        match result {
            Err(ApiError::InternalError(message)) => {
                assert_eq!(message, "Inspector session peer is not available");
            }
            Err(error) => panic!("unexpected error: {:?}", error),
            Ok(_) => panic!("missing peer should fail"),
        }
    }
}
