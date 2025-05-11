// Common helper functions for tool handlers

use std::sync::Arc;

use serde_json::Value;

use crate::{
    api::{handlers::ApiError, models::tool::ToolResponse, routes::AppState},
    conf::operations,
    http::HttpProxyServer,
};

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
            let default_suit = operations::suit::get_default_config_suit(pool)
                .await
                .map_err(|e| {
                    ApiError::InternalError(format!("Failed to get default config suit: {}", e))
                })?;

            // If there's no default suit, try the legacy "default" named suit
            let suit_id = if let Some(suit) = default_suit {
                suit.id.unwrap()
            } else {
                let legacy_default = operations::get_config_suit_by_name(pool, "default")
                    .await
                    .map_err(|e| {
                        ApiError::InternalError(format!(
                            "Failed to get legacy default config suit: {}",
                            e
                        ))
                    })?;

                // If there's no legacy default suit either, create a new default suit
                if let Some(suit) = legacy_default {
                    suit.id.unwrap()
                } else {
                    // Create default config suit if it doesn't exist
                    let mut new_suit = crate::conf::models::ConfigSuit::new_with_description(
                        "default".to_string(),
                        Some("Default configuration suit".to_string()),
                        crate::conf::models::ConfigSuitType::Shared,
                    );

                    // Set active and default flags
                    new_suit.is_active = true;
                    new_suit.is_default = true;
                    new_suit.multi_select = true;
                    operations::upsert_config_suit(pool, &new_suit)
                        .await
                        .map_err(|e| {
                            ApiError::InternalError(format!(
                                "Failed to create default config suit: {}",
                                e
                            ))
                        })?
                }
            };

            // Get the server ID
            let server = operations::get_server(pool, server_name)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to get server: {}", e)))?;

            if let Some(server) = server {
                if let Some(server_id) = &server.id {
                    // Add the tool to the config suit
                    let tool_id = operations::suit::add_tool_to_config_suit(
                        pool, &suit_id, server_id, tool_name, true,
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalError(format!("Failed to add tool to config suit: {}", e))
                    })?;

                    tool_id
                } else {
                    return Err(ApiError::InternalError(format!(
                        "Server '{}' has no ID",
                        server_name
                    )));
                }
            } else {
                return Err(ApiError::NotFound(format!(
                    "Server '{}' not found",
                    server_name
                )));
            }
        }
        Err(e) => {
            return Err(ApiError::InternalError(format!(
                "Failed to get tool ID: {}",
                e
            )));
        }
    };

    // Get the prefixed name
    let prefixed_name =
        match operations::tool::get_tool_prefixed_name(pool, server_name, tool_name).await {
            Ok(prefixed_name) => prefixed_name,
            Err(e) => {
                tracing::warn!("Failed to get tool prefixed name: {}, using None", e);
                None
            }
        };

    Ok((tool_id, prefixed_name, enabled))
}

/// Helper function to create a tool response
pub fn create_tool_response(
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

/// Helper function to get tool schema
pub async fn get_tool_schema(
    proxy: &HttpProxyServer,
    server_name: &str,
    tool_name: &str,
) -> Result<Value, ApiError> {
    // Get the connection pool
    let connection_pool = proxy.connection_pool.lock().await;

    // Find the server in the connection pool
    let instances = connection_pool
        .connections
        .get(server_name)
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
                // Found the tool, return its schema as a Value
                return Ok(Value::Object(tool.input_schema.as_ref().clone()));
            }
        }
    }

    // Tool not found
    Err(ApiError::NotFound(format!(
        "Tool '{}' not found in server '{}'",
        tool_name, server_name
    )))
}
