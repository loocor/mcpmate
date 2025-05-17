// Tool list handlers

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};

use super::common::{create_tool_response, get_context, get_tool_status};
use crate::api::{
    handlers::ApiError,
    models::tool::{
        McpTool, McpToolListResponse, ServerToolsResponse, ToolAnnotations, ToolListResponse,
    },
    routes::AppState,
};

/// List all MCP tools (MCPMate format)
pub async fn all(State(state): State<Arc<AppState>>) -> Result<Json<ToolListResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Get all available tools from the proxy server
    let connection_pool = proxy.connection_pool.lock().await;
    let mut all_tools = Vec::new();

    // Iterate through all servers and their tools
    for (server_name, instances) in connection_pool.connections.iter() {
        for conn in instances.values() {
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

/// List all MCP tools (MCP format)
pub async fn mcp_list(
    State(state): State<Arc<AppState>>
) -> Result<Json<McpToolListResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Get all available tools from the proxy server
    let connection_pool = proxy.connection_pool.lock().await;
    let mut all_tools = Vec::new();

    // Iterate through all servers and their tools
    for (server_name, instances) in connection_pool.connections.iter() {
        for conn in instances.values() {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance
            for tool in &conn.tools {
                let tool_name = tool.name.to_string();

                // Get tool status (ID, prefixed name, enabled status)
                let (_, prefixed_name, enabled) =
                    get_tool_status(&db.pool, server_name, &tool_name).await?;

                // Only include enabled tools
                if enabled {
                    // Create an MCP tool definition
                    let mcp_tool = McpTool {
                        name: prefixed_name.unwrap_or_else(|| tool_name.clone()),
                        description: Some(format!("Tool provided by server '{server_name}'")),
                        input_schema: serde_json::Value::Object(tool.input_schema.as_ref().clone()),
                        annotations: Some(ToolAnnotations {
                            title: Some(tool_name.clone()),
                            read_only_hint: None,
                            destructive_hint: None,
                            idempotent_hint: None,
                            open_world_hint: None,
                        }),
                    };

                    all_tools.push(mcp_tool);
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

    tracing::info!("Returning {} tools to client (MCP format)", all_tools.len());
    Ok(Json(McpToolListResponse {
        tools: all_tools,
        next_cursor: None, // No pagination for now
    }))
}

/// List tools for a specific server
pub async fn list(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<ServerToolsResponse>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Check if the server exists
    let server = crate::conf::operations::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{server_name}' not found"
        )));
    }

    // Get the connection pool
    let connection_pool = proxy.connection_pool.lock().await;
    let mut tools = Vec::new();
    let mut status = "Unknown".to_string();

    // Find the server in the connection pool
    if let Some(instances) = connection_pool.connections.get(&server_name) {
        // Set the status based on the first instance (assuming all instances have the same status)
        if let Some((_, conn)) = instances.iter().next() {
            status = format!("{:?}", conn.status);
        }

        // Iterate through all instances
        for conn in instances.values() {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance
            for tool in &conn.tools {
                let tool_name = tool.name.to_string();

                // Get tool status (ID, prefixed name, enabled status)
                let (tool_id, prefixed_name, enabled) =
                    get_tool_status(&db.pool, &server_name, &tool_name).await?;

                // Create a tool response
                let tool_response =
                    create_tool_response(&server_name, &tool_name, tool_id, prefixed_name, enabled);

                tools.push(tool_response);
            }
        }
    } else {
        status = "NotConnected".to_string();
    }

    // Create the server tools response
    let response = ServerToolsResponse {
        server_name,
        status,
        tools,
    };

    Ok(Json(response))
}

/// Get detailed tool information for a specific server (MCP format)
pub async fn details(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<Vec<rmcp::model::Tool>>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Check if the server exists
    let server = crate::conf::operations::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{server_name}' not found"
        )));
    }

    // Get all available tools from the proxy server
    let connection_pool = proxy.connection_pool.lock().await;
    let mut mcp_tools = Vec::new();

    // Find the server in the connection pool
    if let Some(instances) = connection_pool.connections.get(&server_name) {
        for conn in instances.values() {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance
            for tool in &conn.tools {
                let tool_name = tool.name.to_string();

                // Get tool status (ID, prefixed name, enabled status)
                let (_, prefixed_name, enabled) =
                    get_tool_status(&db.pool, &server_name, &tool_name).await?;

                // Only include enabled tools
                if enabled {
                    // Convert to SDK Tool type
                    let sdk_tool = rmcp::model::Tool {
                        name: prefixed_name.unwrap_or_else(|| tool_name.clone()).into(),
                        description: Some(
                            format!("Tool provided by server '{server_name}'").into(),
                        ),
                        input_schema: tool.input_schema.clone(), // Already an Arc<JsonObject>
                        annotations: None,
                    };

                    mcp_tools.push(sdk_tool);
                }
            }
        }
    } else {
        return Err(ApiError::NotFound(format!(
            "Server '{server_name}' not found in connection pool"
        )));
    }

    tracing::info!(
        "Returning {} tools for server '{}' (MCP format)",
        mcp_tools.len(),
        server_name
    );

    Ok(Json(mcp_tools))
}
