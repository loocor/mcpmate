// MCPMate Proxy API handlers for MCP tool management
// Contains handler functions for MCP tool endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    api::{
        models::tool::{
            ToolConfigRequest, ToolConfigResponse, ToolListResponse, ToolResponse,
            ToolStatusResponse,
        },
        routes::AppState,
    },
    conf::operations,
};

use super::ApiError;

/// List all MCP tools
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ToolListResponse>, ApiError> {
    // Get the HTTP proxy server
    let proxy = state
        .http_proxy
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("HTTP proxy server not available".to_string()))?;

    // Check if database is available
    let db = proxy
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Get all available tools from the proxy server
    let connection_pool = proxy.connection_pool.lock().await;
    let mut all_tools = Vec::new();

    // Iterate through all servers and their tools
    for (server_name, instances) in connection_pool.connections.iter() {
        for (_, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance
            for tool in &conn.tools {
                let tool_name = tool.name.to_string();

                // Check if the tool is enabled
                let enabled = match operations::tool::is_tool_enabled(
                    &db.pool,
                    server_name,
                    &tool_name,
                )
                .await
                {
                    Ok(enabled) => enabled,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to check if tool is enabled: {}, assuming enabled",
                            e
                        );
                        true // Default to enabled if there's an error
                    }
                };

                // Get the tool ID from the database
                let tool_id = match operations::tool::get_tool_id(&db.pool, server_name, &tool_name)
                    .await
                {
                    Ok(Some(id)) => id,
                    Ok(None) => {
                        // If the tool doesn't have an ID in the database, use a placeholder
                        tracing::debug!(
                            "Tool '{}' from server '{}' doesn't have an ID in the database",
                            tool_name,
                            server_name
                        );
                        "0".to_string()
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to get tool ID for '{}' from server '{}': {}, using placeholder",
                            tool_name,
                            server_name,
                            e
                        );
                        "0".to_string()
                    }
                };

                // Get the prefixed name from the database
                let prefixed_name = match operations::tool::get_tool_prefixed_name(
                    &db.pool,
                    server_name,
                    &tool_name,
                )
                .await
                {
                    Ok(name) => name,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to get prefixed name for '{}' from server '{}': {}",
                            tool_name,
                            server_name,
                            e
                        );
                        None
                    }
                };

                // Create a tool response
                let tool_response = ToolResponse {
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

                all_tools.push(tool_response);
            }
        }
    }

    Ok(Json(ToolListResponse { tools: all_tools }))
}

/// Get a specific MCP tool configuration
pub async fn get_tool(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<ToolConfigResponse>, ApiError> {
    // Get the HTTP proxy server
    let proxy = state
        .http_proxy
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("HTTP proxy server not available".to_string()))?;

    // Check if database is available
    let db = proxy
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Check if the server exists
    let server = operations::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {}", e)))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )));
    }

    // Check if the tool is enabled
    let enabled = operations::is_tool_enabled(&db.pool, &server_name, &tool_name)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to check if tool is enabled: {}", e))
        })?;

    // Get the tool ID from the database
    let tool_id = match operations::tool::get_tool_id(&db.pool, &server_name, &tool_name).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            // If the tool doesn't have an ID in the database, use a placeholder
            tracing::debug!(
                "Tool '{}' from server '{}' doesn't have an ID in the database",
                tool_name,
                server_name
            );
            "0".to_string()
        }
        Err(e) => {
            tracing::warn!(
                "Failed to get tool ID for '{}' from server '{}': {}, using placeholder",
                tool_name,
                server_name,
                e
            );
            "0".to_string()
        }
    };

    // Get the prefixed name from the database
    let prefixed_name =
        match operations::tool::get_tool_prefixed_name(&db.pool, &server_name, &tool_name).await {
            Ok(name) => name,
            Err(e) => {
                tracing::warn!(
                    "Failed to get prefixed name for '{}' from server '{}': {}",
                    tool_name,
                    server_name,
                    e
                );
                None
            }
        };

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

