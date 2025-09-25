use futures::StreamExt;
use serde_json::{Value, json};
use std::collections::HashSet;
use tokio::time::Duration;

use crate::api::handlers::ApiError;
use crate::api::models::inspector::{
    InspectorListQuery, InspectorMode, InspectorPromptGetReq, InspectorResourceReadQuery, InspectorToolCallReq,
};
use crate::api::routes::AppState;

use super::sse::{SseEvent, SseEventKind};
use super::{
    bus,
    registry::{CallRegistry, CallSummary},
};

// Public services called by API handlers

pub async fn list_tools(
    state: &AppState,
    query: &InspectorListQuery,
) -> Result<Value, ApiError> {
    let refresh = if query.refresh {
        Some(crate::core::capability::runtime::RefreshStrategy::Force)
    } else {
        Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst)
    };
    let mut tools_all: Vec<rmcp::model::Tool> = Vec::new();
    if query.server_id.is_some() || query.server_name.is_some() {
        let server_id = resolve_server(&query.server_id, &query.server_name).await?;
        let ctx = crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::Tools,
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
        tools_all = res.items.into_tools().unwrap_or_default();
    } else {
        let db = state
            .database
            .as_ref()
            .ok_or(ApiError::InternalError("Database not available".into()))?;
        let enabled_servers: Vec<(String, String, Option<String>)> = sqlx::query_as(
            r#"SELECT sc.id, sc.name, sc.capabilities FROM server_config sc JOIN profile_server ps ON ps.server_id = sc.id AND ps.enabled = 1 JOIN profile p ON p.id = ps.profile_id AND p.is_active = 1 WHERE sc.enabled = 1 GROUP BY sc.id, sc.name, sc.capabilities"#,
        ).fetch_all(&db.pool).await.unwrap_or_default();
        let mut tasks = Vec::new();
        for (server_id, _name, _caps) in enabled_servers {
            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Tools,
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
                    Ok(r) => r.items.into_tools().unwrap_or_default(),
                    Err(_) => Vec::new(),
                }
            });
        }
        for mut v in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            tools_all.append(&mut v);
        }
    }
    Ok(json!({ "mode": format!("{:?}", query.mode).to_lowercase(), "tools": tools_all, "total": tools_all.len() }))
}

pub async fn list_prompts(
    state: &AppState,
    query: &InspectorListQuery,
) -> Result<Value, ApiError> {
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
            r#"SELECT sc.id, sc.name, sc.capabilities FROM server_config sc JOIN profile_server ps ON ps.server_id = sc.id AND ps.enabled = 1 JOIN profile p ON p.id = ps.profile_id AND p.is_active = 1 WHERE sc.enabled = 1 GROUP BY sc.id, sc.name, sc.capabilities"#,
        ).fetch_all(&db.pool).await.unwrap_or_default();
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
                    Ok(r) => r.items.into_prompts().unwrap_or_default(),
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
    Ok(json!({ "mode": format!("{:?}", query.mode).to_lowercase(), "prompts": items, "total": items.len() }))
}

