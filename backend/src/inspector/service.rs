use anyhow::anyhow;
use futures::StreamExt;
use rmcp::RoleClient;
use rmcp::model::{
    CallToolRequest, CallToolRequestParams, ClientRequest, LoggingMessageNotificationParam, ProgressNotificationParam,
    ProgressToken, RequestId,
};
use rmcp::service::{Peer, PeerRequestOptions};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::future::Future;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Instant;
use tokio::sync::oneshot;
use tokio::time::Duration;

use crate::api::handlers::ApiError;
use crate::api::models::inspector::{
    InspectorListQuery, InspectorMode, InspectorPromptGetReq, InspectorResourceReadQuery, InspectorSessionCloseReq,
    InspectorSessionOpenData, InspectorSessionOpenReq, InspectorToolCallReq,
};
use crate::api::routes::AppState;
use crate::core::capability;
use crate::core::capability::resolver;
use crate::core::proxy::server::supports_capability;
use crate::inspector::calls::{InspectorCallInfo, InspectorCallRegistry, InspectorTerminal, RegisteredCall};

const NATIVE_VALIDATION_SESSION_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone)]
pub struct ToolCallOutcome {
    pub result: Option<Value>,
    pub server_id: Option<String>,
    pub elapsed_ms: u64,
    pub message: Option<String>,
}

pub struct PreparedCall {
    pub info: InspectorCallInfo,
    pub completion: oneshot::Receiver<InspectorTerminal>,
    pub server_id: String,
    native_validation_session: Option<NativeValidationSessionGuard>,
}

static GLOBAL_CALL_REGISTRY: OnceLock<Arc<InspectorCallRegistry>> = OnceLock::new();

pub fn set_call_registry(registry: Arc<InspectorCallRegistry>) {
    let _ = GLOBAL_CALL_REGISTRY.set(registry);
}

#[derive(Clone, Copy)]
enum CapabilityKind {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

impl CapabilityKind {
    fn capability_type(self) -> capability::CapabilityType {
        match self {
            CapabilityKind::Tools => capability::CapabilityType::Tools,
            CapabilityKind::Prompts => capability::CapabilityType::Prompts,
            CapabilityKind::Resources => capability::CapabilityType::Resources,
            CapabilityKind::ResourceTemplates => capability::CapabilityType::ResourceTemplates,
        }
    }

    fn response_key(self) -> &'static str {
        match self {
            CapabilityKind::Tools => "tools",
            CapabilityKind::Prompts => "prompts",
            CapabilityKind::Resources => "resources",
            CapabilityKind::ResourceTemplates => "templates",
        }
    }

    fn label(self) -> &'static str {
        match self {
            CapabilityKind::Tools => "tools",
            CapabilityKind::Prompts => "prompts",
            CapabilityKind::Resources => "resources",
            CapabilityKind::ResourceTemplates => "resource templates",
        }
    }

    fn extractor(self) -> fn(capability::runtime::ListResult) -> Vec<Value> {
        match self {
            CapabilityKind::Tools => extract_tools,
            CapabilityKind::Prompts => extract_prompts,
            CapabilityKind::Resources => extract_resources,
            CapabilityKind::ResourceTemplates => extract_resource_templates,
        }
    }
}

struct CapabilityPayload {
    mode: String,
    items: Vec<Value>,
    meta: Vec<Value>,
}

struct NativeValidationSessionGuard {
    connection_pool: Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
    session_id: Option<String>,
}

impl NativeValidationSessionGuard {
    fn new(
        state: &AppState,
        session_id: String,
    ) -> Self {
        Self {
            connection_pool: state.connection_pool.clone(),
            session_id: Some(session_id),
        }
    }

    fn session_id(&self) -> &str {
        self.session_id
            .as_deref()
            .expect("native validation cleanup guard must own a session")
    }

    async fn cleanup(mut self) {
        if let Some(session_id) = self.session_id.clone() {
            destroy_validation_session(&self.connection_pool, &session_id).await;
            self.session_id.take();
        }
    }
}

impl Drop for NativeValidationSessionGuard {
    fn drop(&mut self) {
        let Some(session_id) = self.session_id.take() else {
            return;
        };
        let connection_pool = self.connection_pool.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                destroy_validation_session(&connection_pool, &session_id).await;
            });
        }
    }
}

pub async fn list_tools(
    state: &AppState,
    query: &InspectorListQuery,
) -> Result<Value, ApiError> {
    list_capability_response(state, query, CapabilityKind::Tools).await
}

