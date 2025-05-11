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
            ServerToolsResponse, ToolConfigRequest, ToolConfigResponse, ToolListResponse,
            ToolResponse, ToolStatusResponse,
        },
        routes::AppState,
    },
    conf::operations,
    http::HttpProxyServer,
};

use super::ApiError;

/// Helper function to get HTTP proxy server and database from application state
///
/// This function extracts the HTTP proxy server and database from the application state,
/// handling common error cases and reducing code duplication.
///
/// # Arguments
/// * `state` - The application state
///
/// # Returns
/// * `Result<(&HttpProxyServer, &Database), ApiError>` - The HTTP proxy server and database, or an error
async fn get_context(
    state: &Arc<AppState>,
) -> Result<(&HttpProxyServer, &crate::conf::Database), ApiError> {
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

    Ok((proxy, db))
}

/// Helper function to get tool status (ID, prefixed name, enabled status)
async fn get_tool_status(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    server_name: &str,
    tool_name: &str,
) -> Result<(String, Option<String>, bool), ApiError> {
    // Check if the tool is enabled
    let enabled = match operations::tool::is_tool_enabled(pool, server_name, tool_name).await {
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
    let tool_id = match operations::tool::get_tool_id(pool, server_name, tool_name).await {
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
        match operations::tool::get_tool_prefixed_name(pool, server_name, tool_name).await {
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

    Ok((tool_id, prefixed_name, enabled))
}

/// Helper function to create a tool response
fn create_tool_response(
    server_name: &str,
    tool_name: &str,
    tool_id: String,
    prefixed_name: Option<String>,
    enabled: bool,
) -> ToolResponse {
    ToolResponse {
        id: tool_id,
        server_name: server_name.to_string(),
        tool_name: tool_name.to_string(),
        prefixed_name,
        enabled,
        allowed_operations: vec![if enabled {
            "disable".to_string()
        } else {
            "enable".to_string()
        }],
    }
}

/// Refresh the tool list for a specific server
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<ToolStatusResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, _db) = get_context(&state).await?;

    // Get instance IDs first (to avoid borrowing issues)
    let instance_ids = {
        let connection_pool = proxy.connection_pool.lock().await;

        // Check if the server exists
        if let Some(instances) = connection_pool.connections.get(&server_name) {
            // Collect instance IDs
            instances
                .iter()
                .filter_map(|(id, conn)| {
                    if conn.is_connected() {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
        } else {
            return Err(ApiError::NotFound(format!(
                "Server '{}' not found",
                server_name
            )));
        }
    };

    // Now reconnect each instance
    let mut reconnected = 0;
    for instance_id in &instance_ids {
        tracing::info!(
            "Reconnecting server '{}' instance '{}'",
            server_name,
            instance_id
        );

        // Get a new lock for each operation
        let mut connection_pool = proxy.connection_pool.lock().await;

        // Disconnect first
        if let Err(e) = connection_pool.disconnect(&server_name, instance_id).await {
            tracing::error!(
                "Failed to disconnect server '{}' instance '{}': {}",
                server_name,
                instance_id,
                e
            );
            continue;
        }

        // Release the lock
        drop(connection_pool);

        // Wait a moment before reconnecting
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Get a new lock for reconnect
        let mut connection_pool = proxy.connection_pool.lock().await;

        // Reconnect
        match connection_pool
            .trigger_connect(&server_name, instance_id)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "Successfully reconnected server '{}' instance '{}'",
                    server_name,
                    instance_id
                );
                reconnected += 1;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to reconnect server '{}' instance '{}': {}",
                    server_name,
                    instance_id,
                    e
                );
            }
        }
    }

    // Create response
    let response = ToolStatusResponse {
        id: "0".to_string(),
        server_name: server_name.clone(),
        tool_name: "all".to_string(),
        result: format!(
            "Successfully refreshed {} instances for server '{}'",
            reconnected, server_name
        ),
        status: "Refreshed".to_string(),
        allowed_operations: vec!["refresh".to_string()],
    };

    Ok(Json(response))
}

/// List all MCP tools
pub async fn all(State(state): State<Arc<AppState>>) -> Result<Json<ToolListResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

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

                // Get tool status (ID, prefixed name, enabled status)
                let (tool_id, prefixed_name, enabled) =
                    get_tool_status(&db.pool, server_name, &tool_name).await?;

                // Only include enabled tools
                if enabled {
                    // Create a tool response
                    let tool_response = create_tool_response(
                        server_name,
                        &tool_name,
                        tool_id,
                        prefixed_name,
                        enabled,
                    );

                    all_tools.push(tool_response);
                } else {
                    tracing::debug!(
                        "Skipping disabled tool '{}' from server '{}'",
                        tool_name,
                        server_name
                    );
                }
            }
        }
    }

    tracing::info!("Returning {} tools to client", all_tools.len());
    Ok(Json(ToolListResponse { tools: all_tools }))
}

/// Get a specific MCP tool configuration info
pub async fn info(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<ToolConfigResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (_proxy, db) = get_context(&state).await?;

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

/// Enable a specific MCP tool
pub async fn enable(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<ToolStatusResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (_proxy, db) = get_context(&state).await?;

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
pub async fn disable(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<ToolStatusResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (_proxy, db) = get_context(&state).await?;

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
pub async fn update(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
    Json(request): Json<ToolConfigRequest>,
) -> Result<Json<ToolConfigResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (_proxy, db) = get_context(&state).await?;

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

/// List all tools for a specific server
pub async fn list(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<ServerToolsResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Get all available tools from the proxy server
    let connection_pool = proxy.connection_pool.lock().await;
    let mut server_tools = Vec::new();
    let mut server_status = "Disconnected".to_string();
    let mut enabled_count = 0;
    let mut disabled_count = 0;

    // Find the server in the connection pool
    if let Some(instances) = connection_pool.connections.get(&server_name) {
        for (_, conn) in instances {
            // Update server status
            if conn.is_connected() {
                server_status = "Connected".to_string();

                // Add all tools from this instance
                for tool in &conn.tools {
                    let tool_name = tool.name.to_string();

                    // Get tool status (ID, prefixed name, enabled status)
                    let (tool_id, prefixed_name, enabled) =
                        get_tool_status(&db.pool, &server_name, &tool_name).await?;

                    // Create a tool response
                    let tool_response = create_tool_response(
                        &server_name,
                        &tool_name,
                        tool_id,
                        prefixed_name,
                        enabled,
                    );

                    // Track enabled/disabled counts
                    if enabled {
                        enabled_count += 1;
                    } else {
                        disabled_count += 1;
                    }

                    // Add the tool to the response (include both enabled and disabled tools)
                    server_tools.push(tool_response);
                }
            }
        }
    } else {
        return Err(ApiError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )));
    }

    // Log the number of tools found
    tracing::info!(
        "Found {} tools for server '{}' (enabled: {}, disabled: {}), status: {}",
        server_tools.len(),
        server_name,
        enabled_count,
        disabled_count,
        server_status
    );

    Ok(Json(ServerToolsResponse {
        server_name,
        status: server_status,
        tools: server_tools,
    }))
}
