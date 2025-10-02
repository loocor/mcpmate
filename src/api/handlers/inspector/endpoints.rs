// no StreamExt needed in this module
use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use serde_json::{Value, json};
use tokio::time::Duration;

use crate::api::handlers::ApiError;
// use crate::api::models::inspector::InspectorMode;
use crate::api::models::inspector::{
    InspectorListQuery, InspectorPromptGetData, InspectorPromptGetReq, InspectorPromptGetResp,
    InspectorPromptsListData, InspectorPromptsListResp, InspectorResourceReadData, InspectorResourceReadQuery,
    InspectorResourceReadResp, InspectorResourcesListData, InspectorResourcesListResp, InspectorToolCallData,
    InspectorToolCallReq, InspectorToolCallResp, InspectorToolsListData, InspectorToolsListResp,
};
use crate::api::routes::AppState;
use crate::inspector::registry::{CallRegistry, CallStatus};
// no direct SseEvent import needed; we treat events as opaque JSON
use crate::inspector::{bus, service};
// no extra collections needed now

// Inline wait window (ms); adjust via config API in the future if needed.
const INSPECTOR_WAIT_INLINE_MS: u64 = 10_000;
// SSE stream lifetime guard (ms); terminate stream past TTL
const INSPECTOR_SSE_TTL_MS: u64 = 10 * 60 * 1_000; // 10 minutes
// SSE single-event maximum payload size (bytes); oversize events are truncated
const INSPECTOR_SSE_EVENT_MAX_BYTES: usize = 2 * 1024 * 1024; // 2 MiB

// ==============================
// Tools
// ==============================

pub async fn tools_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorListQuery>,
) -> Result<Json<InspectorToolsListResp>, ApiError> {
    let data_value = service::list_tools(&state, &query).await?;
    // Convert Value into strongly typed data
    let mode = data_value
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let tools: Vec<Value> = match data_value.get("tools").cloned() {
        Some(Value::Array(v)) => v,
        _ => Vec::new(),
    };
    let total = data_value
        .get("total")
        .and_then(|v| v.as_u64())
        .unwrap_or(tools.len() as u64) as usize;
    let meta = data_value.get("meta").and_then(|v| v.as_array()).cloned();
    let data = InspectorToolsListData {
        mode,
        tools,
        total,
        meta,
    };
    Ok(Json(InspectorToolsListResp::success(data)))
}

pub async fn tool_call(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorToolCallReq>,
) -> Result<Json<InspectorToolCallResp>, ApiError> {
    let call_id = service::start_tool_call(&state, &req).await?;
    if let Some(inline) = service::wait_tool_result_inline(&call_id, INSPECTOR_WAIT_INLINE_MS).await {
        // inline is either:
        //  - { success:true, call_id, message, data:{result:?} }
        //  - { success:false, call_id, error:{message,...} }
        let ok = inline.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        if ok {
            let data_v = inline.get("data").cloned().unwrap_or_else(|| json!({}));
            let message = data_v
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("completed")
                .to_string();
            let result: Option<Value> = data_v.get("result").cloned();
            let server_id = data_v.get("server_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let elapsed_ms = data_v.get("elapsed_ms").and_then(|v| v.as_u64());
            let data = InspectorToolCallData {
                call_id: call_id.clone(),
                message,
                result,
                server_id,
                elapsed_ms,
            };
            return Ok(Json(InspectorToolCallResp::success(data)));
        } else {
            let msg = inline
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("tool call failed");
            return Ok(Json(InspectorToolCallResp::error_simple("tool_error", msg)));
        }
    }
    Ok(Json(InspectorToolCallResp::success(InspectorToolCallData {
        call_id,
        message: "accepted".into(),
        result: None,
        server_id: None,
        elapsed_ms: None,
    })))
}

// ==============================
// Prompts
// ==============================

pub async fn prompts_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorListQuery>,
) -> Result<Json<InspectorPromptsListResp>, ApiError> {
    let data_value = service::list_prompts(&state, &query).await?;
    let mode = data_value
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let prompts: Vec<Value> = match data_value.get("prompts").cloned() {
        Some(Value::Array(v)) => v,
        _ => Vec::new(),
    };
    let total = data_value
        .get("total")
        .and_then(|v| v.as_u64())
        .unwrap_or(prompts.len() as u64) as usize;
    let meta = data_value.get("meta").and_then(|v| v.as_array()).cloned();
    let data = InspectorPromptsListData {
        mode,
        prompts,
        total,
        meta,
    };
    Ok(Json(InspectorPromptsListResp::success(data)))
    // legacy code kept in VCS history
    // legacy code below kept for reference; will be removed after stabilization
    /*
    let refresh = if query.refresh {
        Some(crate::core::capability::runtime::RefreshStrategy::Force)
    } else {
        Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst)
    };
    let mut items: Vec<rmcp::model::Prompt> = Vec::new();

    if query.server_id.is_some() || query.server_name.is_some() {
        let server_id = resolve_server(&query.server_id, &query.server_name).await?;
        let ctx = crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::Prompts,
            server_id,
            refresh,
            timeout: Some(Duration::from_secs(10)),
            validation_session: None,
        };
        let res = crate::core::capability::runtime::list(
            &ctx,
            &state.redb_cache,
            &state.connection_pool,
            &state
                .database
                .as_ref()
                .ok_or(ApiError::InternalError("Database not available".into()))?
                .clone(),
        )
        .await
        .map_err(map_anyhow)?;
        items = res.items.into_prompts().unwrap_or_default();
    } else {
        let db = state
            .database
            .as_ref()
            .ok_or(ApiError::InternalError("Database not available".into()))?;
        let enabled_servers: Vec<(String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, sc.name, sc.capabilities
            FROM server_config sc
            JOIN profile_server ps ON ps.server_id = sc.id AND ps.enabled = 1
            JOIN profile p ON p.id = ps.profile_id AND p.is_active = 1
            WHERE sc.enabled = 1
            GROUP BY sc.id, sc.name, sc.capabilities
            "#,
        )
        .fetch_all(&db.pool)
        .await
        .unwrap_or_default();

        let mut tasks = Vec::new();
        for (server_id, _name, _caps) in enabled_servers {
            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Prompts,
                server_id: server_id.clone(),
                refresh,
                timeout: Some(Duration::from_secs(10)),
                validation_session: None,
            };
            let redb = state.redb_cache.clone();
            let pool = state.connection_pool.clone();
            let db_arc = state.database.as_ref().unwrap().clone();
            tasks.push(async move {
                match crate::core::capability::runtime::list(&ctx, &redb, &pool, &db_arc).await {
                    Ok(result) => result.items.into_prompts().unwrap_or_default(),
                    Err(_) => Vec::new(),
                }
            });
        }
        for mut v in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            items.append(&mut v);
        }
    }

    let data = json!({ "mode": format!("{:?}", query.mode).to_lowercase(), "prompts": items, "total": items.len() });
    Ok(Json(json!({"success": true, "data": data})))
    */
}