pub async fn list_prompts(
    state: &AppState,
    query: &InspectorListQuery,
) -> Result<Value, ApiError> {
    list_capability_response(state, query, CapabilityKind::Prompts).await
}

pub async fn list_resources(
    state: &AppState,
    query: &InspectorListQuery,
) -> Result<Value, ApiError> {
    list_capability_response(state, query, CapabilityKind::Resources).await
}

pub async fn list_templates(
    state: &AppState,
    query: &InspectorListQuery,
) -> Result<Value, ApiError> {
    list_capability_response(state, query, CapabilityKind::ResourceTemplates).await
}

pub async fn prompt_get(
    state: &AppState,
    req: &InspectorPromptGetReq,
) -> Result<Value, ApiError> {
    let timeout = inspector_timeout(state, req.timeout_ms).await;
    let start = Instant::now();
    match req.mode {
        InspectorMode::Proxy => {
            let route = capability::naming::resolve_capability_route(capability::naming::NamingKind::Prompt, &req.name)
                .await
                .map_err(map_anyhow)?;
            let server_id = route.server_id;
            let upstream_name = route.upstream_value;
            let server_filter = HashSet::from([server_id.clone()]);

            ensure_proxy_connection(state, &server_id, timeout).await?;

            let res = run_inspector_operation(timeout, "prompts/get", async {
                let mapping =
                    capability::facade::build_prompt_mapping_filtered(&state.connection_pool, Some(&server_filter))
                        .await
                        .map_err(map_anyhow)?;
                capability::facade::get_upstream_prompt(
                    &state.connection_pool,
                    &mapping,
                    &upstream_name,
                    req.arguments.clone(),
                    Some(&server_id),
                    None,
                )
                .await
                .map_err(map_anyhow)
            })
            .await?;

            Ok(json!({
                "result": res,
                "server_id": server_id,
                "elapsed_ms": start.elapsed().as_millis() as u64,
            }))
        }
        InspectorMode::Native => {
            ensure_native_allowed()?;
            let server_id = resolve_server(&req.server_id, &req.server_name).await?;
            let (session_id, cleanup) =
                native_validation_scope(state, &server_id, req.session_id.as_deref(), timeout).await?;
            let res = run_inspector_operation(timeout, "prompts/get", async {
                let conn = clone_native_validation_connection(state, &server_id, &session_id).await?;
                crate::core::capability::prompts::get_upstream_prompt_direct(&conn, &req.name, req.arguments.clone())
                    .await
                    .map_err(map_anyhow)
            })
            .await;
            if let Some(cleanup) = cleanup {
                cleanup.cleanup().await;
            }
            let res = res?;
            Ok(json!({
                "result": res,
                "server_id": server_id,
                "elapsed_ms": start.elapsed().as_millis() as u64,
            }))
        }
    }
}

pub async fn resource_read(
    state: &AppState,
    req: &InspectorResourceReadQuery,
) -> Result<Value, ApiError> {
    let timeout = inspector_timeout(state, req.timeout_ms).await;
    let start = Instant::now();
    match req.mode {
        InspectorMode::Proxy => {
            let route =
                capability::naming::resolve_capability_route(capability::naming::NamingKind::Resource, &req.uri)
                    .await
                    .map_err(map_anyhow)?;
            let server_filter = Some(route.server_id);
            let upstream_uri = route.upstream_value;

            ensure_proxy_connection(
                state,
                server_filter
                    .as_deref()
                    .expect("resolved resource route has a server id"),
                timeout,
            )
            .await?;

            let res = run_inspector_operation(timeout, "resources/read", async {
                let mapping = if let Some(sid) = &server_filter {
                    let mut filter: HashSet<String> = HashSet::new();
                    filter.insert(sid.clone());
                    capability::facade::build_resource_mapping_filtered(
                        &state.connection_pool,
                        state.database.as_ref(),
                        Some(&filter),
                    )
                    .await
                    .map_err(map_anyhow)?
                } else {
                    capability::facade::build_resource_mapping(&state.connection_pool, state.database.as_ref())
                        .await
                        .map_err(map_anyhow)?
                };
                capability::facade::read_upstream_resource(
                    &state.connection_pool,
                    &mapping,
                    &upstream_uri,
                    server_filter.as_deref(),
                    None,
                )
                .await
                .map_err(map_anyhow)
            })
            .await?;
            Ok(json!({
                "result": res,
                "server_id": server_filter,
                "elapsed_ms": start.elapsed().as_millis() as u64,
            }))
        }
        InspectorMode::Native => {
            ensure_native_allowed()?;
            let server_id = resolve_server(&req.server_id, &req.server_name).await?;
            let (session_id, cleanup) =
                native_validation_scope(state, &server_id, req.session_id.as_deref(), timeout).await?;
            let res = run_inspector_operation(timeout, "resources/read", async {
                let conn = clone_native_validation_connection(state, &server_id, &session_id).await?;
                crate::core::capability::resources::read_upstream_resource_direct(&conn, &req.uri)
                    .await
                    .map_err(map_anyhow)
            })
            .await;
            if let Some(cleanup) = cleanup {
                cleanup.cleanup().await;
            }
            let res = res?;
            Ok(json!({
                "result": res,
                "server_id": server_id,
                "elapsed_ms": start.elapsed().as_millis() as u64,
            }))
        }
    }
}

