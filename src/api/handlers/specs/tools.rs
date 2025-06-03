// MCP specification-compliant tool handlers
// Provides handlers for MCP specification-compliant tool information

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    api::{handlers::ApiError, routes::AppState},
    config::{operations, server, suit},
    core::http::HttpProxyServer,
};

/// List all MCP specification-compliant tools
pub async fn list_all(
    State(state): State<Arc<AppState>>
) -> Result<Json<Vec<rmcp::model::Tool>>, ApiError> {
    // Get the HTTP proxy server and database
    let (proxy, db) = get_context(&state).await?;

    // Get all available tools from the proxy server
    let connection_pool = proxy.connection_pool.lock().await;
    let mut mcp_tools = Vec::new();

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

                // Get tool status (ID, unique_name, enabled status)
                let (_, unique_name, enabled) =
                    get_tool_status(&db.pool, server_name, &tool_name).await?;

                // Only include enabled tools
                if enabled {
                    // Use unique_name if available, otherwise use original tool name
                    let display_name = unique_name.unwrap_or_else(|| tool_name.clone());

                    // Convert to SDK Tool type
                    let sdk_tool = rmcp::model::Tool {
                        name: display_name.into(),
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
    let server = server::get_server(&db.pool, &server_name)
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

                // Get tool status (ID, unique_name, enabled status)
                let (_, unique_name, enabled) =
                    get_tool_status(&db.pool, &server_name, &tool_name).await?;

                // Only include enabled tools
                if enabled {
                    // Use unique_name if available, otherwise use original tool name
                    let display_name = unique_name.unwrap_or_else(|| tool_name.clone());

                    // Convert to SDK Tool type
                    let sdk_tool = rmcp::model::Tool {
                        name: display_name.into(),
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
    let server = server::get_server(&db.pool, &server_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    if server.is_none() {
        return Err(ApiError::NotFound(format!(
            "Server '{server_name}' not found"
        )));
    }

    // Get tool status (ID, unique_name, enabled status)
    let (_, unique_name, enabled) = get_tool_status(&db.pool, &server_name, &tool_name).await?;

    // Check if the tool is enabled
    if !enabled {
        return Err(ApiError::NotFound(format!(
            "Tool '{tool_name}' is disabled or not found in server '{server_name}'"
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
                "Server '{server_name}' not found in connection pool"
            ))
        })?;

    // Look for the tool in all instances of this server
    for conn in instances.values() {
        if !conn.is_connected() {
            continue;
        }

        // Look for the tool in this instance
        for tool in &conn.tools {
            if tool.name == tool_name {
                // Found the tool, return its MCP specification-compliant information
                // Use unique_name if available, otherwise use original tool name
                let display_name = unique_name.unwrap_or_else(|| tool_name.clone());

                let sdk_tool = rmcp::model::Tool {
                    name: display_name.into(),
                    description: Some(format!("Tool provided by server '{server_name}'").into()),
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
        "Tool '{tool_name}' not found in server '{server_name}'"
    )))
}

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
pub async fn get_context(
    state: &Arc<AppState>
) -> Result<(&HttpProxyServer, &crate::config::database::Database), ApiError> {
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

/// Helper function to get tool status (ID, unique name, enabled status)
pub async fn get_tool_status(
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

    // Get the tool ID
    let tool_id = match operations::tool::get_tool_id(pool, server_name, tool_name).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            // Tool not found in database, create a new record
            // Get the default config suit
            let default_suit = suit::get_default_config_suit(pool).await.map_err(|e| {
                ApiError::InternalError(format!("Failed to get default config suit: {e}"))
            })?;

            // If there's no default suit, try the legacy "default" named suit
            let suit_id = if let Some(suit) = default_suit {
                suit.id.unwrap()
            } else {
                let legacy_default = suit::get_config_suit_by_name(pool, "default")
                    .await
                    .map_err(|e| {
                        ApiError::InternalError(format!(
                            "Failed to get legacy default config suit: {e}"
                        ))
                    })?;

                // If there's no legacy default suit either, create a new default suit
                if let Some(suit) = legacy_default {
                    suit.id.unwrap()
                } else {
                    // Create default config suit if it doesn't exist
                    let mut new_suit = crate::config::models::ConfigSuit::new_with_description(
                        "default".to_string(),
                        Some("Default configuration suit".to_string()),
                        crate::common::config::ConfigSuitType::Shared,
                    );

                    // Set active and default flags
                    new_suit.is_active = true;
                    new_suit.is_default = true;
                    new_suit.multi_select = true;
                    suit::upsert_config_suit(pool, &new_suit)
                        .await
                        .map_err(|e| {
                            ApiError::InternalError(format!(
                                "Failed to create default config suit: {e}"
                            ))
                        })?
                }
            };

            // Get the server ID
            let server = server::get_server(pool, server_name)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

            if let Some(server) = server {
                if let Some(server_id) = &server.id {
                    // Add the tool to the config suit
                    suit::add_tool_to_config_suit(pool, &suit_id, server_id, tool_name, true)
                        .await
                        .map_err(|e| {
                            ApiError::InternalError(format!(
                                "Failed to add tool to config suit: {e}"
                            ))
                        })?
                } else {
                    return Err(ApiError::InternalError(format!(
                        "Server '{server_name}' has no ID"
                    )));
                }
            } else {
                return Err(ApiError::NotFound(format!(
                    "Server '{server_name}' not found"
                )));
            }
        }
        Err(e) => {
            return Err(ApiError::InternalError(format!(
                "Failed to get tool ID: {e}"
            )));
        }
    };

    // Get the unique name for the tool
    let unique_name = sqlx::query_scalar::<_, String>(
        r#"
        SELECT unique_name
        FROM config_suit_tool
        WHERE server_name = ? AND tool_name = ?
        LIMIT 1
        "#,
    )
    .bind(server_name)
    .bind(tool_name)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to get unique name: {e}")))?;

    Ok((tool_id, unique_name, enabled))
}
