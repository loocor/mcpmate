use anyhow::anyhow;
use futures::StreamExt;
use rmcp::RoleClient;
use rmcp::model::{
    CallToolRequest, CallToolRequestParam, ClientRequest, LoggingMessageNotificationParam, ProgressNotificationParam,
    ProgressToken, RequestId,
};
use rmcp::service::{Peer, PeerRequestOptions};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;

use crate::api::handlers::ApiError;
use crate::api::models::inspector::{
    InspectorListQuery, InspectorMode, InspectorPromptGetReq, InspectorResourceReadQuery, InspectorToolCallReq,
};
use crate::api::routes::AppState;
use crate::core::capability;
use crate::core::capability::resolver;
use crate::core::proxy::server::supports_capability;

#[derive(Debug, Clone)]
pub struct ToolCallOutcome {
    pub result: Option<Value>,
    pub server_id: Option<String>,
    pub elapsed_ms: u64,
    pub message: Option<String>,
}

#[derive(Clone, Copy)]
enum CapabilityKind {
    Tools,
    Prompts,
    Resources,
}

impl CapabilityKind {
    fn capability_type(self) -> capability::CapabilityType {
        match self {
            CapabilityKind::Tools => capability::CapabilityType::Tools,
            CapabilityKind::Prompts => capability::CapabilityType::Prompts,
            CapabilityKind::Resources => capability::CapabilityType::Resources,
        }
    }

    fn response_key(self) -> &'static str {
        match self {
            CapabilityKind::Tools => "tools",
            CapabilityKind::Prompts => "prompts",
            CapabilityKind::Resources => "resources",
        }
    }

    fn extractor(self) -> fn(capability::runtime::ListResult) -> Vec<Value> {
        match self {
            CapabilityKind::Tools => extract_tools,
            CapabilityKind::Prompts => extract_prompts,
            CapabilityKind::Resources => extract_resources,
        }
    }
}

