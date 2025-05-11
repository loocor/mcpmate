// Tool action handlers

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::api::{
    handlers::ApiError,
    models::tool::ToolStatusResponse,
    routes::AppState,
};

use super::common::{get_context, get_tool_status};

/// Enable a specific MCP tool
pub async fn enable(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<ToolStatusResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (_proxy, db) = get_context(&state).await?;

    // Check if the server exists
    let server = crate::conf::operations::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {}", e)))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )));
    }

    // Get tool status (ID, prefixed name, enabled status)
    let (tool_id, _prefixed_name, enabled) =
        get_tool_status(&db.pool, &server_name, &tool_name).await?;

    // Check if the tool is already enabled
    if enabled {
        return Ok(Json(ToolStatusResponse {
            id: tool_id,
            server_name: server_name.clone(),
            tool_name: tool_name.clone(),
            result: "Already enabled".to_string(),
            status: "Enabled".to_string(),
            allowed_operations: vec!["disable".to_string()],
        }));
    }

    // Enable the tool
    crate::conf::operations::tool::set_tool_enabled(&db.pool, &server_name, &tool_name, true)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to enable tool: {}", e)))?;

    // Create response
    let response = ToolStatusResponse {
        id: tool_id,
        server_name: server_name.clone(),
        tool_name: tool_name.clone(),
        result: "Enabled".to_string(),
        status: "Enabled".to_string(),
        allowed_operations: vec!["disable".to_string()],
    };

    tracing::info!(
        "Enabled tool '{}' from server '{}'",
        tool_name,
        server_name
    );

    Ok(Json(response))
}

/// Disable a specific MCP tool
pub async fn disable(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<ToolStatusResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (_proxy, db) = get_context(&state).await?;

    // Check if the server exists
    let server = crate::conf::operations::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {}", e)))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )));
    }

    // Get tool status (ID, prefixed name, enabled status)
    let (tool_id, _prefixed_name, enabled) =
        get_tool_status(&db.pool, &server_name, &tool_name).await?;

    // Check if the tool is already disabled
    if !enabled {
        return Ok(Json(ToolStatusResponse {
            id: tool_id,
            server_name: server_name.clone(),
            tool_name: tool_name.clone(),
            result: "Already disabled".to_string(),
            status: "Disabled".to_string(),
            allowed_operations: vec!["enable".to_string()],
        }));
    }

    // Disable the tool
    crate::conf::operations::tool::set_tool_enabled(&db.pool, &server_name, &tool_name, false)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to disable tool: {}", e)))?;

    // Create response
    let response = ToolStatusResponse {
        id: tool_id,
        server_name: server_name.clone(),
        tool_name: tool_name.clone(),
        result: "Disabled".to_string(),
        status: "Disabled".to_string(),
        allowed_operations: vec!["enable".to_string()],
    };

    tracing::info!(
        "Disabled tool '{}' from server '{}'",
        tool_name,
        server_name
    );

    Ok(Json(response))
}

/// Refresh a specific server's tools
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<ToolStatusResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Check if the server exists
    let server = crate::conf::operations::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {}", e)))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )));
    }

    // Refresh the server's connections
    let mut connection_pool = proxy.connection_pool.lock().await;
    
    // Check if the server exists in the connection pool
    if !connection_pool.connections.contains_key(&server_name) {
        return Err(ApiError::NotFound(format!(
            "Server '{}' not found in connection pool",
            server_name
        )));
    }

    // Reconnect all instances of the server
    for (instance_id, conn) in connection_pool.connections.get_mut(&server_name).unwrap() {
        // Skip instances that are already connected
        if conn.is_connected() {
            tracing::info!(
                "Instance '{}' of server '{}' is already connected, skipping reconnection",
                instance_id,
                server_name
            );
            continue;
        }

        // Reconnect the instance
        tracing::info!(
            "Reconnecting instance '{}' of server '{}'",
            instance_id,
            server_name
        );
        
        // Increment connection attempts
        conn.connection_attempts += 1;
        
        // TODO: Implement actual reconnection logic
        // This would involve calling the appropriate connection function based on the server type
        // For now, we just log a message
        tracing::info!(
            "Reconnection of instance '{}' of server '{}' not implemented yet",
            instance_id,
            server_name
        );
    }

    // Create response
    let response = ToolStatusResponse {
        id: "0".to_string(), // Not applicable for server refresh
        server_name: server_name.clone(),
        tool_name: "".to_string(), // Not applicable for server refresh
        result: "Refreshed".to_string(),
        status: "Refreshed".to_string(),
        allowed_operations: vec!["refresh".to_string()],
    };

    tracing::info!("Refreshed server '{}'", server_name);

    Ok(Json(response))
}