/// Enable a specific MCP tool
pub async fn enable_tool(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<ToolStatusResponse>, ApiError> {
    // Get the HTTP proxy server
    let proxy = state
        .http_proxy
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("HTTP proxy server not available".to_string()))?;

    // Check if database is available
    let db = proxy
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Enable the tool
    let id = operations::tool::set_tool_enabled(&db.pool, &server_name, &tool_name, true)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to enable tool: {}", e)))?;

    // Notify clients about tool list change
    tracing::info!(
        "Tool '{}' from server '{}' has been enabled",
        tool_name,
        server_name
    );

    // Notify all connected clients about the tool list change
    // Note: We can't directly notify clients from the API server
    // This would require a more complex implementation with a shared connection pool
    // For now, we'll just log the change and rely on clients to refresh their tool list
    tracing::info!("Tool list has changed, clients will need to refresh their tool list");

    // Create tool status response
    let response = ToolStatusResponse {
        id: id.to_string(), // Convert i64 to String
        server_name,
        tool_name,
        result: "Successfully enabled tool".to_string(),
        status: "Enabled".to_string(),
        allowed_operations: vec!["disable".to_string()],
    };

    Ok(Json(response))
}

/// Disable a specific MCP tool
pub async fn disable_tool(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<ToolStatusResponse>, ApiError> {
    // Get the HTTP proxy server
    let proxy = state
        .http_proxy
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("HTTP proxy server not available".to_string()))?;

    // Check if database is available
    let db = proxy
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Disable the tool
    let id = operations::tool::set_tool_enabled(&db.pool, &server_name, &tool_name, false)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to disable tool: {}", e)))?;

    // Notify clients about tool list change
    tracing::info!(
        "Tool '{}' from server '{}' has been disabled",
        tool_name,
        server_name
    );

    // Notify all connected clients about the tool list change
    // Note: We can't directly notify clients from the API server
    // This would require a more complex implementation with a shared connection pool
    // For now, we'll just log the change and rely on clients to refresh their tool list
    tracing::info!("Tool list has changed, clients will need to refresh their tool list");

    // Create tool status response
    let response = ToolStatusResponse {
        id: id.to_string(), // Convert i64 to String
        server_name,
        tool_name,
        result: "Successfully disabled tool".to_string(),
        status: "Disabled".to_string(),
        allowed_operations: vec!["enable".to_string()],
    };

    Ok(Json(response))
}

/// Update a specific MCP tool configuration
pub async fn update_tool(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
    Json(request): Json<ToolConfigRequest>,
) -> Result<Json<ToolConfigResponse>, ApiError> {
    // Get the HTTP proxy server
    let proxy = state
        .http_proxy
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("HTTP proxy server not available".to_string()))?;

    // Check if database is available
    let db = proxy
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Enable or disable the tool
    let id =
        operations::tool::set_tool_enabled(&db.pool, &server_name, &tool_name, request.enabled)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to update tool enabled status: {}", e))
            })?;

    // Update the prefixed name if provided
    if request.prefixed_name.is_some() {
        operations::tool::update_tool_prefixed_name(
            &db.pool,
            &server_name,
            &tool_name,
            request.prefixed_name.clone(),
        )
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to update tool prefixed name: {}", e))
        })?;
    }

    // Notify clients about tool list change
    tracing::info!(
        "Tool '{}' from server '{}' has been updated, enabled: {}",
        tool_name,
        server_name,
        request.enabled
    );

    // Notify all connected clients about the tool list change
    // Note: We can't directly notify clients from the API server
    // This would require a more complex implementation with a shared connection pool
    // For now, we'll just log the change and rely on clients to refresh their tool list
    tracing::info!("Tool list has changed, clients will need to refresh their tool list");

    // Create tool configuration response
    let response = ToolConfigResponse {
        id: id.to_string(), // Convert i64 to String
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

    Ok(Json(response))
}
