// MCP specification-compliant tool handlers
// Provides handlers for MCP specification-compliant tool information

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    api::{
        handlers::ApiError,
        routes::AppState,
    },
    conf::operations,
};

use crate::api::handlers::tool::common::{get_context, get_tool_status};

/// List all MCP specification-compliant tools
pub async fn list_all(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<rmcp::model::Tool>>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Get all available tools from the proxy server
    let connection_pool = proxy.connection_pool.lock().await;
    let mut mcp_tools = Vec::new();

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
                let (_, prefixed_name, enabled) =
                    get_tool_status(&db.pool, server_name, &tool_name).await?;

                // Only include enabled tools
                if enabled {
                    // Convert to SDK Tool type
                    let sdk_tool = rmcp::model::Tool {
                        name: prefixed_name.unwrap_or_else(|| tool_name.clone()).into(),
                        description: Some(
                            format!("Tool provided by server '{}'", server_name).into(),
                        ),
                        input_schema: tool.input_schema.clone(), // Already an Arc<JsonObject>
                        annotations: None,
                    };

                    mcp_tools.push(sdk_tool);
                }
            }
        }
    }

    tracing::info!(
        "Returning {} tools in MCP specification format",
        mcp_tools.len()
    );
    
    Ok(Json(mcp_tools))
}

/// List MCP specification-compliant tools for a specific server
pub async fn list_server(
    State(state): State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<Vec<rmcp::model::Tool>>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

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

    // Get all available tools from the proxy server
    let connection_pool = proxy.connection_pool.lock().await;
    let mut mcp_tools = Vec::new();

    // Find the server in the connection pool
    if let Some(instances) = connection_pool.connections.get(&server_name) {
        for (_, conn) in instances {
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
                            format!("Tool provided by server '{}'", server_name).into(),
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
            "Server '{}' not found in connection pool",
            server_name
        )));
    }

    tracing::info!(
        "Returning {} tools for server '{}' in MCP specification format",
        mcp_tools.len(),
        server_name
    );
    
    Ok(Json(mcp_tools))
}

/// Get MCP specification-compliant information for a specific tool
pub async fn get_tool(
    State(state): State<Arc<AppState>>,
    Path((server_name, tool_name)): Path<(String, String)>,
) -> Result<Json<rmcp::model::Tool>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

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
    let (_, prefixed_name, enabled) =
        get_tool_status(&db.pool, &server_name, &tool_name).await?;

    // Check if the tool is enabled
    if !enabled {
        return Err(ApiError::NotFound(format!(
            "Tool '{}' is disabled or not found in server '{}'",
            tool_name, server_name
        )));
    }

    // Get the connection pool
    let connection_pool = proxy.connection_pool.lock().await;

    // Find the server in the connection pool
    let instances = connection_pool
        .connections
        .get(&server_name)
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Server '{}' not found in connection pool",
                server_name
            ))
        })?;

    // Look for the tool in all instances of this server
    for (_, conn) in instances {
        if !conn.is_connected() {
            continue;
        }

        // Look for the tool in this instance
        for tool in &conn.tools {
            if tool.name.to_string() == tool_name {
                // Found the tool, return its MCP specification-compliant information
                let sdk_tool = rmcp::model::Tool {
                    name: prefixed_name.unwrap_or_else(|| tool_name.clone()).into(),
                    description: Some(
                        format!("Tool provided by server '{}'", server_name).into(),
                    ),
                    input_schema: tool.input_schema.clone(), // Already an Arc<JsonObject>
                    annotations: None,
                };

                tracing::info!(
                    "Returning tool '{}' from server '{}' in MCP specification format",
                    tool_name,
                    server_name
                );

                return Ok(Json(sdk_tool));
            }
        }
    }

    // Tool not found
    Err(ApiError::NotFound(format!(
        "Tool '{}' not found in server '{}'",
        tool_name, server_name
    )))
}
