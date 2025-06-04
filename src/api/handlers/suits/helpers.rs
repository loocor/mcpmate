// MCPMate Proxy API handlers for Config Suit helpers
// Contains helper functions for Config Suit handlers

use crate::{
    api::handlers::ApiError,
    config::{
        database::Database,
        models::{ConfigSuit, ConfigSuitResource, ConfigSuitTool, Server},
    },
};

/// Get a configuration suit by ID or return an error
pub async fn get_suit_or_error(
    db: &Database,
    suit_id: &str,
) -> Result<ConfigSuit, ApiError> {
    let suit = crate::config::suit::get_config_suit(&db.pool, suit_id)
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
    let server = crate::config::server::get_server_by_id(&db.pool, server_id)
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
    let tool = crate::config::operations::tool::get_config_suit_tool_by_id(&db.pool, tool_id)
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

/// Get a resource by ID or return an error
pub async fn get_resource_or_error(
    db: &Database,
    resource_id: &str,
) -> Result<ConfigSuitResource, ApiError> {
    let resource = get_resource_by_id(db, resource_id).await?;
    Ok(resource)
}

/// Get a resource by ID (internal helper)
pub async fn get_resource_by_id(
    db: &Database,
    resource_id: &str,
) -> Result<ConfigSuitResource, ApiError> {
    // Query the resource by ID
    let resource = sqlx::query_as::<_, ConfigSuitResource>(
        r#"
        SELECT id, config_suit_id, server_id, server_name, resource_uri, enabled, created_at, updated_at
        FROM config_suit_resource
        WHERE id = ?
        "#,
    )
    .bind(resource_id)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to get resource: {e}")))?;

    match resource {
        Some(r) => Ok(r),
        None => Err(ApiError::NotFound(format!(
            "Resource with ID '{resource_id}' not found"
        ))),
    }
}

/// Check if a resource belongs to a specific configuration suit
pub fn check_resource_belongs_to_suit(
    resource: &ConfigSuitResource,
    suit_id: &str,
) -> Result<(), ApiError> {
    if resource.config_suit_id != suit_id {
        return Err(ApiError::BadRequest(format!(
            "Resource with ID '{}' does not belong to configuration suit with ID '{}'",
            resource.id.as_ref().unwrap_or(&"unknown".to_string()),
            suit_id
        )));
    }
    Ok(())
}