pub async fn call_tool(
    state: &Arc<AppState>,
    req: &InspectorToolCallReq,
) -> Result<ToolCallOutcome, ApiError> {
    let prepared = start_tool_call_internal(state, req).await?;
    let PreparedCall {
        completion,
        mut native_validation_session,
        ..
    } = prepared;

    let result = match completion.await {
        Ok(result) => result,
        Err(_) => {
            if let Some(cleanup) = native_validation_session.take() {
                cleanup.cleanup().await;
            }
            return Err(ApiError::InternalError("Inspector call channel dropped".into()));
        }
    };

    if let Some(cleanup) = native_validation_session {
        cleanup.cleanup().await;
    }

    match result {
        InspectorTerminal::Result {
            result,
            elapsed_ms,
            server_id,
        } => Ok(ToolCallOutcome {
            result: Some(result),
            server_id: Some(server_id),
            elapsed_ms,
            message: Some("completed".to_string()),
        }),
        InspectorTerminal::Error { message, .. } => Err(ApiError::InternalError(message)),
        InspectorTerminal::Cancelled { reason, .. } => Err(ApiError::Conflict(
            reason.unwrap_or_else(|| "Inspector call cancelled".to_string()),
        )),
    }
}

pub async fn start_tool_call(
    state: &Arc<AppState>,
    req: &InspectorToolCallReq,
) -> Result<InspectorCallInfo, ApiError> {
    let prepared = start_tool_call_internal(state, req).await?;
    let info = prepared.info.clone();

    tokio::spawn(async move {
        let PreparedCall {
            completion,
            native_validation_session,
            ..
        } = prepared;

        let _ = completion.await;

        if let Some(cleanup) = native_validation_session {
            cleanup.cleanup().await;
        }
    });

    Ok(info)
}

pub async fn open_session(
    state: &Arc<AppState>,
    req: &InspectorSessionOpenReq,
) -> Result<InspectorSessionOpenData, ApiError> {
    let timeout = inspector_timeout(state, req.timeout_ms).await;
    if matches!(req.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }

    let server_id = resolve_server(&req.server_id, &req.server_name).await?;
    let session_id = crate::generate_id!("inspses");
    let (peer, validation_session) = if matches!(req.mode, InspectorMode::Native) {
        let validation_session = native_session_id_for_inspector_session(&session_id);
        ensure_native_session_with_timeout(state, &server_id, &validation_session, timeout).await?;
        acquire_validation_peer(state, &server_id, &validation_session)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        (None, Some(validation_session))
    } else {
        ensure_proxy_connection(state, &server_id, timeout).await?;
        let peer = acquire_peer_for_call(state, &server_id, None)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;
        (Some(peer), None)
    };

    let info = state
        .inspector_sessions
        .open_session(
            session_id.clone(),
            server_id.clone(),
            req.mode,
            peer,
            validation_session.clone(),
        )
        .await;

    Ok(InspectorSessionOpenData {
        session_id: info.session_id,
        server_id: info.server_id,
        mode: info.mode,
        expires_at_epoch_ms: info.expires_at_epoch_ms,
    })
}

