use anyhow::anyhow;
use dashmap::DashMap;
use futures::StreamExt;
use once_cell::sync::Lazy;
use rmcp::RoleClient;
use rmcp::model::{
    CallToolRequest, CallToolRequestParam, ClientRequest, LoggingMessageNotificationParam, NumberOrString,
    ProgressNotificationParam, ProgressToken, RequestId,
};
use rmcp::service::{Peer, PeerRequestOptions};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
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
use crate::inspector::bus;
use crate::inspector::registry::{CallRegistry, CallStatus, CallSummary};
use crate::inspector::sse::{SseEvent, SseEventKind};

type SharedCall = Arc<InspectorCallContext>;

struct InspectorCallContext {
    call_id: String,
    server_id: String,
    progress_token: ProgressToken,
    request_id: RequestId,
    peer: Peer<RoleClient>,
    seq: AtomicU64,
}

impl InspectorCallContext {
    fn new(
        call_id: String,
        server_id: String,
        progress_token: ProgressToken,
        request_id: RequestId,
        peer: Peer<RoleClient>,
        initial_seq: u64,
    ) -> Self {
        Self {
            call_id,
            server_id,
            progress_token,
            request_id,
            peer,
            seq: AtomicU64::new(initial_seq),
        }
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }
}

static CALLS_BY_ID: Lazy<DashMap<String, SharedCall>> = Lazy::new(DashMap::new);
static CALLS_BY_TOKEN: Lazy<DashMap<String, SharedCall>> = Lazy::new(DashMap::new);
static CALLS_BY_REQUEST: Lazy<DashMap<String, SharedCall>> = Lazy::new(DashMap::new);

enum ToolCallError {
    Api(ApiError),
    Handled,
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

pub async fn start_tool_call(
    state: &AppState,
    req: &InspectorToolCallReq,
) -> Result<String, ApiError> {
    if matches!(req.mode, InspectorMode::Native) {
        ensure_native_allowed()?;
    }
    let call_id = nanoid::nanoid!(12);
    let bus = bus::global();
    let _ = bus.create_call(&call_id, 256).await;
    CallRegistry::global()
        .insert(CallSummary::new(
            &call_id,
            &format!("{:?}", req.mode).to_lowercase(),
            "tool",
            "call",
            Some(req.tool.clone()),
        ))
        .await;

    let timeout_ms = req.timeout_ms.unwrap_or(60_000);
    let state_cloned = state.clone();
    let req_cloned = req.clone();
    let call_id_clone = call_id.clone();
    let bus_for_task = bus.clone();
    tokio::spawn(async move {
        let result = do_tool_call(&state_cloned, &req_cloned, &call_id_clone, timeout_ms).await;
        if let Err(ToolCallError::Api(err)) = result {
            let message = err.to_string();
            let seq = 3;
            let _ = publish(
                &bus_for_task,
                &call_id_clone,
                SseEventKind::Error,
                json!({"message": message}),
                Some(seq),
            )
            .await;
            CallRegistry::global()
                .update_status(&call_id_clone, CallStatus::Error, seq, Some(message), None)
                .await;
        }
        bus_for_task.finish(&call_id_clone).await;
    });
    let _ = publish(&bus, &call_id, SseEventKind::Log, json!({"message":"queued"}), Some(1)).await;
    let _ = publish(
        &bus,
        &call_id,
        SseEventKind::Progress,
        json!({"percent":5, "message":"connecting"}),
        Some(2),
    )
    .await;
    Ok(call_id)
}

pub async fn wait_tool_result_inline(
    call_id: &str,
    wait_ms: u64,
) -> Option<Value> {
    let bus = bus::global();
    if let Some(mut rx) = bus.subscribe(call_id).await {
        let fut = async move {
            loop {
                match rx.recv().await {
                    Ok(ev) => match ev.event {
                        SseEventKind::Result => {
                            return Some(json!({
                                "success": true,
                                "call_id": call_id,
                                "message": "completed",
                                "data": ev.data
                            }));
                        }
                        SseEventKind::Error => {
                            return Some(json!({"success": false, "call_id": call_id, "error": ev.data}));
                        }
                        SseEventKind::Cancelled => {
                            return Some(json!({
                                "success": false,
                                "call_id": call_id,
                                "error": {"message": "cancelled"}
                            }));
                        }
                        _ => {}
                    },
                    Err(_) => return None,
                }
            }
        };
        if let Ok(Some(v)) = tokio::time::timeout(Duration::from_millis(wait_ms), fut).await {
            return Some(v);
        }
    }
    None
}

pub async fn cancel_tool_call(call_id: &str) {
    if cancel_active_call(call_id).await {
        return;
    }
    let bus = bus::global();
    bus.cancel(call_id).await;
    let _ = publish(
        &bus,
        call_id,
        SseEventKind::Cancelled,
        json!({"message": "cancelled"}),
        None,
    )
    .await;
    CallRegistry::global()
        .update_status(call_id, CallStatus::Cancelled, 0, None, None)
        .await;
    tokio::spawn({
        let bus = bus.clone();
        let id = call_id.to_string();
        async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            bus.finish(&id).await;
        }
    });
}

