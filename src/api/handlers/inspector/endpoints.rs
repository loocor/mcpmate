// no StreamExt needed in this module
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use futures::StreamExt;
use serde_json::{Value, json};
use std::{convert::Infallible, time::Duration};
use tokio_stream::wrappers::BroadcastStream;

use crate::api::handlers::ApiError;
// use crate::api::models::inspector::InspectorMode;
use crate::api::models::inspector::{
    InspectorCallEventsQuery, InspectorListQuery, InspectorPromptGetData, InspectorPromptGetReq,
    InspectorPromptGetResp, InspectorPromptsListData, InspectorPromptsListResp, InspectorResourceReadData,
    InspectorResourceReadQuery, InspectorResourceReadResp, InspectorResourcesListData, InspectorResourcesListResp,
    InspectorSessionCloseData, InspectorSessionCloseReq, InspectorSessionCloseResp, InspectorSessionOpenReq,
    InspectorSessionOpenResp, InspectorToolCallCancelData, InspectorToolCallCancelReq, InspectorToolCallCancelResp,
    InspectorToolCallData, InspectorToolCallReq, InspectorToolCallResp, InspectorToolCallStartData,
    InspectorToolCallStartResp, InspectorToolsListData, InspectorToolsListResp,
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
    let outcome = service::call_tool(&state, &req).await?;
    let data = InspectorToolCallData {
        message: outcome.message.unwrap_or_else(|| "completed".to_string()),
        result: outcome.result,
        server_id: outcome.server_id,
        elapsed_ms: Some(outcome.elapsed_ms),
    };
    Ok(Json(InspectorToolCallResp::success(data)))
}

pub async fn tool_call_start(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InspectorToolCallReq>,
) -> Result<Json<InspectorToolCallStartResp>, ApiError> {
    let info = service::start_tool_call(&state, &req).await?;
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
    let subscription = state
        .inspector_calls
        .subscribe(&query.call_id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Inspector call '{}' not found", query.call_id)))?;

    let stream: futures::stream::BoxStream<'static, Result<Event, Infallible>> = match subscription {
        CallSubscription::Active(receiver) => BroadcastStream::new(receiver)
            .filter_map(|msg| async move {
                match msg {
                    Ok(event) => match Event::default().json_data(&event) {
                        Ok(ev) => Some(Ok::<Event, Infallible>(ev)),
                        Err(err) => {
                            tracing::warn!(error = %err, "Failed to serialize inspector event");
                            None
                        }
                    },
                    Err(err) => {
                        tracing::warn!("Inspector events stream lagged: {}", err);
                        None
                    }
                }
            })
            .boxed(),
        CallSubscription::Completed(event) => {
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

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive")))
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

// no SSE / call history endpoints in synchronous mode