pub async fn close_session(
    state: &Arc<AppState>,
    req: &InspectorSessionCloseReq,
) -> Result<bool, ApiError> {
    if let Some(closed) = state.inspector_sessions.close_session(&req.session_id).await {
        if matches!(closed.mode, InspectorMode::Native) {
            cleanup_native_session(state, &closed.server_id, closed.validation_session.as_deref()).await;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn get_inspector_timeout_default(state: &AppState) -> Option<u64> {
    let db = state.database.as_ref()?;
    crate::system::settings::get_inspector_timeout_ms(&db.pool).await.ok()
}

async fn inspector_timeout(
    state: &AppState,
    requested_ms: Option<u64>,
) -> Duration {
    let timeout_ms = match requested_ms {
        Some(timeout_ms) => timeout_ms,
        None => get_inspector_timeout_default(state).await.unwrap_or(60_000),
    };
    Duration::from_millis(timeout_ms)
}

async fn run_inspector_operation<T, F>(
    timeout: Duration,
    operation: &'static str,
    future: F,
) -> Result<T, ApiError>
where
    F: Future<Output = Result<T, ApiError>>,
{
    tokio::time::timeout(timeout, future)
        .await
        .map_err(|_| ApiError::Timeout(format!("Inspector {operation} exceeded {} ms", timeout.as_millis())))?
}

async fn ensure_proxy_connection(
    state: &AppState,
    server_id: &str,
    timeout: Duration,
) -> Result<(), ApiError> {
    run_inspector_operation(timeout, "server connect", async {
        let mut pool = state.connection_pool.lock().await;
        pool.ensure_connected(server_id).await.map_err(map_anyhow)?;
        Ok(())
    })
    .await
}

async fn start_tool_call_internal(
    state: &Arc<AppState>,
    req: &InspectorToolCallReq,
) -> Result<PreparedCall, ApiError> {
    let timeout = inspector_timeout(state, req.timeout_ms).await;
    start_tool_call_internal_with_timeout(state, req, timeout).await
}

async fn start_tool_call_internal_with_timeout(
    state: &Arc<AppState>,
    req: &InspectorToolCallReq,
    timeout: Duration,
) -> Result<PreparedCall, ApiError> {
    if matches!(req.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }

    let active_session =
        if let Some(session_id) = req.session_id.as_ref() {
            Some(state.inspector_sessions.get_session(session_id).await.ok_or_else(|| {
                ApiError::NotFound(format!("Inspector session '{}' not found or expired", session_id))
            })?)
        } else {
            None
        };

    let (server_id, upstream_tool_name) = match req.mode {
        InspectorMode::Proxy => {
            let route = capability::naming::resolve_capability_route(capability::naming::NamingKind::Tool, &req.tool)
                .await
                .map_err(map_anyhow)?;
            (route.server_id, route.upstream_value)
        }
        InspectorMode::Native => {
            let sid = if let Some(session) = &active_session {
                session.server_id.clone()
            } else {
                resolve_server(&req.server_id, &req.server_name).await?
            };
            (sid.clone(), req.tool.clone())
        }
    };

    if let Some(session) = &active_session {
        if session.server_id != server_id {
            return Err(ApiError::BadRequest(
                "Inspector session is bound to a different server".into(),
            ));
        }
        if session.mode != req.mode {
            return Err(ApiError::BadRequest(
                "Inspector session is bound to a different mode".into(),
            ));
        }
    }

    let (peer, native_cleanup_session) = match (req.mode, active_session) {
        (InspectorMode::Native, Some(session)) => {
            let validation_session = session.validation_session.as_deref().ok_or_else(|| {
                ApiError::InternalError("Native Inspector session is missing validation ownership".into())
            })?;
            let peer = acquire_validation_peer(state, &server_id, validation_session)
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;
            (peer, None)
        }
        (InspectorMode::Native, None) => {
            let validation_session = native_temporary_session_id();
            ensure_native_session_with_timeout(state, &server_id, &validation_session, timeout).await?;
            let cleanup = NativeValidationSessionGuard::new(state, validation_session);
            let peer = acquire_validation_peer(state, &server_id, cleanup.session_id())
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;
            (peer, Some(cleanup))
        }
        (_, Some(session)) => {
            let peer = session
                .peer
                .ok_or_else(|| ApiError::InternalError("Inspector session peer is not available".into()))?;
            (peer, None)
        }
        _ => {
            ensure_proxy_connection(state, &server_id, timeout).await?;
            let peer = acquire_peer_for_call(state, &server_id, None)
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;
            (peer, None)
        }
    };

    let params =
        CallToolRequestParams::new(upstream_tool_name).with_arguments(req.arguments.clone().unwrap_or_default());
    let request = ClientRequest::CallToolRequest(CallToolRequest::new(params));

    let mut options = PeerRequestOptions::no_options();
    options.timeout = Some(timeout);

    let call_id = crate::generate_id!("inspcall");
    let mut native_cleanup_session = native_cleanup_session;
    let handle = match peer.send_cancellable_request(request, options).await {
        Ok(handle) => handle,
        Err(err) => {
            if let Some(cleanup) = native_cleanup_session.take() {
                cleanup.cleanup().await;
            }
            return Err(ApiError::InternalError(err.to_string()));
        }
    };

    let RegisteredCall { info, completion } = state
        .inspector_calls
        .start_call(call_id, server_id.clone(), req.mode, req.session_id.clone(), handle)
        .await;

    Ok(PreparedCall {
        info,
        completion,
        server_id,
        native_validation_session: native_cleanup_session,
    })
}

fn extract_tools(result: capability::runtime::ListResult) -> Vec<Value> {
    result
        .items
        .into_tools()
        .unwrap_or_default()
        .into_iter()
        .map(|tool| serde_json::to_value(tool).unwrap_or_else(|_| json!({})))
        .collect()
}

fn extract_prompts(result: capability::runtime::ListResult) -> Vec<Value> {
    result
        .items
        .into_prompts()
        .unwrap_or_default()
        .into_iter()
        .map(|prompt| serde_json::to_value(prompt).unwrap_or_else(|_| json!({})))
        .collect()
}

fn extract_resources(result: capability::runtime::ListResult) -> Vec<Value> {
    result
        .items
        .into_resources()
        .unwrap_or_default()
        .into_iter()
        .map(|resource| serde_json::to_value(resource).unwrap_or_else(|_| json!({})))
        .collect()
}

fn extract_resource_templates(result: capability::runtime::ListResult) -> Vec<Value> {
    result
        .items
        .into_resource_templates()
        .unwrap_or_default()
        .into_iter()
        .map(|template| serde_json::to_value(template).unwrap_or_else(|_| json!({})))
        .collect()
}

async fn list_capability_response(
    state: &AppState,
    query: &InspectorListQuery,
    kind: CapabilityKind,
) -> Result<Value, ApiError> {
    let start = Instant::now();
    let payload = list_capability_payload(state, query, kind).await?;
    Ok(json!({
        "mode": payload.mode,
        kind.response_key(): payload.items,
        "total": payload.items.len(),
        "meta": payload.meta,
        "elapsed_ms": start.elapsed().as_millis() as u64,
    }))
}

async fn list_capability_payload(
    state: &AppState,
    query: &InspectorListQuery,
    kind: CapabilityKind,
) -> Result<CapabilityPayload, ApiError> {
    let timeout = inspector_timeout(state, query.timeout_ms).await;
    let refresh = if query.refresh {
        Some(capability::runtime::RefreshStrategy::Force)
    } else {
        Some(capability::runtime::RefreshStrategy::CacheFirst)
    };

    let extractor = kind.extractor();
    let capability_type = kind.capability_type();

    let mut items: Vec<Value> = Vec::new();
    let mut meta_entries: Vec<Value> = Vec::new();

    match query.mode {
        InspectorMode::Proxy => {
            if query.server_id.is_some() || query.server_name.is_some() {
                let server_id = resolve_server(&query.server_id, &query.server_name).await?;
                let redb = state.redb_cache.clone();
                let pool = state.connection_pool.clone();
                let database = state
                    .database
                    .as_ref()
                    .ok_or(ApiError::InternalError("Database not available".into()))?
                    .clone();
                let (extracted, meta) = list_capability_via_components(
                    redb,
                    pool,
                    database,
                    &server_id,
                    refresh,
                    capability_type,
                    timeout,
                    None,
                    capability::runtime::NameDomain::External,
                    extractor,
                )
                .await?;
                items = extracted;
                meta_entries.push(meta);
            } else {
                let db = state
                    .database
                    .as_ref()
                    .ok_or(ApiError::InternalError("Database not available".into()))?;
                let enabled_servers: Vec<(String, String, Option<String>)> = sqlx::query_as(
                    r#"SELECT sc.id, sc.name, sc.capabilities FROM server_config sc
                       JOIN profile_server ps ON ps.server_id = sc.id AND ps.enabled = 1
                       JOIN profile p ON p.id = ps.profile_id AND p.is_active = 1
                       WHERE sc.enabled = 1
                       GROUP BY sc.id, sc.name, sc.capabilities"#,
                )
                .fetch_all(&db.pool)
                .await
                .map_err(|error| {
                    ApiError::InternalError(format!(
                        "Failed to load enabled servers for Inspector aggregate: {error}"
                    ))
                })?;

                let mut tasks = Vec::new();
                let redb = state.redb_cache.clone();
                let pool = state.connection_pool.clone();
                for (server_id, server_name, caps) in enabled_servers {
                    if !supports_capability(caps.as_deref(), capability_type) {
                        continue;
                    }
                    let db_arc = state.database.as_ref().unwrap().clone();
                    let redb_clone = redb.clone();
                    let pool_clone = pool.clone();
                    let db_clone = db_arc.clone();
                    tasks.push(async move {
                        let sid = server_id.clone();
                        list_capability_via_components(
                            redb_clone,
                            pool_clone,
                            db_clone,
                            &sid,
                            refresh,
                            capability_type,
                            timeout,
                            None,
                            capability::runtime::NameDomain::External,
                            extractor,
                        )
                        .await
                        .map(|(values, meta)| (sid.clone(), server_name.clone(), values, meta))
                        .map_err(|err| (sid.clone(), server_name.clone(), err))
                    });
                }

                let mut aggregate = capability::aggregate::AggregateListStatus::new(kind.label());
                for outcome in futures::stream::iter(tasks)
                    .buffer_unordered(capability::facade::concurrency_limit())
                    .collect::<Vec<_>>()
                    .await
                {
                    match outcome {
                        Ok((_, _, mut values, meta)) => {
                            aggregate.record_success();
                            meta_entries.push(meta);
                            items.append(&mut values);
                        }
                        Err((server_id, server_name, error)) => {
                            aggregate.record_failure(&server_id, &server_name, &error);
                            meta_entries.push(meta_error(&server_id, &error.to_string()));
                        }
                    }
                }
                aggregate
                    .finish_for_result(!items.is_empty())
                    .map_err(|error| ApiError::ServiceUnavailable(error.to_string()))?;
            }
        }
        InspectorMode::Native => {
            ensure_native_allowed()?;
            let server_id = resolve_server(&query.server_id, &query.server_name).await?;
            let (session_id, cleanup) =
                native_validation_scope(state, &server_id, query.session_id.as_deref(), timeout).await?;
            let redb = state.redb_cache.clone();
            let pool = state.connection_pool.clone();
            let database = match state.database.as_ref() {
                Some(database) => database.clone(),
                None => {
                    if let Some(cleanup) = cleanup {
                        cleanup.cleanup().await;
                    }
                    return Err(ApiError::InternalError("Database not available".into()));
                }
            };
            let result = list_capability_via_components(
                redb,
                pool,
                database,
                &server_id,
                refresh,
                capability_type,
                timeout,
                Some(session_id),
                capability::runtime::NameDomain::Upstream,
                extractor,
            )
            .await;
            if let Some(cleanup) = cleanup {
                cleanup.cleanup().await;
            }
            let (extracted, meta) = result?;
            items = extracted;
            meta_entries.push(meta);
        }
    }

    Ok(CapabilityPayload {
        mode: format!("{:?}", query.mode).to_lowercase(),
        items,
        meta: meta_entries,
    })
}

async fn list_capability_via_components(
    redb: Arc<crate::core::cache::RedbCacheManager>,
    pool: Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
    database: Arc<crate::config::database::Database>,
    server_id: &str,
    refresh: Option<capability::runtime::RefreshStrategy>,
    capability_type: capability::CapabilityType,
    timeout: Duration,
    session_id: Option<String>,
    name_domain: capability::runtime::NameDomain,
    extractor: fn(capability::runtime::ListResult) -> Vec<Value>,
) -> Result<(Vec<Value>, Value), ApiError> {
    let service = capability::CapabilityService::new(redb, pool, database);

    let ctx = capability::runtime::ListCtx {
        capability: capability_type,
        server_id: server_id.to_string(),
        refresh,
        timeout: Some(timeout),
        validation_session: session_id,
        runtime_identity: None,
        connection_selection: None,
        name_domain,
    };

    let result = service.list(&ctx).await.map_err(map_capability_list_error)?;
    let meta = meta_success(server_id, &result.meta);
    let items = extractor(result);
    Ok((items, meta))
}

async fn cleanup_native_session(
    state: &AppState,
    _server_id: &str,
    session_id: Option<&str>,
) {
    if let Some(session) = session_id {
        destroy_validation_session(&state.connection_pool, session).await;
    }
}

async fn destroy_validation_session(
    connection_pool: &Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
    session_id: &str,
) {
    let mut pool = connection_pool.lock().await;
    let _ = pool.destroy_validation_session(session_id).await;
}

fn map_anyhow(e: anyhow::Error) -> ApiError {
    let mut message = e.to_string();
    for cause in e.chain().skip(1) {
        let cause = cause.to_string();
        if !cause.is_empty() && !message.contains(&cause) {
            message.push_str(": ");
            message.push_str(&cause);
        }
    }
    ApiError::InternalError(message)
}

fn map_capability_list_error(error: anyhow::Error) -> ApiError {
    if let Some(timeout_ms) = capability::service::connection_timeout_ms(&error) {
        return ApiError::Timeout(format!("Inspector server connect exceeded {timeout_ms} ms"));
    }
    map_anyhow(error)
}

fn meta_success(
    server_id: &str,
    meta: &capability::runtime::Meta,
) -> Value {
    json!({
        "server_id": server_id,
        "cache_hit": meta.cache_hit,
        "source": meta.source,
        "duration_ms": meta.duration_ms,
        "had_peer": meta.had_peer,
    })
}

fn meta_error(
    server_id: &str,
    message: &str,
) -> Value {
    json!({
        "server_id": server_id,
        "error": message,
    })
}

fn native_mode_enabled() -> bool {
    match std::env::var("MCPMATE_INSPECTOR_NATIVE") {
        Ok(val) => {
            let val_lower = val.trim().to_ascii_lowercase();
            // Explicit "off" values disable; everything else (including unknown) enables
            !matches!(val_lower.as_str(), "0" | "false" | "off" | "no")
        }
        // Default to enabled if not set
        Err(_) => true,
    }
}

fn ensure_native_allowed() -> Result<(), ApiError> {
    if native_mode_enabled() {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Native inspector mode is disabled; set MCPMATE_INSPECTOR_NATIVE=on (or unset) to enable".into(),
        ))
    }
}

async fn ensure_native_session(
    state: &AppState,
    server_id: &str,
    session_id: &str,
) -> Result<String, ApiError> {
    {
        let mut pool = state.connection_pool.lock().await;
        pool.cleanup_expired_sessions();
        pool.upsert_validation_session(session_id, NATIVE_VALIDATION_SESSION_TTL);
        if let Err(err) = pool
            .get_or_create_validation_instance(server_id, session_id, NATIVE_VALIDATION_SESSION_TTL)
            .await
        {
            let _ = pool.destroy_validation_session(session_id).await;
            return Err(map_anyhow(err));
        }
    }

    Ok(session_id.to_string())
}

async fn ensure_native_session_with_timeout(
    state: &AppState,
    server_id: &str,
    session_id: &str,
    timeout: Duration,
) -> Result<String, ApiError> {
    let result = run_inspector_operation(timeout, "server connect", async {
        ensure_native_session(state, server_id, session_id).await
    })
    .await;
    if result.is_err() {
        destroy_validation_session(&state.connection_pool, session_id).await;
    }
    result
}

fn native_session_id_for_inspector_session(session_id: &str) -> String {
    format!("inspector_native_session::{session_id}")
}

fn native_temporary_session_id() -> String {
    crate::generate_id!("inspnative")
}

async fn native_validation_session_for_request(
    state: &AppState,
    server_id: &str,
    inspector_session_id: &str,
) -> Result<String, ApiError> {
    let session = state
        .inspector_sessions
        .get_session(inspector_session_id)
        .await
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Inspector session '{}' not found or expired",
                inspector_session_id
            ))
        })?;

    if session.mode != InspectorMode::Native {
        return Err(ApiError::BadRequest(
            "Inspector session is bound to a different mode".into(),
        ));
    }
    if session.server_id != server_id {
        return Err(ApiError::BadRequest(
            "Inspector session is bound to a different server".into(),
        ));
    }

    let validation_session = session
        .validation_session
        .ok_or_else(|| ApiError::InternalError("Native Inspector session is missing validation ownership".into()))?;

    // Native Inspector reuse is intentionally scoped to the explicit inspector
    // session id. Do not reuse a live validation instance by server id alone:
    // service deployments can have different users or organizations inspecting
    // the same server concurrently, and MCP sessions can carry state. When
    // auth/RBAC lands, extend this ownership boundary with actor/org scope.
    let mut pool = state.connection_pool.lock().await;
    if pool.refresh_validation_session(&validation_session, NATIVE_VALIDATION_SESSION_TTL) {
        Ok(validation_session)
    } else {
        Err(ApiError::NotFound(format!(
            "Native Inspector validation session '{}' not found or expired",
            inspector_session_id
        )))
    }
}

