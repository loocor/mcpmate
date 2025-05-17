// Tool detail handlers

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};

use super::common::{get_context, get_tool_status};
use crate::api::{
    handlers::ApiError,
    models::tool::{ToolConfigRequest, ToolConfigResponse},
    routes::AppState,
};

/// Get a specific MCP tool configuration info
pub async fn info(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<ToolConfigResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (_proxy, db) = get_context(&state).await?;

    // Check if the server exists
    let server = crate::conf::operations::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{server_name}' not found"
        )));
    }

    // Get tool status (ID, prefixed name, enabled status)
    let (tool_id, prefixed_name, enabled) =
        get_tool_status(&db.pool, &server_name, &tool_name).await?;

    // Create tool configuration response
    let response = ToolConfigResponse {
        id: tool_id,
        server_name: server_name.clone(),
        tool_name: tool_name.clone(),
        prefixed_name,
        enabled,
        allowed_operations: vec![if enabled {
            "disable".to_string()
        } else {
            "enable".to_string()
        }],
    };

    Ok(Json(response))
}

/// Update a specific MCP tool configuration
pub async fn update(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
    Json(request): Json<ToolConfigRequest>,
) -> Result<Json<ToolConfigResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (_proxy, db) = get_context(&state).await?;

    // Check if the server exists
    let server = crate::conf::operations::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{server_name}' not found"
        )));
    }

    // Get tool status (ID, prefixed name, enabled status)
    let (tool_id, current_prefixed_name, current_enabled) =
        get_tool_status(&db.pool, &server_name, &tool_name).await?;

    // Update the tool configuration
    let mut updated = false;

    // Update enabled status if it changed
    if request.enabled != current_enabled {
        crate::conf::operations::tool::set_tool_enabled(
            &db.pool,
            &server_name,
            &tool_name,
            request.enabled,
        )
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to update tool enabled status: {e}"))
        })?;
        updated = true;
    }

    // Update prefixed name if it changed
    if request.prefixed_name != current_prefixed_name {
        crate::conf::operations::tool::update_tool_prefixed_name(
            &db.pool,
            &server_name,
            &tool_name,
            request.prefixed_name.clone(),
        )
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to update tool prefixed name: {e}"))
        })?;
        updated = true;
    }

    // Create response
    let response = ToolConfigResponse {
        id: tool_id,
        server_name: server_name.clone(),
        tool_name: tool_name.clone(),
        prefixed_name: request.prefixed_name,
        enabled: request.enabled,
        allowed_operations: vec![if request.enabled {
            "disable".to_string()
        } else {
            "enable".to_string()
        }],
    };

    // Log the update
    if updated {
        tracing::info!(
            "Updated tool configuration for '{}' from server '{}'",
            tool_name,
            server_name
        );
    } else {
        tracing::info!(
            "No changes to tool configuration for '{}' from server '{}'",
            tool_name,
            server_name
        );
    }

    Ok(Json(response))
}
