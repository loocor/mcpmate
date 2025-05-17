// MCPMate Proxy API handlers for Config Suit helpers
// Contains helper functions for Config Suit handlers

use crate::{
    api::handlers::ApiError,
    conf::{
        database::Database,
        models::{ConfigSuit, ConfigSuitTool, Server},
    },
};

/// Get a configuration suit by ID or return an error
pub async fn get_suit_or_error(
    db: &Database,
    suit_id: &str,
) -> Result<ConfigSuit, ApiError> {
    let suit = crate::conf::operations::suit::get_config_suit(&db.pool, suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    match suit {
        Some(s) => Ok(s),
        None => Err(ApiError::NotFound(format!(
            "Configuration suit with ID '{suit_id}' not found"
        ))),
    }
}

/// Get a server by ID or return an error
pub async fn get_server_or_error(
    db: &Database,
    server_id: &str,
) -> Result<Server, ApiError> {
    let server = crate::conf::operations::get_server_by_id(&db.pool, server_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    match server {
        Some(s) => Ok(s),
        None => Err(ApiError::NotFound(format!(
            "Server with ID '{server_id}' not found"
        ))),
    }
}

/// Get a tool by ID or return an error
pub async fn get_tool_or_error(
    db: &Database,
    tool_id: &str,
) -> Result<ConfigSuitTool, ApiError> {
    let tool = crate::conf::operations::tool::get_config_suit_tool_by_id(&db.pool, tool_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool: {e}")))?;

    match tool {
        Some(t) => Ok(t),
        None => Err(ApiError::NotFound(format!(
            "Tool with ID '{tool_id}' not found"
        ))),
    }
}

/// Check if a tool belongs to a specific configuration suit
pub fn check_tool_belongs_to_suit(
    tool: &ConfigSuitTool,
    suit_id: &str,
) -> Result<(), ApiError> {
    if tool.config_suit_id != suit_id {
        return Err(ApiError::BadRequest(format!(
            "Tool with ID '{}' does not belong to configuration suit with ID '{}'",
            tool.id.as_ref().unwrap_or(&"unknown".to_string()),
            suit_id
        )));
    }
    Ok(())
}