struct CapabilityPayload {
    mode: String,
    items: Vec<Value>,
    meta: Vec<Value>,
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

pub async fn prompt_get(
    state: &AppState,
    req: &InspectorPromptGetReq,
) -> Result<Value, ApiError> {
    if matches!(req.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }
    let start = Instant::now();
    let (server_id, upstream_name) = match req.mode {
        InspectorMode::Proxy => {
            if let Ok((server_name, upstream)) =
                capability::naming::resolve_unique_name(capability::naming::NamingKind::Prompt, &req.name).await
            {
                let sid = resolver::to_id(&server_name)
                    .await
                    .ok()
                    .flatten()
                    .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", server_name)))?;
                (sid, upstream)
            } else {
                let sid = resolve_server(&req.server_id, &req.server_name).await?;
                (sid, req.name.clone())
            }
        }
        InspectorMode::Native => {
            let sid = resolve_server(&req.server_id, &req.server_name).await?;
            (sid, req.name.clone())
        }
    };

    let mapping = capability::facade::build_prompt_mapping(&state.connection_pool).await;
    let res = capability::facade::get_upstream_prompt(
        &state.connection_pool,
        &mapping,
        &upstream_name,
        req.arguments.clone(),
        Some(&server_id),
    )
    .await
    .map_err(map_anyhow)?;
    Ok(json!({
        "result": res,
        "server_id": server_id,
        "elapsed_ms": start.elapsed().as_millis() as u64,
    }))
}

pub async fn resource_read(
    state: &AppState,
    req: &InspectorResourceReadQuery,
) -> Result<Value, ApiError> {
    if matches!(req.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }
    let start = Instant::now();
    let (server_filter, upstream_uri) = match req.mode {
        InspectorMode::Proxy => {
            if let Ok((server_name, upstream)) =
                capability::naming::resolve_unique_name(capability::naming::NamingKind::Resource, &req.uri).await
            {
                let sid = resolver::to_id(&server_name)
                    .await
                    .ok()
                    .flatten()
                    .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", server_name)))?;
                (Some(sid), upstream)
            } else {
                (req.server_id.clone(), req.uri.clone())
            }
        }
        InspectorMode::Native => (
            resolve_server(&req.server_id, &req.server_name).await.ok(),
            req.uri.clone(),
        ),
    };

    let mapping = if let Some(sid) = &server_filter {
        let mut filter: HashSet<String> = HashSet::new();
        filter.insert(sid.clone());
        capability::facade::build_resource_mapping_filtered(
            &state.connection_pool,
            state.database.as_ref(),
            Some(&filter),
        )
        .await
    } else {
        capability::facade::build_resource_mapping(&state.connection_pool, state.database.as_ref()).await
    };
    let res = capability::facade::read_upstream_resource(
        &state.connection_pool,
        &mapping,
        &upstream_uri,
        server_filter.as_deref(),
    )
    .await
    .map_err(map_anyhow)?;
    Ok(json!({
        "result": res,
        "server_id": server_filter,
        "elapsed_ms": start.elapsed().as_millis() as u64,
    }))
}

pub async fn call_tool(
    state: &AppState,
    req: &InspectorToolCallReq,
) -> Result<ToolCallOutcome, ApiError> {
    if matches!(req.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }
    let started = Instant::now();
    let (server_id, upstream_tool_name, session_id) = match req.mode {
        InspectorMode::Proxy => {
            if let Ok((server_name, upstream)) =
                capability::naming::resolve_unique_name(capability::naming::NamingKind::Tool, &req.tool).await
            {
                let sid = resolver::to_id(&server_name)
                    .await
                    .ok()
                    .flatten()
                    .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", server_name)))?;
                (sid, upstream, None)
            } else {
                let sid = resolve_server(&req.server_id, &req.server_name).await?;
                (sid, req.tool.clone(), None)
            }
        }
        InspectorMode::Native => {
            let sid = resolve_server(&req.server_id, &req.server_name).await?;
            let session = ensure_native_session(state, &sid).await?;
            let requested = req.tool.clone();
            let expected_server_name = if let Some(name) = req.server_name.clone() {
                Some(name)
            } else {
                resolver::to_name(&sid).await.ok().flatten()
            };
            let upstream =
                match capability::naming::resolve_unique_name(capability::naming::NamingKind::Tool, &requested).await {
                    Ok((unique_server_name, upstream_name)) => {
                        let matches_server = match expected_server_name {
                            Some(ref expected) => expected == &unique_server_name,
                            None => resolver::to_id(&unique_server_name)
                                .await
                                .ok()
                                .flatten()
                                .is_some_and(|resolved_id| resolved_id == sid),
                        };
                        if matches_server {
                            upstream_name
                        } else {
                            requested.clone()
                        }
                    }
                    Err(_) => requested.clone(),
                };
            (sid, upstream, Some(session))
        }
    };

    let peer = acquire_peer_for_call(state, &server_id, session_id.as_deref())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let timeout = Duration::from_millis(req.timeout_ms.unwrap_or(60_000));
    let request = ClientRequest::CallToolRequest(CallToolRequest {
        method: Default::default(),
        params: CallToolRequestParam {
            name: upstream_tool_name.into(),
            arguments: req.arguments.clone(),
        },
        extensions: Default::default(),
    });

    let options = PeerRequestOptions {
        timeout: Some(timeout),
        meta: None,
    };

    let handle = peer
        .send_cancellable_request(request, options)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let response = handle.await_response().await;
    let elapsed_ms = started.elapsed().as_millis() as u64;

    let outcome = match response {
        Ok(rmcp::model::ServerResult::CallToolResult(result)) => {
            let value = serde_json::to_value(result).unwrap_or_else(|_| json!({}));
            ToolCallOutcome {
                result: Some(value),
                server_id: Some(server_id.clone()),
                elapsed_ms,
                message: Some("completed".to_string()),
            }
        }
        Ok(other) => {
            cleanup_native_session(state, &server_id, session_id.as_deref()).await;
            return Err(ApiError::InternalError(format!(
                "Unexpected server result: {:?}",
                other
            )));
        }
        Err(err) => {
            cleanup_native_session(state, &server_id, session_id.as_deref()).await;
            let (api_error, _message) = map_tool_call_error(anyhow!(err.to_string()));
            return Err(api_error);
        }
    };

    cleanup_native_session(state, &server_id, session_id.as_deref()).await;
    Ok(outcome)
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

async fn list_capability_response(
    state: &AppState,
    query: &InspectorListQuery,
    kind: CapabilityKind,
) -> Result<Value, ApiError> {
    let payload = list_capability_payload(state, query, kind).await?;
    Ok(json!({
        "mode": payload.mode,
        kind.response_key(): payload.items,
        "total": payload.items.len(),
        "meta": payload.meta,
    }))
}

async fn list_capability_payload(
    state: &AppState,
    query: &InspectorListQuery,
    kind: CapabilityKind,
) -> Result<CapabilityPayload, ApiError> {
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
                    None,
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
                .unwrap_or_default();

                let mut tasks = Vec::new();
                let redb = state.redb_cache.clone();
                let pool = state.connection_pool.clone();
                for (server_id, _name, caps) in enabled_servers {
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
                            None,
                            extractor,
                        )
                        .await
                        .map(|(values, meta)| (sid.clone(), values, meta))
                        .map_err(|err| (sid.clone(), err))
                    });
                }

                for outcome in futures::stream::iter(tasks)
                    .buffer_unordered(capability::facade::concurrency_limit())
                    .collect::<Vec<_>>()
                    .await
                {
                    match outcome {
                        Ok((_, mut values, meta)) => {
                            meta_entries.push(meta);
                            items.append(&mut values);
                        }
                        Err((sid, err)) => meta_entries.push(meta_error(&sid, &err.to_string())),
                    }
                }
            }
        }
        InspectorMode::Native => {
            ensure_native_allowed()?;
            let server_id = resolve_server(&query.server_id, &query.server_name).await?;
            let session_id = ensure_native_session(state, &server_id).await?;
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
                Some(session_id),
                extractor,
            )
            .await?;
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
    session_id: Option<String>,
    extractor: fn(capability::runtime::ListResult) -> Vec<Value>,
) -> Result<(Vec<Value>, Value), ApiError> {
    let service = capability::CapabilityService::new(redb, pool, database);

    let ctx = capability::runtime::ListCtx {
        capability: capability_type,
        server_id: server_id.to_string(),
        refresh,
        timeout: Some(Duration::from_secs(10)),
        validation_session: session_id,
    };

    let result = service.list(&ctx).await.map_err(map_anyhow)?;
    let meta = meta_success(server_id, &result.meta);
    let items = extractor(result);
    Ok((items, meta))
}