async fn native_validation_scope(
    state: &AppState,
    server_id: &str,
    inspector_session_id: Option<&str>,
    timeout: Duration,
) -> Result<(String, Option<NativeValidationSessionGuard>), ApiError> {
    if let Some(inspector_session_id) = inspector_session_id {
        let validation_session = native_validation_session_for_request(state, server_id, inspector_session_id).await?;
        return Ok((validation_session, None));
    }

    let validation_session = native_temporary_session_id();
    ensure_native_session_with_timeout(state, server_id, &validation_session, timeout).await?;
    let cleanup = NativeValidationSessionGuard::new(state, validation_session);
    Ok((cleanup.session_id().to_string(), Some(cleanup)))
}

async fn acquire_validation_peer(
    state: &AppState,
    server_id: &str,
    session_id: &str,
) -> Result<Peer<RoleClient>, anyhow::Error> {
    let mut pool_guard = state.connection_pool.lock().await;
    let peer = pool_guard
        .validation_sessions
        .get(session_id)
        .and_then(|session_servers| session_servers.get(server_id))
        .and_then(|conn| conn.service.as_ref())
        .map(|service| service.peer().clone());

    if let Some(peer) = peer {
        if pool_guard.refresh_validation_session(session_id, NATIVE_VALIDATION_SESSION_TTL) {
            return Ok(peer);
        }
    }

    Err(anyhow!(
        "Native Inspector session '{}' is no longer connected for server '{}'",
        session_id,
        server_id
    ))
}

