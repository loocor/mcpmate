// Server tools handlers
// Provides handlers for server tool inspect endpoints

use crate::api::{
    models::server::{ServerCapabilityMeta, ServerCapabilityReq, ServerToolsData, ServerToolsResp},
    routes::AppState,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use super::capability::{CapabilityType, enrich_capability_items};
use super::common::{InspectQuery, create_inspect_response};

/// List all tools for a specific server
pub async fn server_tools(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerToolsResp>, StatusCode> {
    let result = server_tools_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for server tools operation
async fn server_tools_core(
    request: &ServerCapabilityReq,
    state: &Arc<AppState>,
) -> Result<ServerToolsResp, StatusCode> {
    // Convert ServerCapabilityReq to InspectQuery for compatibility with existing logic
    let query = InspectQuery {
        refresh: request.refresh.as_ref().map(|r| (*r).into()),
        format: None,
        include_meta: None,
        timeout: None,
    };

    // Validate and get server info using unified function
    let (db, server_info, params) = super::common::get_server_info_for_inspect(state, &request.id, &query).await?;

    // Check if server supports tools capability
    if let Some(response) = super::common::check_capability_or_error(
        &db.pool,
        &server_info,
        crate::common::capability::CapabilityToken::Tools,
        &params,
    )
    .await
    {
        let tools_resp = ServerToolsResp::success(json_to_server_tools_resp(response));
        return project_tool_response(&db, &server_info.server_id, tools_resp).await;
    }

    // Through CapabilityService: handler only requests result; service orchestrates cache/runtime/temp
    let refresh = match params.refresh {
        Some(super::common::RefreshStrategy::Force) => Some(crate::core::capability::runtime::RefreshStrategy::Force),
        _ => Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
    };
    let service = crate::core::capability::CapabilityService::new(
        state.redb_cache.clone(),
        state.connection_pool.clone(),
        db.clone(),
    );
    let list_result = service
        .list(&crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::Tools,
            server_id: server_info.server_id.clone(),
            refresh,
            timeout: Some(std::time::Duration::from_secs(10)),
            validation_session: Some(crate::core::capability::service::CAPABILITY_VALIDATION_SESSION.to_string()),
            runtime_identity: None,
            connection_selection: None,
            name_domain: crate::core::capability::runtime::NameDomain::External,
        })
        .await;
    match list_result {
        Ok(list_result) => {
            let crate::core::capability::runtime::ListResult { items, meta } = list_result;
            if let Some(tool_items) = items.into_tools() {
                if !tool_items.is_empty() {
                    let json_items: Vec<serde_json::Value> = tool_items
                        .into_iter()
                        .map(|tool| serde_json::to_value(tool).unwrap_or(serde_json::Value::Null))
                        .collect();
                    let response =
                        create_inspect_response(json_items, meta.cache_hit, params.refresh, meta.source.as_str());
                    let tools_resp = ServerToolsResp::success(json_to_server_tools_resp(response));
                    return project_tool_response(&db, &server_info.server_id, tools_resp).await;
                }
            } else {
                tracing::error!("Capability runtime returned non-tool items for tool capability");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }

            // Temporary instance is handled inside CapabilityService; handler does not touch pool
        }
        Err(e) => {
            tracing::error!("Failed to list tools via unified entry: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // No handler-side temporary creation

    // No offline fallback to avoid stale uncertainty; return empty

    // No data available
    let result = create_inspect_response(
        Vec::new(),
        false,
        params.refresh,
        crate::common::constants::strategies::NONE,
    );
    let tools_resp = ServerToolsResp::success(json_to_server_tools_resp(result));
    project_tool_response(&db, &server_info.server_id, tools_resp).await
}

/// Helper function to convert Json response to ServerToolsResp
fn json_to_server_tools_resp(json_response: axum::Json<serde_json::Value>) -> ServerToolsData {
    let json_value = json_response.0;

    let items = json_value
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let state = json_value
        .get("state")
        .and_then(|s| s.as_str())
        .unwrap_or("ok")
        .to_string();

    let meta_value = json_value.get("meta").cloned().unwrap_or_default();
    let meta = ServerCapabilityMeta {
        cache_hit: meta_value.get("cache_hit").and_then(|v| v.as_bool()).unwrap_or(false),
        strategy: meta_value
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        source: meta_value
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
    };

    ServerToolsData { items, state, meta }
}

async fn project_tool_response(
    db: &Arc<crate::config::database::Database>,
    server_id: &str,
    mut response: ServerToolsResp,
) -> Result<ServerToolsResp, StatusCode> {
    let Some(data) = response.data.take() else {
        return Ok(response);
    };

    let items = enrich_capability_items(CapabilityType::Tools, &db.pool, server_id, data.items)
        .await
        .map_err(|error| {
            tracing::error!(server_id = %server_id, error = %error, "Tool naming projection failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    response.data = Some(ServerToolsData {
        items,
        state: data.state,
        meta: data.meta,
    });
    Ok(response)
}