pub(crate) async fn inspector_forward_progress(params: &ProgressNotificationParam) -> bool {
    handle_inspector_progress(params).await
}

pub(crate) async fn inspector_forward_log(
    token: Option<&ProgressToken>,
    params: &LoggingMessageNotificationParam,
) -> bool {
    handle_inspector_log(token, params).await
}

pub(crate) async fn inspector_forward_cancel(
    request_id: &RequestId,
    reason: Option<String>,
) -> bool {
    handle_inspector_cancel(request_id, reason).await
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
                let (extracted, meta) =
                    list_capability_for_server(state, &server_id, refresh, capability_type, None, extractor).await?;
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
                for (server_id, _name, caps) in enabled_servers {
                    if !supports_capability(caps.as_deref(), capability_type) {
                        continue;
                    }
                    let redb = state.redb_cache.clone();
                    let pool = state.connection_pool.clone();
                    let db_arc = state.database.as_ref().unwrap().clone();
                    tasks.push(async move {
                        let sid = server_id.clone();
                        list_capability_for_server_with_context(
                            &redb,
                            &pool,
                            &db_arc,
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
            let (extracted, meta) =
                list_capability_for_server(state, &server_id, refresh, capability_type, Some(session_id), extractor)
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

async fn list_capability_for_server(
    state: &AppState,
    server_id: &str,
    refresh: Option<capability::runtime::RefreshStrategy>,
    capability_type: capability::CapabilityType,
    session_id: Option<String>,
    extractor: fn(capability::runtime::ListResult) -> Vec<Value>,
) -> Result<(Vec<Value>, Value), ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or(ApiError::InternalError("Database not available".into()))?
        .clone();
    let (items, meta) = list_capability_for_server_with_context(
        &state.redb_cache,
        &state.connection_pool,
        &db,
        server_id,
        refresh,
        capability_type,
        session_id.as_deref(),
        extractor,
    )
    .await
    .map_err(map_anyhow)?;
    Ok((items, meta))
}

async fn list_capability_for_server_with_context(
    redb: &Arc<crate::core::cache::RedbCacheManager>,
    pool: &Arc<tokio::sync::Mutex<crate::core::pool::UpstreamConnectionPool>>,
    db: &Arc<crate::config::database::Database>,
    server_id: &str,
    refresh: Option<capability::runtime::RefreshStrategy>,
    capability_type: capability::CapabilityType,
    session_id: Option<&str>,
    extractor: fn(capability::runtime::ListResult) -> Vec<Value>,
) -> Result<(Vec<Value>, Value), anyhow::Error> {
    let ctx = capability::runtime::ListCtx {
        capability: capability_type,
        server_id: server_id.to_string(),
        refresh,
        timeout: Some(Duration::from_secs(10)),
        validation_session: session_id.map(|s| s.to_string()),
    };
    capability::runtime::list(&ctx, redb, pool, db).await.map(|result| {
        let meta = meta_success(server_id, &result.meta);
        let items = extractor(result);
        (items, meta)
    })
}

async fn cancel_active_call(call_id: &str) -> bool {
    if let Some(call) = get_call_by_id(call_id) {
        let reason = Some("cancelled_by_inspector".to_string());
        let _ = call
            .peer
            .notify_cancelled(rmcp::model::CancelledNotificationParam {
                request_id: call.request_id.clone(),
                reason: reason.clone(),
            })
            .await;
        complete_call_cancelled_manual(call, reason).await;
        return true;
    }
    false
}

fn number_or_string_to_string(value: &NumberOrString) -> String {
    match value {
        NumberOrString::Number(n) => n.to_string(),
        NumberOrString::String(s) => s.to_string(),
    }
}

fn progress_token_key(token: &ProgressToken) -> String {
    number_or_string_to_string(&token.0)
}

fn request_id_key(id: &RequestId) -> String {
    number_or_string_to_string(id)
}

fn insert_call(call: SharedCall) {
    CALLS_BY_ID.insert(call.call_id.clone(), call.clone());
    CALLS_BY_TOKEN.insert(progress_token_key(&call.progress_token), call.clone());
    CALLS_BY_REQUEST.insert(request_id_key(&call.request_id), call);
}

fn remove_call(call: &InspectorCallContext) {
    CALLS_BY_ID.remove(&call.call_id);
    CALLS_BY_TOKEN.remove(&progress_token_key(&call.progress_token));
    CALLS_BY_REQUEST.remove(&request_id_key(&call.request_id));
}

fn get_call_by_id(call_id: &str) -> Option<SharedCall> {
    CALLS_BY_ID.get(call_id).map(|entry| entry.clone())
}

fn get_call_by_token(token: &ProgressToken) -> Option<SharedCall> {
    CALLS_BY_TOKEN
        .get(&progress_token_key(token))
        .map(|entry| entry.clone())
}

fn get_call_by_request_id(request_id: &RequestId) -> Option<SharedCall> {
    CALLS_BY_REQUEST
        .get(&request_id_key(request_id))
        .map(|entry| entry.clone())
}

async fn publish_for_call(
    call: &InspectorCallContext,
    kind: SseEventKind,
    data: Value,
    seq: Option<u64>,
) -> bool {
    let bus = bus::global();
    bus.publish(
        &call.call_id,
        SseEvent {
            event: kind,
            call_id: call.call_id.clone(),
            seq,
            data: Some(data),
        },
    )
    .await
}

async fn handle_inspector_progress(params: &ProgressNotificationParam) -> bool {
    if let Some(call) = get_call_by_token(&params.progress_token) {
        let seq = call.next_seq();
        let percent = match params.total {
            Some(total) if total > 0.0 => ((params.progress / total) * 100.0).clamp(0.0, 100.0),
            _ => params.progress,
        };
        let data = json!({
            "progress_token": progress_token_key(&params.progress_token),
            "progress": params.progress,
            "total": params.total,
            "percent": percent,
            "message": params.message,
        });
        let _ = publish_for_call(&call, SseEventKind::Progress, data, Some(seq)).await;
        CallRegistry::global()
            .update_progress(&call.call_id, seq, percent.round().clamp(0.0, 100.0) as u8)
            .await;
        return true;
    }
    false
}

async fn handle_inspector_log(
    token: Option<&ProgressToken>,
    params: &LoggingMessageNotificationParam,
) -> bool {
    if let Some(token) = token {
        if let Some(call) = get_call_by_token(token) {
            let seq = call.next_seq();
            let data = json!({
                "progress_token": progress_token_key(token),
                "level": params.level,
                "logger": params.logger,
                "data": params.data,
            });
            let _ = publish_for_call(&call, SseEventKind::Log, data, Some(seq)).await;
            return true;
        }
    }
    false
}

async fn handle_inspector_cancel(
    request_id: &RequestId,
    reason: Option<String>,
) -> bool {
    if let Some(call) = get_call_by_request_id(request_id) {
        let seq = call.next_seq();
        let data = json!({
            "request_id": request_id_key(request_id),
            "reason": reason,
        });
        let _ = publish_for_call(&call, SseEventKind::Cancelled, data, Some(seq)).await;
        CallRegistry::global()
            .update_status(&call.call_id, CallStatus::Cancelled, seq, None, None)
            .await;
        remove_call(&call);
        return true;
    }
    false
}

fn register_proxy_call(
    call_id: &str,
    server_id: &str,
    progress_token: ProgressToken,
    request_id: RequestId,
    peer: Peer<RoleClient>,
    initial_seq: u64,
) -> SharedCall {
    let call = Arc::new(InspectorCallContext::new(
        call_id.to_string(),
        server_id.to_string(),
        progress_token,
        request_id,
        peer,
        initial_seq,
    ));
    insert_call(call.clone());
    call
}

async fn complete_call_success(
    call: SharedCall,
    result: Value,
    elapsed_ms: u64,
) {
    let seq = call.next_seq();
    let data = json!({
        "message": "completed",
        "result": result,
        "server_id": call.server_id,
        "elapsed_ms": elapsed_ms,
    });
    let _ = publish_for_call(&call, SseEventKind::Result, data, Some(seq)).await;
    CallRegistry::global()
        .update_status(&call.call_id, CallStatus::Ok, seq, None, Some(elapsed_ms))
        .await;
    remove_call(&call);
}

async fn complete_call_error(
    call: SharedCall,
    message: String,
    elapsed_ms: Option<u64>,
) {
    let seq = call.next_seq();
    let data = json!({
        "message": message,
        "server_id": call.server_id,
        "elapsed_ms": elapsed_ms,
    });
    let _ = publish_for_call(&call, SseEventKind::Error, data, Some(seq)).await;
    CallRegistry::global()
        .update_status(&call.call_id, CallStatus::Error, seq, Some(message), elapsed_ms)
        .await;
    remove_call(&call);
}

async fn complete_call_cancelled_manual(
    call: SharedCall,
    message: Option<String>,
) {
    let seq = call.next_seq();
    let data = json!({
        "message": message,
        "server_id": call.server_id,
    });
    let _ = publish_for_call(&call, SseEventKind::Cancelled, data, Some(seq)).await;
    CallRegistry::global()
        .update_status(&call.call_id, CallStatus::Cancelled, seq, None, None)
        .await;
    remove_call(&call);
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

async fn publish(
    bus: &bus::CallBus,
    call_id: &str,
    kind: SseEventKind,
    data: Value,
    seq: Option<u64>,
) -> bool {
    bus.publish(
        call_id,
        SseEvent {
            event: kind,
            call_id: call_id.to_string(),
            seq,
            data: Some(data),
        },
    )
    .await
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
        Err(_) => false,
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
        if let Some((_, _status, _res, _prm, peer_opt)) = instances.iter().find(|(_, st, _, _, p)| {
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

async fn do_tool_call(
    state: &AppState,
    req: &InspectorToolCallReq,
    call_id: &str,
    timeout_ms: u64,
) -> Result<(), ToolCallError> {
    let started = Instant::now();

    let (server_id, upstream_tool_name, session_id) = match req.mode {
        InspectorMode::Proxy => {
            if let Ok((server_name, upstream)) =
                capability::naming::resolve_unique_name(capability::naming::NamingKind::Tool, &req.tool).await
            {
                let sid = resolver::to_id(&server_name).await.ok().flatten().ok_or_else(|| {
                    ToolCallError::Api(ApiError::BadRequest(format!("Server '{}' not found", server_name)))
                })?;
                (sid, upstream, None)
            } else {
                let sid = resolve_server(&req.server_id, &req.server_name)
                    .await
                    .map_err(ToolCallError::Api)?;
                (sid, req.tool.clone(), None)
            }
        }
        InspectorMode::Native => {
            let sid = resolve_server(&req.server_id, &req.server_name)
                .await
                .map_err(ToolCallError::Api)?;
            let session = ensure_native_session(state, &sid).await.map_err(ToolCallError::Api)?;
            (sid, req.tool.clone(), Some(session))
        }
    };

    let peer = acquire_peer_for_call(state, &server_id, session_id.as_deref())
        .await
        .map_err(|e| ToolCallError::Api(ApiError::InternalError(e.to_string())))?;

    let timeout = Duration::from_millis(timeout_ms);
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
        .map_err(|e| ToolCallError::Api(ApiError::InternalError(e.to_string())))?;

    let call_ctx = register_proxy_call(
        call_id,
        &server_id,
        handle.progress_token.clone(),
        handle.id.clone(),
        peer.clone(),
        3,
    );

    let response = handle.await_response().await;

    match response {
        Ok(rmcp::model::ServerResult::CallToolResult(result)) => {
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let value = serde_json::to_value(result).unwrap_or_else(|_| json!({}));
            complete_call_success(call_ctx, value, elapsed_ms).await;
            cleanup_native_session(state, &server_id, session_id.as_deref()).await;
            Ok(())
        }
        Ok(other) => {
            let message = format!("Unexpected server result: {:?}", other);
            complete_call_error(call_ctx, message, Some(started.elapsed().as_millis() as u64)).await;
            cleanup_native_session(state, &server_id, session_id.as_deref()).await;
            Err(ToolCallError::Handled)
        }
        Err(err) => {
            let (_api_error, message) = map_tool_call_error(anyhow!(err.to_string()));
            complete_call_error(call_ctx, message, Some(started.elapsed().as_millis() as u64)).await;
            cleanup_native_session(state, &server_id, session_id.as_deref()).await;
            Err(ToolCallError::Handled)
        }
    }
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
