use std::sync::Arc;

use axum::{
    Json,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::{Value, json};
use tokio_stream::wrappers::BroadcastStream;

use crate::api::handlers::ApiError;
use crate::api::handlers::llm::{llm_manager, map_llm_error};
use crate::api::models::inspector::{
    InspectorCallEventsQuery, InspectorCapabilityPatchData, InspectorCapabilityPatchResp,
    InspectorCapabilityPatchUpsertReq, InspectorCompatibilitySnapshotData, InspectorCompatibilitySnapshotResp,
    InspectorListQuery, InspectorLlmEvaluationData, InspectorLlmEvaluationReq, InspectorLlmEvaluationResp,
    InspectorPackageSafetySnapshotData, InspectorPackageSafetySnapshotResp, InspectorPromptGetData,
    InspectorPromptGetReq, InspectorPromptGetResp, InspectorPromptsListData, InspectorPromptsListResp,
    InspectorResourceReadData, InspectorResourceReadQuery, InspectorResourceReadResp, InspectorResourcesListData,
    InspectorResourcesListResp, InspectorScratchServerCreateData, InspectorScratchServerCreateReq,
    InspectorScratchServerCreateResp, InspectorScratchServerDeleteData, InspectorScratchServerDeleteReq,
    InspectorScratchServerDeleteResp, InspectorScratchServerListData, InspectorScratchServerListResp,
    InspectorSessionCloseData, InspectorSessionCloseReq, InspectorSessionCloseResp, InspectorSessionOpenData,
    InspectorSessionOpenReq, InspectorSessionOpenResp, InspectorSessionRefreshReq, InspectorSessionRefreshResp,
    InspectorSnapshotQuery, InspectorTemplatesListData, InspectorTemplatesListResp, InspectorToolCallCancelData,
    InspectorToolCallCancelReq, InspectorToolCallCancelResp, InspectorToolCallData, InspectorToolCallEvidenceData,
    InspectorToolCallEvidenceQuery, InspectorToolCallEvidenceResp, InspectorToolCallReq, InspectorToolCallResp,
    InspectorToolCallStartData, InspectorToolCallStartResp, InspectorToolsListData, InspectorToolsListResp,
};
use crate::api::routes::AppState;
use crate::inspector::{calls::CallSubscription, context::InspectorServiceContext, service};

fn evidence_value(data_value: &Value) -> Option<Value> {
    data_value.get("evidence").cloned()
}

struct CapabilityListParts {
    mode: String,
    items: Vec<Value>,
    total: usize,
    meta: Option<Vec<Value>>,
    elapsed_ms: u64,
    evidence: Option<Value>,
}

fn value_array(
    data_value: &Value,
    key: &str,
) -> Vec<Value> {
    match data_value.get(key).cloned() {
        Some(Value::Array(items)) => items,
        _ => Vec::new(),
    }
}

fn value_string(
    data_value: &Value,
    key: &str,
) -> String {
    data_value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn value_u64(
    data_value: &Value,
    key: &str,
) -> u64 {
    data_value.get(key).and_then(Value::as_u64).unwrap_or_default()
}

fn nullable_string(
    data_value: &Value,
    key: &str,
) -> Option<String> {
    data_value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn capability_list_parts(
    data_value: &Value,
    items_key: &str,
) -> CapabilityListParts {
    let items = value_array(data_value, items_key);
    let total = data_value
        .get("total")
        .and_then(Value::as_u64)
        .unwrap_or(items.len() as u64) as usize;

    CapabilityListParts {
        mode: value_string(data_value, "mode"),
        items,
        total,
        meta: data_value.get("meta").and_then(Value::as_array).cloned(),
        elapsed_ms: value_u64(data_value, "elapsed_ms"),
        evidence: evidence_value(data_value),
    }
}

fn serialize_inspector_value<T: Serialize>(
    value: T,
    label: &str,
) -> Result<Value, ApiError> {
    serde_json::to_value(value)
        .map_err(|error| ApiError::InternalError(format!("Failed to serialize Inspector {label}: {error}")))
}

// ==============================
// Tools
// ==============================

pub async fn tools_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorListQuery>,
) -> Result<Json<InspectorToolsListResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let data_value = service::list_tools(&context, (&query).into()).await?;
    let parts = capability_list_parts(&data_value, "tools");
    let data = InspectorToolsListData {
        mode: parts.mode,
        tools: parts.items,
        total: parts.total,
        meta: parts.meta,
        elapsed_ms: parts.elapsed_ms,
        evidence: parts.evidence,
    };
    Ok(Json(InspectorToolsListResp::success(data)))
}

pub async fn tool_call(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorToolCallReq>,
) -> Result<Json<InspectorToolCallResp>, ApiError> {
    // Execute and normalize errors as 200 with structured error payload (Inspector UX contract)
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    match service::call_tool(&context, (&req).into()).await {
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
                ApiError::ServiceUnavailable(m) => ("service_unavailable", m.as_str()),
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
        proxy_mode = ?req.proxy_mode,
        proxy_scope = ?req.proxy_scope,
        has_arguments = req.arguments.is_some(),
        "Inspector tool_call_start request received"
    );

    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let info = service::start_tool_call(&context, (&req).into()).await?;

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

pub async fn tool_call_evidence(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorToolCallEvidenceQuery>,
) -> Result<Json<InspectorToolCallEvidenceResp>, ApiError> {
    let snapshot = state
        .inspector_calls
        .evidence_snapshot(&query.call_id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Inspector call '{}' not found", query.call_id)))?;
    let snapshot = serialize_inspector_value(snapshot, "evidence")?;

    Ok(Json(InspectorToolCallEvidenceResp::success(
        InspectorToolCallEvidenceData { snapshot },
    )))
}

pub async fn session_open(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorSessionOpenReq>,
) -> Result<Json<InspectorSessionOpenResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let info = service::open_session(&context, (&req).into()).await?;
    Ok(Json(InspectorSessionOpenResp::success(session_info_data(info))))
}

pub async fn session_close(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorSessionCloseReq>,
) -> Result<Json<InspectorSessionCloseResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let closed = service::close_session(&context, &req.session_id).await?;
    Ok(Json(InspectorSessionCloseResp::success(InspectorSessionCloseData {
        closed,
    })))
}

pub async fn session_refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorSessionRefreshReq>,
) -> Result<Json<InspectorSessionRefreshResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let info = service::refresh_session(&context, &req.session_id).await?;
    Ok(Json(InspectorSessionRefreshResp::success(session_info_data(info))))
}

