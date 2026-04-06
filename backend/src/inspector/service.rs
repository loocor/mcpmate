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
use crate::core::capability::naming::NamingKind;
use crate::core::capability::resolver;
use crate::core::proxy::server::supports_capability;
use crate::inspector::calls::{InspectorCallInfo, InspectorCallRegistry, InspectorTerminal, RegisteredCall};

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
    pub native_validation_session: Option<String>,
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
    let start = Instant::now();
    match req.mode {
        InspectorMode::Proxy => {
            // Try to resolve unique name first (e.g., "server_name::prompt_name")
            let (server_filter, upstream_name) = if let Ok((server_name, upstream)) =
                capability::naming::resolve_unique_name(capability::naming::NamingKind::Prompt, &req.name).await
            {
                let sid = resolver::to_id(&server_name)
                    .await
                    .ok()
                    .flatten()
                    .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", server_name)))?;
                (Some(sid), upstream)
            } else if req.server_id.is_some() || req.server_name.is_some() {
                // If server is explicitly provided, use it
                let sid = resolve_server(&req.server_id, &req.server_name).await?;
                (Some(sid), req.name.clone())
            } else {
                // No server specified, let facade search across all servers
                (None, req.name.clone())
            };

            let mapping = capability::facade::build_prompt_mapping(&state.connection_pool).await;
            let res = capability::facade::get_upstream_prompt(
                &state.connection_pool,
                &mapping,
                &upstream_name,
                req.arguments.clone(),
                server_filter.as_deref(),
                None,
            )
            .await
            .map_err(map_anyhow)?;

            Ok(json!({
                "result": res,
                "server_id": server_filter,
                "elapsed_ms": start.elapsed().as_millis() as u64,
            }))
        }
        InspectorMode::Native => {
            ensure_native_allowed()?;
            let server_id = resolve_server(&req.server_id, &req.server_name).await?;
            let session_id = ensure_native_session(state, &server_id).await?;
            let server_name = resolver::to_name(&server_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| server_id.clone());
            // Clone the connection to reuse capability direct helper
            let conn = {
                let pool = state.connection_pool.lock().await;
                pool.validation_sessions
                    .get(&session_id)
                    .and_then(|m| m.get(&server_name))
                    .cloned()
                    .ok_or_else(|| ApiError::InternalError("Validation connection not found".into()))?
            };
            let res =
                crate::core::capability::prompts::get_upstream_prompt_direct(&conn, &req.name, req.arguments.clone())
                    .await
                    .map_err(map_anyhow)?;
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
    let start = Instant::now();
    match req.mode {
        InspectorMode::Proxy => {
            let (server_filter, upstream_uri) = if let Ok((server_name, upstream)) =
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
                None,
            )
            .await
            .map_err(map_anyhow)?;
            Ok(json!({
                "result": res,
                "server_id": server_filter,
                "elapsed_ms": start.elapsed().as_millis() as u64,
            }))
        }
        InspectorMode::Native => {
            ensure_native_allowed()?;
            let server_id = resolve_server(&req.server_id, &req.server_name).await?;
            let session_id = ensure_native_session(state, &server_id).await?;
            let server_name = resolver::to_name(&server_id)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| server_id.clone());
            let conn = {
                let pool = state.connection_pool.lock().await;
                pool.validation_sessions
                    .get(&session_id)
                    .and_then(|m| m.get(&server_name))
                    .cloned()
                    .ok_or_else(|| ApiError::InternalError("Validation connection not found".into()))?
            };
            let res = crate::core::capability::resources::read_upstream_resource_direct(&conn, &req.uri)
                .await
                .map_err(map_anyhow)?;
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
    let native_cleanup = prepared.native_validation_session.clone();
    let server_id = prepared.server_id.clone();

    let result = prepared
        .completion
        .await
        .map_err(|_| ApiError::InternalError("Inspector call channel dropped".into()))?;

    if let Some(session) = native_cleanup {
        cleanup_native_session(state, &server_id, Some(&session)).await;
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
    let state_clone = state.clone();

    tokio::spawn(async move {
        let completion = prepared.completion;
        let server_id = prepared.server_id.clone();
        let native_cleanup = prepared.native_validation_session.clone();

        let _ = completion.await;

        if let Some(session) = native_cleanup {
            cleanup_native_session(&state_clone, &server_id, Some(&session)).await;
        }
    });

    Ok(info)
}

pub async fn open_session(
    state: &Arc<AppState>,
    req: &InspectorSessionOpenReq,
) -> Result<InspectorSessionOpenData, ApiError> {
    if matches!(req.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }

    let server_id = resolve_server(&req.server_id, &req.server_name).await?;
    let validation_session = if matches!(req.mode, InspectorMode::Native) {
        Some(ensure_native_session(state, &server_id).await?)
    } else {
        {
            let mut pool = state.connection_pool.lock().await;
            pool.ensure_connected(&server_id).await.map_err(map_anyhow)?;
        }
        None
    };

    let peer = acquire_peer_for_call(state, &server_id, validation_session.as_deref())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let session_id = crate::generate_id!("inspses");
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

async fn start_tool_call_internal(
    state: &Arc<AppState>,
    req: &InspectorToolCallReq,
) -> Result<PreparedCall, ApiError> {
    if matches!(req.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }

    let timeout = Duration::from_millis(req.timeout_ms.unwrap_or(60_000));

    let active_session =
        if let Some(session_id) = req.session_id.as_ref() {
            Some(state.inspector_sessions.get_session(session_id).await.ok_or_else(|| {
                ApiError::NotFound(format!("Inspector session '{}' not found or expired", session_id))
            })?)
        } else {
            None
        };

    let (server_id, mut upstream_tool_name) = match req.mode {
        InspectorMode::Proxy => {
            if let Ok((server_name, upstream)) =
                capability::naming::resolve_unique_name(capability::naming::NamingKind::Tool, &req.tool).await
            {
                let sid = resolver::to_id(&server_name)
                    .await
                    .ok()
                    .flatten()
                    .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", server_name)))?;
                (sid, upstream)
            } else {
                let sid = if let Some(session) = &active_session {
                    session.server_id.clone()
                } else {
                    resolve_server(&req.server_id, &req.server_name).await?
                };
                (sid.clone(), req.tool.clone())
            }
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

    if let Ok((resolved_server_name, resolved_tool_name)) =
        capability::naming::resolve_unique_name(NamingKind::Tool, &upstream_tool_name).await
    {
        let resolved_server_id = resolver::to_id(&resolved_server_name).await.ok().flatten();
        let matches_server = resolved_server_id
            .as_deref()
            .map(|id| id == server_id)
            .unwrap_or_else(|| resolved_server_name == server_id);
        if matches_server {
            upstream_tool_name = resolved_tool_name;
        }
    }

    if let Some(session) = &active_session {
        if session.server_id != server_id {
            return Err(ApiError::BadRequest(
                "Inspector session is bound to a different server".into(),
            ));
        }
    }

    let (peer, native_cleanup_session) = match (req.mode, active_session) {
        (InspectorMode::Native, Some(session)) => (session.peer, None),
        (InspectorMode::Native, None) => {
            let validation_session = ensure_native_session(state, &server_id).await?;
            let peer = acquire_peer_for_call(state, &server_id, Some(&validation_session))
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;
            (peer, Some(validation_session))
        }
        (_, Some(session)) => (session.peer, None),
        _ => {
            {
                let mut pool = state.connection_pool.lock().await;
                pool.ensure_connected(&server_id).await.map_err(map_anyhow)?;
            }
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
    let handle = peer
        .send_cancellable_request(request, options)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

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
        runtime_identity: None,
        connection_selection: None,
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