pub async fn prompt_get(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorPromptGetReq>,
) -> Result<Json<InspectorPromptGetResp>, ApiError> {
    let v = service::prompt_get(&state, &req).await?;
    let result: Value = v.get("result").cloned().unwrap_or_else(|| json!({}));
    let server_id = v
        .get("server_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let elapsed_ms = v.get("elapsed_ms").and_then(|v| v.as_u64()).unwrap_or_default();
    Ok(Json(InspectorPromptGetResp::success(InspectorPromptGetData {
        result,
        server_id,
        elapsed_ms,
    })))
}

// ==============================
// Resources
// ==============================

pub async fn resources_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorListQuery>,
) -> Result<Json<InspectorResourcesListResp>, ApiError> {
    let v = service::list_resources(&state, &query).await?;
    let mode = v.get("mode").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let resources: Vec<Value> = match v.get("resources").cloned() {
        Some(Value::Array(v)) => v,
        _ => Vec::new(),
    };
    let total = v
        .get("total")
        .and_then(|v| v.as_u64())
        .unwrap_or(resources.len() as u64) as usize;
    let meta = v.get("meta").and_then(|v| v.as_array()).cloned();
    Ok(Json(InspectorResourcesListResp::success(InspectorResourcesListData {
        mode,
        resources,
        total,
        meta,
    })))
}

pub async fn resource_read(
    State(state): State<Arc<AppState>>,
    Query(req): Query<InspectorResourceReadQuery>,
) -> Result<Json<InspectorResourceReadResp>, ApiError> {
    let v = service::resource_read(&state, &req).await?;
    let result: Value = v.get("result").cloned().unwrap_or_else(|| json!({}));
    let server_id = v.get("server_id").cloned().and_then(|vv| {
        if vv.is_null() {
            None
        } else {
            vv.as_str().map(|s| s.to_string())
        }
    });
    let elapsed_ms = v.get("elapsed_ms").and_then(|v| v.as_u64()).unwrap_or_default();
    Ok(Json(InspectorResourceReadResp::success(InspectorResourceReadData {
        result,
        server_id,
        elapsed_ms,
    })))
}

// ==============================
// SSE & Cancel & Recent Calls
// ==============================

pub async fn tool_call_stream(
    _state: State<Arc<AppState>>,
    Query(q): Query<HashMap<String, String>>,
) -> axum::response::Sse<
    futures::stream::BoxStream<'static, Result<axum::response::sse::Event, std::convert::Infallible>>,
> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use futures::stream::{self, BoxStream};
    use std::convert::Infallible;

    let bus = bus::global();
    let call_id = q.get("call_id").cloned().unwrap_or_default();

    let stream: BoxStream<'static, Result<Event, Infallible>> = if let Some(rx0) = bus.subscribe(&call_id).await {
        let call_id_clone = call_id.clone();
        let bus_clone = bus.clone();
        let start_at = std::time::Instant::now();
        let unfold = stream::unfold(rx0, move |mut rx| {
            let call_id_local = call_id_clone.clone();
            let bus_local = bus_clone.clone();
            async move {
                // TTL guard
                if start_at.elapsed() >= Duration::from_millis(INSPECTOR_SSE_TTL_MS) {
                    // end stream and cleanup channel
                    bus_local.finish(&call_id_local).await;
                    return None;
                }
                match rx.recv().await {
                    Ok(mut ev) => {
                        // Event size guard: clamp oversize data
                        if let Some(data) = ev.data.take() {
                            if let Ok(len) = serde_json::to_vec(&data).map(|v| v.len()) {
                                if len > INSPECTOR_SSE_EVENT_MAX_BYTES {
                                    tracing::warn!(
                                        call_id=%call_id_local,
                                        size=len,
                                        limit=INSPECTOR_SSE_EVENT_MAX_BYTES,
                                        "SSE event data too large; truncating"
                                    );
                                    ev.data = Some(json!({
                                        "truncated": true,
                                        "omitted_bytes": len,
                                        "note": "event data exceeds limit"
                                    }));
                                } else {
                                    ev.data = Some(data);
                                }
                            } else {
                                ev.data = Some(data);
                            }
                        }
                        let mut e = Event::default()
                            .event(ev.event.to_string())
                            .data(serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string()));
                        if let Some(seq) = ev.seq {
                            e = e.id(seq.to_string());
                        }
                        Some((Ok::<Event, Infallible>(e), rx))
                    }
                    Err(_) => None,
                }
            }
        });
        Box::pin(unfold)
    } else {
        Box::pin(stream::once(async move {
            let ev = Event::default()
                .event("error")
                .data("{\"message\":\"call_id not found\"}");
            Ok::<Event, Infallible>(ev)
        }))
    };

    Sse::new(stream).keep_alive(KeepAlive::new().text("keep-alive").interval(Duration::from_secs(15)))
}

pub async fn tool_call_cancel(
    _state: State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let Some(call_id) = body.get("call_id").and_then(|v| v.as_str()) else {
        return Err(ApiError::BadRequest("call_id is required".into()));
    };
    service::cancel_tool_call(call_id).await;
    Ok(Json(json!({"success": true})))
}

pub async fn calls_recent(
    _state: State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(50);
    let reg = CallRegistry::global();
    let list = reg.recent(limit).await;
    let items: Vec<Value> = list.into_iter().map(|s| json!({
        "call_id": s.call_id,
        "mode": s.mode,
        "capability": s.capability,
        "action": s.action,
        "target": s.target,
        "status": match s.status { CallStatus::Pending=>"pending", CallStatus::Running=>"running", CallStatus::Ok=>"ok", CallStatus::Error=>"error", CallStatus::Cancelled=>"cancelled" },
        "started_at": s.started_at.to_rfc3339(),
        "finished_at": s.finished_at.map(|t| t.to_rfc3339()),
        "elapsed_ms": s.elapsed_ms,
        "progress": s.progress,
        "last_seq": s.last_seq,
        "last_error": s.last_error,
    })).collect();
    Ok(Json(json!({"success": true, "data": {"items": items}})))
}

pub async fn calls_details(
    _state: State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let Some(call_id) = params.get("call_id").cloned() else {
        return Err(ApiError::BadRequest("call_id is required".into()));
    };
    let reg = CallRegistry::global();
    match reg.get(&call_id).await {
        Some(s) => Ok(Json(json!({"success": true, "data": {
            "call_id": s.call_id,
            "mode": s.mode,
            "capability": s.capability,
            "action": s.action,
            "target": s.target,
            "status": match s.status { CallStatus::Pending=>"pending", CallStatus::Running=>"running", CallStatus::Ok=>"ok", CallStatus::Error=>"error", CallStatus::Cancelled=>"cancelled" },
            "started_at": s.started_at.to_rfc3339(),
            "finished_at": s.finished_at.map(|t| t.to_rfc3339()),
            "elapsed_ms": s.elapsed_ms,
            "progress": s.progress,
            "last_seq": s.last_seq,
            "last_error": s.last_error,
        }}))),
        None => Err(ApiError::NotFound("call not found".into())),
    }
}

pub async fn calls_clear(_state: State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let reg = CallRegistry::global();
    reg.clear().await;
    Ok(Json(json!({"success": true})))
}