async fn clone_native_validation_connection(
    state: &AppState,
    server_id: &str,
    session_id: &str,
) -> Result<crate::core::pool::UpstreamConnection, ApiError> {
    let mut pool = state.connection_pool.lock().await;
    if !pool.refresh_validation_session(session_id, NATIVE_VALIDATION_SESSION_TTL) {
        return Err(ApiError::NotFound(format!(
            "Native Inspector validation session '{}' not found or expired",
            session_id
        )));
    }
    pool.validation_sessions
        .get(session_id)
        .and_then(|session| session.get(server_id))
        .cloned()
        .ok_or_else(|| ApiError::InternalError("Validation connection not found".into()))
}

async fn acquire_peer_for_call(
    state: &AppState,
    server_id: &str,
    session_id: Option<&str>,
) -> Result<Peer<RoleClient>, anyhow::Error> {
    let pool_guard = state.connection_pool.lock().await;
    let snapshot = pool_guard.get_snapshot();
    if let Some(instances) = snapshot.get(server_id) {
        if let Some((_, _status, _, _, peer_opt)) = instances.iter().find(|(_, st, _, _, p)| {
            matches!(st, crate::core::foundation::types::ConnectionStatus::Ready) && p.is_some()
        }) {
            if let Some(peer) = peer_opt.clone() {
                return Ok(peer);
            }
        }
    }

    if let Some(session_id) = session_id {
        if let Some(session_servers) = pool_guard.validation_sessions.get(session_id) {
            if let Some(conn) = session_servers.get(server_id) {
                if let Some(service) = conn.service.as_ref() {
                    return Ok(service.peer().clone());
                }
            }
        }
    }

    Err(anyhow!(
        "No ready upstream instance available for server '{}'",
        server_id
    ))
}