fn session_info_data(info: crate::inspector::sessions::InspectorSessionInfo) -> InspectorSessionOpenData {
    InspectorSessionOpenData {
        session_id: info.session_id,
        target: (&info.target).into(),
        server_id: info.target.server_id().map(str::to_string),
        scratch_id: info.target.scratch_id().map(str::to_string),
        mode: info.target.mode(),
        proxy_mode: info.target.proxy_mode(),
        proxy_scope: info.target.proxy_scope(),
        expires_at_epoch_ms: info.expires_at_epoch_ms,
        handshake: info.handshake,
    }
}

// ==============================
// Compatibility
// ==============================

pub async fn compatibility_snapshot(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorSnapshotQuery>,
) -> Result<Json<InspectorCompatibilitySnapshotResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let snapshot = service::compatibility_snapshot(&context, (&query).into()).await?;
    Ok(Json(InspectorCompatibilitySnapshotResp::success(
        InspectorCompatibilitySnapshotData { snapshot },
    )))
}

pub async fn package_safety_snapshot(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorSnapshotQuery>,
) -> Result<Json<InspectorPackageSafetySnapshotResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let snapshot = service::package_safety_snapshot(&context, (&query).into()).await?;
    Ok(Json(InspectorPackageSafetySnapshotResp::success(
        InspectorPackageSafetySnapshotData { snapshot },
    )))
}

pub async fn capability_patch_upsert(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorCapabilityPatchUpsertReq>,
) -> Result<Json<InspectorCapabilityPatchResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let record = service::upsert_capability_patch(&context, (&req).into()).await?;
    let record = serialize_inspector_value(record, "capability patch")?;
    Ok(Json(InspectorCapabilityPatchResp::success(
        InspectorCapabilityPatchData { record },
    )))
}

pub async fn llm_evaluate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorLlmEvaluationReq>,
) -> Result<Json<InspectorLlmEvaluationResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let prepared = service::prepare_llm_evaluation(&context, (&req).into()).await?;
    let result = llm_manager(state)?
        .chat_completion(prepared.provider_id.as_deref(), prepared.chat_request.clone())
        .await
        .map_err(map_llm_error)?;
    let evaluation = service::finish_llm_evaluation(prepared, result.provider, result.response)?;
    Ok(Json(InspectorLlmEvaluationResp::success(InspectorLlmEvaluationData {
        evaluation,
    })))
}