pub async fn list_resources(
    state: &AppState,
    query: &InspectorListQuery,
) -> Result<Value, ApiError> {
    let refresh = if query.refresh {
        Some(crate::core::capability::runtime::RefreshStrategy::Force)
    } else {
        Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst)
    };
    let mut items: Vec<rmcp::model::Resource> = Vec::new();
    if query.server_id.is_some() || query.server_name.is_some() {
        let server_id = resolve_server(&query.server_id, &query.server_name).await?;
        let ctx = crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::Resources,
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
        items = res.items.into_resources().unwrap_or_default();
    } else {
        let db = state
            .database
            .as_ref()
            .ok_or(ApiError::InternalError("Database not available".into()))?;
        let enabled_servers: Vec<(String, String, Option<String>)> = sqlx::query_as(
            r#"SELECT sc.id, sc.name, sc.capabilities FROM server_config sc JOIN profile_server ps ON ps.server_id = sc.id AND ps.enabled = 1 JOIN profile p ON p.id = ps.profile_id AND p.is_active = 1 WHERE sc.enabled = 1 GROUP BY sc.id, sc.name, sc.capabilities"#,
        ).fetch_all(&db.pool).await.unwrap_or_default();
        let mut tasks = Vec::new();
        for (server_id, _name, _caps) in enabled_servers {
            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Resources,
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
                    Ok(r) => r.items.into_resources().unwrap_or_default(),
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
    Ok(json!({ "mode": format!("{:?}", query.mode).to_lowercase(), "resources": items, "total": items.len() }))
}

pub async fn prompt_get(
    state: &AppState,
    req: &InspectorPromptGetReq,
) -> Result<Value, ApiError> {
    let (server_id, upstream_name) = match req.mode {
        InspectorMode::Proxy => {
            if let Ok((server_name, upstream)) = crate::core::capability::naming::resolve_unique_name(
                crate::core::capability::naming::NamingKind::Prompt,
                &req.name,
            )
            .await
            {
                let sid = crate::core::capability::resolver::to_id(&server_name)
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

    let mapping = crate::core::capability::facade::build_prompt_mapping(&state.connection_pool).await;
    let res = crate::core::capability::facade::get_upstream_prompt(
        &state.connection_pool,
        &mapping,
        &upstream_name,
        req.arguments.clone(),
        Some(&server_id),
    )
    .await
    .map_err(map_anyhow)?;
    Ok(json!({"result": res, "server_id": server_id}))
}

pub async fn resource_read(
    state: &AppState,
    req: &InspectorResourceReadQuery,
) -> Result<Value, ApiError> {
    let (server_filter, upstream_uri) = match req.mode {
        InspectorMode::Proxy => {
            if let Ok((server_name, upstream)) = crate::core::capability::naming::resolve_unique_name(
                crate::core::capability::naming::NamingKind::Resource,
                &req.uri,
            )
            .await
            {
                let sid = crate::core::capability::resolver::to_id(&server_name)
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
        crate::core::capability::facade::build_resource_mapping_filtered(
            &state.connection_pool,
            state.database.as_ref(),
            Some(&filter),
        )
        .await
    } else {
        crate::core::capability::facade::build_resource_mapping(&state.connection_pool, state.database.as_ref()).await
    };
    let res = crate::core::capability::facade::read_upstream_resource(
        &state.connection_pool,
        &mapping,
        &upstream_uri,
        server_filter.as_deref(),
    )
    .await
    .map_err(map_anyhow)?;
    Ok(json!({"result": res, "server_id": server_filter}))
}

pub async fn start_tool_call(
    state: &AppState,
    req: &InspectorToolCallReq,
) -> Result<String, ApiError> {
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
        let _ = publish(
            &bus_for_task,
            &call_id_clone,
            SseEventKind::Log,
            json!({"message":"queued"}),
            Some(1),
        )
        .await;
        let result = do_tool_call(&state_cloned, &req_cloned, &bus_for_task, &call_id_clone, timeout_ms).await;
        if let Err(e) = result {
            let _ = publish(
                &bus_for_task,
                &call_id_clone,
                SseEventKind::Error,
                json!({"message": e.to_string()}),
                None,
            )
            .await;
        }
        bus_for_task.finish(&call_id_clone).await;
    });
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
                            return Some(
                                json!({"success": true, "call_id": call_id, "message":"completed", "data": ev.data }),
                            );
                        }
                        SseEventKind::Error => {
                            return Some(json!({"success": false, "call_id": call_id, "error": ev.data}));
                        }
                        SseEventKind::Cancelled => {
                            return Some(
                                json!({"success": false, "call_id": call_id, "error": {"message":"cancelled"}}),
                            );
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
    let bus = bus::global();
    bus.cancel(call_id).await;
    let _ = publish(&bus, call_id, SseEventKind::Cancelled, json!({}), None).await;
    tokio::spawn({
        let bus = bus.clone();
        let id = call_id.to_string();
        async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            bus.finish(&id).await;
        }
    });
}

// Internal helpers

fn map_anyhow(e: anyhow::Error) -> ApiError {
    ApiError::InternalError(e.to_string())
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

async fn resolve_server(
    server_id: &Option<String>,
    server_name: &Option<String>,
) -> Result<String, ApiError> {
    if let Some(id) = server_id.clone() {
        return Ok(id);
    }
    if let Some(name) = server_name.clone() {
        return crate::core::capability::resolver::to_id(&name)
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
    bus: &bus::CallBus,
    call_id: &str,
    timeout_ms: u64,
) -> Result<(), ApiError> {
    let (server_id, upstream_tool_name) = match req.mode {
        InspectorMode::Proxy => {
            if let Ok((server_name, upstream)) = crate::core::capability::naming::resolve_unique_name(
                crate::core::capability::naming::NamingKind::Tool,
                &req.tool,
            )
            .await
            {
                let sid = crate::core::capability::resolver::to_id(&server_name)
                    .await
                    .ok()
                    .flatten()
                    .ok_or_else(|| ApiError::BadRequest(format!("Server '{}' not found", server_name)))?;
                (sid, upstream)
            } else {
                (
                    resolve_server(&req.server_id, &req.server_name).await?,
                    req.tool.clone(),
                )
            }
        }
        InspectorMode::Native => (
            resolve_server(&req.server_id, &req.server_name).await?,
            req.tool.clone(),
        ),
    };
    let timeout = Duration::from_millis(timeout_ms);
    let ctx = crate::core::capability::runtime::CallCtx {
        call_id: call_id.to_string(),
        server_id: server_id.clone(),
        tool_name: upstream_tool_name,
        timeout: Some(timeout),
        arguments: req.arguments.clone(),
    };
    let _ = publish(
        bus,
        call_id,
        SseEventKind::Progress,
        json!({"percent":5, "message":"connecting"}),
        None,
    )
    .await;
    match crate::core::capability::runtime::call_tool(&ctx, &state.connection_pool).await {
        Ok(result) => {
            let data = json!({"result": result});
            let _ = publish(bus, call_id, SseEventKind::Result, data, None).await;
            Ok(())
        }
        Err(e) => Err(map_anyhow(e)),
    }
}