async fn resolve_server(
    server_id: &Option<String>,
    server_name: &Option<String>,
) -> Result<String, ApiError> {
    if let Some(id) = server_id.clone() {
        return Ok(id);
    }
    if let Some(name) = server_name.clone() {
        return resolver::to_id(&name)
            .await
            .ok()
            .flatten()
            .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", name)));
    }
    Err(ApiError::BadRequest("server_id or server_name is required".into()))
}

pub(crate) async fn inspector_forward_progress(params: &ProgressNotificationParam) -> bool {
    if let Some(registry) = GLOBAL_CALL_REGISTRY.get() {
        registry.emit_progress(params).await;
    }
    false
}

pub(crate) async fn inspector_forward_log(
    token: Option<&ProgressToken>,
    params: &LoggingMessageNotificationParam,
) -> bool {
    if let Some(registry) = GLOBAL_CALL_REGISTRY.get() {
        registry.emit_log(token, params).await;
    }
    false
}

pub(crate) async fn inspector_forward_cancel(
    request_id: &RequestId,
    reason: Option<String>,
) -> bool {
    if let Some(registry) = GLOBAL_CALL_REGISTRY.get() {
        registry.emit_cancelled(request_id, reason).await;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::run_inspector_operation;
    use crate::api::handlers::ApiError;
    use tokio::time::Duration;

    #[tokio::test]
    async fn inspector_operation_timeout_is_scoped_to_one_operation() {
        let error = run_inspector_operation(Duration::from_millis(5), "resources/read", async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            Ok::<_, ApiError>(())
        })
        .await
        .expect_err("slow resource read should time out");

        assert!(matches!(error, ApiError::Timeout(message) if message.contains("resources/read")));

        run_inspector_operation(Duration::from_millis(25), "prompts/get", async {
            Ok::<_, ApiError>(())
        })
        .await
        .expect("a later operation receives its own timeout budget");
    }
}
