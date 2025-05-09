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
    conf::{models::ToolConfig, operations},
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
        .db
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Get all tool configurations from the database
    tracing::info!("Fetching all tool configurations from database");
    let configs = match operations::get_all_tool_configs(&db.pool).await {
        Ok(configs) => {
            tracing::info!("Successfully fetched {} tool configurations", configs.len());
            configs
        }
        Err(e) => {
            tracing::error!("Failed to get tool configurations: {}", e);
            return Err(ApiError::InternalError(format!(
                "Failed to get tool configurations: {}",
                e
            )));
        }
    };

    // Create a map of existing configurations for quick lookup
    let mut config_map: std::collections::HashMap<String, crate::conf::models::ToolConfig> =
        std::collections::HashMap::new();
    for config in configs {
        let key = format!("{}:{}", config.server_name, config.tool_name);
        config_map.insert(key, config);
    }

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
                // Create a tool response for each tool
                let key = format!("{}:{}", server_name, tool.name);
                let tool_response = if let Some(config) = config_map.get(&key) {
                    // Tool exists in database, use its configuration
                    let display_name = config
                        .alias_name
                        .clone()
                        .unwrap_or_else(|| config.tool_name.clone());
                    ToolResponse {
                        id: config.id.unwrap_or(0),
                        server_name: config.server_name.clone(),
                        tool_name: config.tool_name.clone(),
                        alias_name: config.alias_name.clone(),
                        display_name,
                        enabled: config.enabled,
                        created_at: config.created_at.map(|dt| dt.to_rfc3339()),
                        updated_at: config.updated_at.map(|dt| dt.to_rfc3339()),
                    }
                } else {
                    // Tool doesn't exist in database, create a default entry (enabled by default)
                    let tool_name = tool.name.to_string();
                    ToolResponse {
                        id: 0, // Will be assigned when actually stored in DB
                        server_name: server_name.clone(),
                        tool_name: tool_name.clone(),
                        alias_name: None,
                        display_name: tool_name,
                        enabled: true, // Default to enabled
                        created_at: None,
                        updated_at: None,
                    }
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
        .db
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Get the tool configuration from the database
    let config = operations::get_tool_config(&db.pool, &server_name, &tool_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool configuration: {}", e)))?;

    // Check if the tool configuration exists
    let config = config.ok_or_else(|| {
        ApiError::NotFound(format!(
            "Tool '{}' from server '{}' not found",
            tool_name, server_name
        ))
    })?;

    // Create tool configuration response
    let display_name = config
        .alias_name
        .clone()
        .unwrap_or_else(|| config.tool_name.clone());
    let response = ToolConfigResponse {
        id: config.id.unwrap_or(0),
        server_name: config.server_name,
        tool_name: config.tool_name,
        alias_name: config.alias_name.clone(),
        display_name,
        enabled: config.enabled,
        created_at: config.created_at.map(|dt| dt.to_rfc3339()),
        updated_at: config.updated_at.map(|dt| dt.to_rfc3339()),
        allowed_operations: vec![if config.enabled {
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
        .db
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Get existing configuration if any
    let existing_config = operations::get_tool_config(&db.pool, &server_name, &tool_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool configuration: {}", e)))?;

    // Create or update the tool configuration, preserving alias if it exists
    let alias_name = existing_config.as_ref().and_then(|c| c.alias_name.clone());
    let config =
        ToolConfig::new_with_alias(server_name.clone(), tool_name.clone(), alias_name, true);
    let id = operations::upsert_tool_config(&db.pool, &config)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to update tool configuration: {}", e))
        })?;

    // Notify clients about tool list change
    // TODO: Implement notification with HTTP proxy server
    // This will be implemented in a future update
    tracing::info!(
        "Tool '{}' from server '{}' has been enabled",
        tool_name,
        server_name
    );

    // Create tool status response
    let response = ToolStatusResponse {
        id,
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
        .db
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Get existing configuration if any
    let existing_config = operations::get_tool_config(&db.pool, &server_name, &tool_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool configuration: {}", e)))?;

    // Create or update the tool configuration, preserving alias if it exists
    let alias_name = existing_config.as_ref().and_then(|c| c.alias_name.clone());
    let config =
        ToolConfig::new_with_alias(server_name.clone(), tool_name.clone(), alias_name, false);
    let id = operations::upsert_tool_config(&db.pool, &config)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to update tool configuration: {}", e))
        })?;

    // Notify clients about tool list change
    // TODO: Implement notification with HTTP proxy server
    // This will be implemented in a future update
    tracing::info!(
        "Tool '{}' from server '{}' has been disabled",
        tool_name,
        server_name
    );

    // Create tool status response
    let response = ToolStatusResponse {
        id,
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
        .db
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Get existing configuration if any
    let existing_config = operations::get_tool_config(&db.pool, &server_name, &tool_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool configuration: {}", e)))?;

    // Determine the alias name to use
    let alias_name = if request.alias_name.is_some() {
        // Use the provided alias name if it exists
        request.alias_name.clone()
    } else {
        // Otherwise, preserve the existing alias name if any
        existing_config.as_ref().and_then(|c| c.alias_name.clone())
    };

    // Create or update the tool configuration
    let config = ToolConfig::new_with_alias(
        server_name.clone(),
        tool_name.clone(),
        alias_name,
        request.enabled,
    );
    let id = operations::upsert_tool_config(&db.pool, &config)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to update tool configuration: {}", e))
        })?;

    // Notify clients about tool list change
    // TODO: Implement notification with HTTP proxy server
    // This will be implemented in a future update
    tracing::info!(
        "Tool '{}' from server '{}' has been updated, enabled: {}",
        tool_name,
        server_name,
        request.enabled
    );

    // Get the updated tool configuration
    let updated_config = operations::get_tool_config(&db.pool, &server_name, &tool_name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool configuration: {}", e)))?
        .ok_or_else(|| {
            ApiError::InternalError(format!(
                "Tool configuration not found after update: {}/{}",
                server_name, tool_name
            ))
        })?;

    // Create tool configuration response
    let display_name = updated_config
        .alias_name
        .clone()
        .unwrap_or_else(|| updated_config.tool_name.clone());
    let response = ToolConfigResponse {
        id,
        server_name: updated_config.server_name,
        tool_name: updated_config.tool_name,
        alias_name: updated_config.alias_name.clone(),
        display_name,
        enabled: updated_config.enabled,
        created_at: updated_config.created_at.map(|dt| dt.to_rfc3339()),
        updated_at: updated_config.updated_at.map(|dt| dt.to_rfc3339()),
        allowed_operations: vec![if updated_config.enabled {
            "disable".to_string()
        } else {
            "enable".to_string()
        }],
    };

    Ok(Json(response))
}
