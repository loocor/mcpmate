// no StreamExt needed in this module
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State, ws::{Message, WebSocket, WebSocketUpgrade}},
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
};
use futures::{StreamExt, SinkExt};
use serde_json::{Value, json};
use std::{convert::Infallible, time::Duration};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

use crate::api::handlers::ApiError;
// use crate::api::models::inspector::InspectorMode;
use crate::api::models::inspector::{
    InspectorCallEventsQuery, InspectorListQuery, InspectorPromptGetData, InspectorPromptGetReq,
    InspectorPromptGetResp, InspectorPromptsListData, InspectorPromptsListResp, InspectorResourceReadData,
    InspectorResourceReadQuery, InspectorResourceReadResp, InspectorResourcesListData, InspectorResourcesListResp,
    InspectorSessionCloseData, InspectorSessionCloseReq, InspectorSessionCloseResp, InspectorSessionOpenReq,
    InspectorSessionOpenResp, InspectorTemplatesListData, InspectorTemplatesListResp, InspectorToolCallCancelData,
    InspectorToolCallCancelReq, InspectorToolCallCancelResp, InspectorToolCallData, InspectorToolCallReq,
    InspectorToolCallResp, InspectorToolCallStartData, InspectorToolCallStartResp, InspectorToolsListData,
    InspectorToolsListResp,
};
use crate::api::routes::AppState;
use crate::inspector::{calls::CallSubscription, service};

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
    // Preflight: When neither server_id nor server_name is provided, avoid hitting naming resolver
    if req.server_id.is_none() && req.server_name.is_none() {
        let resp = InspectorToolCallResp::error_simple("bad_request", "server_id or server_name is required");
        return Ok(Json(resp));
    }

    // Execute and normalize errors as 200 with structured error payload (Inspector UX contract)
    match service::call_tool(&state, &req).await {
        Ok(outcome) => {
            let data = InspectorToolCallData {
                message: outcome.message.unwrap_or_else(|| "completed".to_string()),
                result: outcome.result,
                server_id: outcome.server_id,
                elapsed_ms: Some(outcome.elapsed_ms),
            };
            Ok(Json(InspectorToolCallResp::success(data)))
        }
        Err(err) => {
            // Map handler error to client model without changing HTTP status
            let (code, message) = match &err {
                ApiError::NotFound(m) => ("not_found", m.as_str()),
                ApiError::BadRequest(m) => ("bad_request", m.as_str()),
                ApiError::InternalError(m) => ("internal_error", m.as_str()),
                ApiError::Conflict(m) => ("conflict", m.as_str()),
                ApiError::Forbidden(m) => ("forbidden", m.as_str()),
                ApiError::Timeout(m) => ("timeout", m.as_str()),
            };
            let resp = InspectorToolCallResp::error_simple(code, message);
            Ok(Json(resp))
        }
    }
}

pub async fn tool_call_start(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorToolCallReq>,
) -> Result<Json<InspectorToolCallStartResp>, ApiError> {
    tracing::info!(
        tool = %req.tool,
        mode = ?req.mode,
        server_id = ?req.server_id,
        server_name = ?req.server_name,
        session_id = ?req.session_id,
        has_arguments = req.arguments.is_some(),
        "Inspector tool_call_start request received"
    );

    let info = service::start_tool_call(&state, &req).await?;

    tracing::info!(
        call_id = %info.call_id,
        "Inspector tool call started successfully"
    );

    let data = InspectorToolCallStartData {
        call_id: info.call_id,
        server_id: info.server_id,
        mode: info.mode,
        session_id: info.session_id,
        request_id: info.request_id,
        progress_token: info.progress_token,
    };
    Ok(Json(InspectorToolCallStartResp::success(data)))
}

pub async fn tool_call_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorCallEventsQuery>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    tracing::info!(
        call_id = %query.call_id,
        "SSE connection request received for inspector call"
    );

    let subscription = state.inspector_calls.subscribe(&query.call_id).await.ok_or_else(|| {
        tracing::warn!(
            call_id = %query.call_id,
            "Inspector call not found for SSE subscription"
        );
        ApiError::NotFound(format!("Inspector call '{}' not found", query.call_id))
    })?;

    let call_id_clone = query.call_id.clone();
    let stream: futures::stream::BoxStream<'static, Result<Event, Infallible>> = match subscription {
        CallSubscription::Active(receiver) => {
            tracing::info!(
                call_id = %call_id_clone,
                "SSE subscription established with Active receiver"
            );
            BroadcastStream::new(receiver)
                .filter_map(move |msg| {
                    let call_id = call_id_clone.clone();
                    async move {
                        match msg {
                            Ok(event) => {
                                tracing::info!(
                                    call_id = %call_id,
                                    event = ?event,
                                    "SSE broadcasting inspector event"
                                );
                                match Event::default().json_data(&event) {
                                    Ok(ev) => Some(Ok::<Event, Infallible>(ev)),
                                    Err(err) => {
                                        tracing::warn!(error = %err, "Failed to serialize inspector event");
                                        None
                                    }
                                }
                            }
                            Err(err) => {
                                tracing::warn!(
                                    call_id = %call_id,
                                    error = ?err,
                                    "Inspector events stream lagged"
                                );
                                None
                            }
                        }
                    }
                })
                .boxed()
        }
        CallSubscription::Completed(event) => {
            tracing::info!(
                call_id = %query.call_id,
                event = ?event,
                "SSE subscription for already completed call, sending terminal event"
            );
            let event_opt = match Event::default().json_data(&event) {
                Ok(ev) => Some(ev),
                Err(err) => {
                    tracing::warn!(error = %err, "Failed to serialize inspector terminal event");
                    None
                }
            };
            futures::stream::iter(event_opt.into_iter().map(Ok::<Event, Infallible>)).boxed()
        }
    };

    tracing::info!(
        call_id = %query.call_id,
        "SSE stream created, returning to client"
    );

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive")))
}