// ==============================
// Scratch Servers
// ==============================

pub async fn scratch_server_list(
    State(state): State<Arc<AppState>>
) -> Result<Json<InspectorScratchServerListResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let records = service::list_scratch_server_records(&context)?;
    let total = records.len();
    let records = serialize_inspector_value(records, "scratch records")?
        .as_array()
        .cloned()
        .unwrap_or_default();
    Ok(Json(InspectorScratchServerListResp::success(
        InspectorScratchServerListData { records, total },
    )))
}

pub async fn scratch_server_create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorScratchServerCreateReq>,
) -> Result<Json<InspectorScratchServerCreateResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let input = (&req)
        .try_into()
        .map_err(|error| ApiError::BadRequest(format!("Invalid Inspector scratch server config: {}", error)))?;
    let record = service::create_scratch_server_record(&context, input)?;
    let record = serialize_inspector_value(record, "scratch record")?;
    Ok(Json(InspectorScratchServerCreateResp::success(
        InspectorScratchServerCreateData { record },
    )))
}

pub async fn scratch_server_delete(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorScratchServerDeleteReq>,
) -> Result<Json<InspectorScratchServerDeleteResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let deleted = service::delete_scratch_server_record(&context, &req.record_id)?;
    Ok(Json(InspectorScratchServerDeleteResp::success(
        InspectorScratchServerDeleteData {
            record_id: req.record_id,
            deleted,
        },
    )))
}

// ==============================
// Prompts
// ==============================

pub async fn prompts_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorListQuery>,
) -> Result<Json<InspectorPromptsListResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let data_value = service::list_prompts(&context, (&query).into()).await?;
    let parts = capability_list_parts(&data_value, "prompts");
    let data = InspectorPromptsListData {
        mode: parts.mode,
        prompts: parts.items,
        total: parts.total,
        meta: parts.meta,
        elapsed_ms: parts.elapsed_ms,
        evidence: parts.evidence,
    };
    Ok(Json(InspectorPromptsListResp::success(data)))
}

pub async fn prompt_get(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorPromptGetReq>,
) -> Result<Json<InspectorPromptGetResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let v = service::prompt_get(&context, (&req).into()).await?;
    let result: Value = v.get("result").cloned().unwrap_or_else(|| json!({}));
    let server_id = nullable_string(&v, "server_id");
    let elapsed_ms = value_u64(&v, "elapsed_ms");
    Ok(Json(InspectorPromptGetResp::success(InspectorPromptGetData {
        result,
        server_id,
        elapsed_ms,
        evidence: evidence_value(&v),
    })))
}

// ==============================
// Resources
// ==============================

pub async fn resources_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorListQuery>,
) -> Result<Json<InspectorResourcesListResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let v = service::list_resources(&context, (&query).into()).await?;
    let parts = capability_list_parts(&v, "resources");
    Ok(Json(InspectorResourcesListResp::success(InspectorResourcesListData {
        mode: parts.mode,
        resources: parts.items,
        total: parts.total,
        meta: parts.meta,
        elapsed_ms: parts.elapsed_ms,
        evidence: parts.evidence,
    })))
}

pub async fn resource_read(
    State(state): State<Arc<AppState>>,
    Query(req): Query<InspectorResourceReadQuery>,
) -> Result<Json<InspectorResourceReadResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let v = service::resource_read(&context, (&req).into()).await?;
    let result: Value = v.get("result").cloned().unwrap_or_else(|| json!({}));
    let server_id = nullable_string(&v, "server_id");
    let elapsed_ms = value_u64(&v, "elapsed_ms");
    Ok(Json(InspectorResourceReadResp::success(InspectorResourceReadData {
        result,
        server_id,
        elapsed_ms,
        evidence: evidence_value(&v),
    })))
}

// ==============================
// Templates
// ==============================

pub async fn templates_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InspectorListQuery>,
) -> Result<Json<InspectorTemplatesListResp>, ApiError> {
    let context = InspectorServiceContext::from_app_state(state.as_ref());
    let data_value = service::list_templates(&context, (&query).into()).await?;
    let parts = capability_list_parts(&data_value, "templates");
    let data = InspectorTemplatesListData {
        mode: parts.mode,
        templates: parts.items,
        total: parts.total,
        meta: parts.meta,
        elapsed_ms: parts.elapsed_ms,
        evidence: parts.evidence,
    };
    Ok(Json(InspectorTemplatesListResp::success(data)))
}
