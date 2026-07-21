// Server tools handlers
// Provides handlers for server tool inspect endpoints

use crate::api::{
    handlers::ApiError,
    models::server::{ServerCapabilityMeta, ServerCapabilityReq, ServerToolsData, ServerToolsResp},
    routes::AppState,
};
use axum::{
    extract::{Query, State},
    response::Json,
};
use std::sync::Arc;

use super::capability::{CapabilityType, enrich_capability_items};
use super::common::{InspectQuery, create_inspect_response};

/// List all tools for a specific server
pub async fn server_tools(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerToolsResp>, ApiError> {
    let result = server_tools_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for server tools operation
async fn server_tools_core(
    request: &ServerCapabilityReq,
    state: &Arc<AppState>,
) -> Result<ServerToolsResp, ApiError> {
    // Convert ServerCapabilityReq to InspectQuery for compatibility with existing logic
    let query = InspectQuery {
        refresh: request.refresh.as_ref().map(|r| (*r).into()),
        format: None,
        include_meta: None,
        timeout: None,
    };

    // Validate and get server info using unified function
    let (db, server_info, params) = super::common::get_server_info_for_inspect(state, &request.id, &query).await?;

    // CapabilityReadService owns cache and on-demand discovery orchestration.
    let refresh = match params.refresh {
        Some(super::common::RefreshStrategy::Force) => Some(crate::core::capability::runtime::RefreshStrategy::Force),
        _ => Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
    };
    let service = crate::core::capability::read_service::CapabilityReadService::from_runtime(
        db.clone(),
        state.connection_pool.clone(),
    );
    let list_result = service
        .list(&crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::Tools,
            server_id: server_info.server_id.clone(),
            refresh,
            timeout: Some(std::time::Duration::from_secs(10)),
            validation_session: None,
            runtime_identity: None,
            connection_selection: None,
            visibility_snapshot: None,
            name_domain: crate::core::capability::runtime::NameDomain::External,
        })
        .await
        .map_err(|error| {
            tracing::error!(server_id = %server_info.server_id, error = %error, "Failed to list tools");
            crate::core::capability::service::map_capability_read_error(&error)
        })?;
    let crate::core::capability::runtime::ListResult { items, meta } = list_result;
    let tool_items = match items {
        crate::core::capability::runtime::CapabilityItems::Tools(items) => items,
        _ => {
            tracing::error!("Capability read service returned non-tool items for tool capability");
            return Err(ApiError::InternalError(
                "Capability read service returned non-tool items for tool capability".to_string(),
            ));
        }
    };
    let json_items = tool_items
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            tracing::error!(server_id = %server_info.server_id, error = %error, "Failed to serialize tools");
            ApiError::InternalError(format!("Failed to serialize tools: {error}"))
        })?;
    let response = create_inspect_response(json_items, meta.cache_hit, params.refresh, meta.source.as_str());
    let tools_resp = ServerToolsResp::success(json_to_server_tools_resp(response));
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
) -> Result<ServerToolsResp, ApiError> {
    let Some(data) = response.data.take() else {
        return Ok(response);
    };

    let items = enrich_capability_items(CapabilityType::Tools, &db.pool, server_id, data.items)
        .await
        .map_err(|error| {
            tracing::error!(server_id = %server_id, error = %error, "Tool naming projection failed");
            ApiError::InternalError(format!("Tool naming projection failed: {error}"))
        })?;

    response.data = Some(ServerToolsData {
        items,
        state: data.state,
        meta: data.meta,
    });
    Ok(response)
}