async fn cleanup_native_session(
    state: &AppState,
    server_id: &str,
    session_id: Option<&str>,
) {
    if let Some(session) = session_id {
        let server_name = resolver::to_name(server_id)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| server_id.to_string());
        let mut pool = state.connection_pool.lock().await;
        let _ = pool.destroy_validation_instance(&server_name, session).await;
    }
}

fn map_anyhow(e: anyhow::Error) -> ApiError {
    ApiError::InternalError(e.to_string())
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
            matches!(val_lower.as_str(), "1" | "true" | "on" | "yes")
        }
        Err(_) => true,
    }
}

fn ensure_native_allowed() -> Result<(), ApiError> {
    if native_mode_enabled() {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Native inspector mode is disabled; set MCPMATE_INSPECTOR_NATIVE=on to enable".into(),
        ))
    }
}

async fn ensure_native_session(
    state: &AppState,
    server_id: &str,
) -> Result<String, ApiError> {
    let session_id = format!("inspector_native::{}", server_id);
    let server_name = resolver::to_name(server_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| server_id.to_string());

    {
        let mut pool = state.connection_pool.lock().await;
        pool.cleanup_expired_sessions();
        pool.upsert_validation_session(&session_id, Duration::from_secs(300));
        pool.get_or_create_validation_instance(&server_name, &session_id, Duration::from_secs(300))
            .await
            .map_err(map_anyhow)?;
    }

    Ok(session_id)
}

async fn acquire_peer_for_call(
    state: &AppState,
    server_id: &str,
    session_id: Option<&str>,
) -> Result<Peer<RoleClient>, anyhow::Error> {
    let server_name = resolver::to_name(server_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| server_id.to_string());

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
            if let Some(conn) = session_servers.get(&server_name) {
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

fn map_tool_call_error(e: anyhow::Error) -> (ApiError, String) {
    let message = e.to_string();
    let lower = message.to_ascii_lowercase();
    if lower.contains("timeout") {
        (ApiError::Timeout(message.clone()), message)
    } else if lower.contains("not found") {
        (ApiError::NotFound(message.clone()), message)
    } else if lower.contains("forbidden") {
        (ApiError::Forbidden(message.clone()), message)
    } else {
        (ApiError::InternalError(message.clone()), message)
    }
}

// Inspector forwarders are retained for compatibility with the upstream transport handler,
// but now operate as no-ops because Inspector runs synchronously.
pub(crate) async fn inspector_forward_progress(_params: &ProgressNotificationParam) -> bool {
    false
}

pub(crate) async fn inspector_forward_log(
    _token: Option<&ProgressToken>,
    _params: &LoggingMessageNotificationParam,
) -> bool {
    false
}

pub(crate) async fn inspector_forward_cancel(
    _request_id: &RequestId,
    _reason: Option<String>,
) -> bool {
    false
}