/// WebSocket handler for inspector tool call events.
/// This provides a more reliable alternative to SSE for Tauri/WKWebView environments.
pub async fn tool_call_events_ws(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
    Query(query): Query<InspectorCallEventsQuery>,
) -> impl IntoResponse {
    tracing::info!(
        call_id = %query.call_id,
        "WebSocket connection request received for inspector call"
    );

    let call_id = query.call_id.clone();
    let subscription = state.inspector_calls.subscribe(&query.call_id).await;

    match subscription {
        Some(sub) => {
            tracing::info!(call_id = %call_id, "WebSocket subscription established");
            ws.on_upgrade(move |socket| handle_ws_events(socket, call_id, sub))
        }
        None => {
            tracing::warn!(
                call_id = %query.call_id,
                "Inspector call not found for WebSocket subscription"
            );
            ws.on_upgrade(move |mut socket| async move {
                let _ = socket.close().await;
            })
        }
    }
}

/// Handle WebSocket events for a specific call_id
async fn handle_ws_events(
    socket: WebSocket,
    call_id: String,
    subscription: crate::inspector::calls::CallSubscription,
) {
    use tokio::sync::broadcast;

    let (mut sender, _receiver) = socket.split();

    // Subscribe to broadcast channel based on subscription type
    let rx: broadcast::Receiver<crate::inspector::calls::InspectorEvent> = match subscription {
        CallSubscription::Active(rx) => rx,
        CallSubscription::Completed(event) => {
            // Send the completed event immediately and close
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = sender.send(Message::Text(json.into())).await;
            }
            let _ = sender.close().await;
            return;
        }
    };

    // Use BroadcastStream to convert broadcast to Stream
    let mut stream = BroadcastStream::new(rx);

    // Send events from broadcast channel to WebSocket
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                match serde_json::to_string(&event) {
                    Ok(json) => {
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to serialize inspector event for WS");
                    }
                }
            }
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_)) => {
                // Skip lagged messages
                continue;
            }
        }
    }

    tracing::info!(call_id = %call_id, "WebSocket connection closed");
}

pub async fn tool_call_cancel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorToolCallCancelReq>,
) -> Result<Json<InspectorToolCallCancelResp>, ApiError> {
    state
        .inspector_calls
        .cancel_call(&req.call_id, req.reason.clone())
        .await
        .map_err(ApiError::BadRequest)?;

    Ok(Json(InspectorToolCallCancelResp::success(
        InspectorToolCallCancelData { cancelled: true },
    )))
}

pub async fn session_open(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorSessionOpenReq>,
) -> Result<Json<InspectorSessionOpenResp>, ApiError> {
    let data = service::open_session(&state, &req).await?;
    Ok(Json(InspectorSessionOpenResp::success(data)))
}

pub async fn session_close(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorSessionCloseReq>,
) -> Result<Json<InspectorSessionCloseResp>, ApiError> {
    let closed = service::close_session(&state, &req).await?;
    Ok(Json(InspectorSessionCloseResp::success(InspectorSessionCloseData {
        closed,
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
// Templates
// ==============================

pub async fn templates_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorListQuery>,
) -> Result<Json<InspectorTemplatesListResp>, ApiError> {
    let data_value = service::list_templates(&state, &query).await?;
    let mode = data_value
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let templates: Vec<Value> = match data_value.get("templates").cloned() {
        Some(Value::Array(v)) => v,
        _ => Vec::new(),
    };
    let total = data_value
        .get("total")
        .and_then(|v| v.as_u64())
        .unwrap_or(templates.len() as u64) as usize;
    let meta = data_value.get("meta").and_then(|v| v.as_array()).cloned();
    let data = InspectorTemplatesListData {
        mode,
        templates,
        total,
        meta,
    };
    Ok(Json(InspectorTemplatesListResp::success(data)))
}
