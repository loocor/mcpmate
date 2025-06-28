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

/// Get a tool by ID or return an error (new architecture)
pub async fn get_tool_or_error(
    db: &Database,
    tool_id: &str,
) -> Result<ConfigSuitTool, ApiError> {
    let tool = sqlx::query_as::<_, ConfigSuitTool>("SELECT * FROM config_suit_tool WHERE id = ?")
        .bind(tool_id)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool: {e}")))?;

    match tool {
        Some(t) => Ok(t),
        None => Err(ApiError::NotFound(format!(
            "Tool with ID '{tool_id}' not found"
        ))),
    }
}

/// Get a tool with details by ID or return an error (new architecture)
pub async fn get_tool_with_details_or_error(
    db: &Database,
    tool_id: &str,
) -> Result<crate::config::models::ConfigSuitToolWithDetails, ApiError> {
    let query = crate::config::suit::tool::build_tool_details_query(Some("cst.id = ?"));
    let tool = sqlx::query_as::<_, crate::config::models::ConfigSuitToolWithDetails>(&query)
        .bind(tool_id)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool with details: {e}")))?;

    match tool {
        Some(t) => Ok(t),
        None => Err(ApiError::NotFound(format!(
            "Tool with ID '{tool_id}' not found"
        ))),
    }
}

/// Check if a tool belongs to a specific configuration suit (new architecture)
pub fn check_tool_belongs_to_suit(
    tool: &ConfigSuitTool,
    suit_id: &str,
) -> Result<(), ApiError> {
    if tool.config_suit_id != suit_id {
        return Err(ApiError::BadRequest(format!(
            "Tool with ID '{}' does not belong to configuration suit with ID '{}'",
            tool.id, suit_id
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

/// Get the config suit ID to use, either from parameter or first active suit
pub async fn get_config_suit_id(
    config_suit_id: Option<String>,
    db_pool: &sqlx::SqlitePool,
) -> Result<String, ApiError> {
    match config_suit_id {
        Some(id) => Ok(id),
        None => {
            // Get the first active config suit
            let active_suits = crate::config::suit::get_active_config_suits(db_pool)
                .await
                .map_err(|e| {
                    ApiError::InternalError(format!("Failed to get active config suits: {}", e))
                })?;

            match active_suits.first() {
                Some(suit) => Ok(suit.id.clone().unwrap_or_else(|| "default".to_string())),
                None => {
                    tracing::warn!("No active config suits found");
                    Err(ApiError::NotFound(
                        "No active config suits found".to_string(),
                    ))
                }
            }
        }
    }
}

/// Sync client configurations using the specified or first active config suit
pub async fn sync_client_configurations(
    state: &std::sync::Arc<crate::api::routes::AppState>,
    config_suit_id: Option<String>,
) -> Result<(), ApiError> {
    // Get database reference
    let db = get_database(state).await?;

    // Get the config suit ID to use
    let suit_id = get_config_suit_id(config_suit_id, &db.pool).await?;

    // Create client manager
    let mut client_manager =
        crate::config::client::manager::ClientManager::new(std::sync::Arc::new(db.pool.clone()));

    // Apply configuration to all enabled clients
    match client_manager.apply_config_batch(Some(suit_id)).await {
        Ok(result) => {
            tracing::info!(
                "Synced configurations to {} clients, {} failed",
                result.success_count,
                result.failed_clients.len()
            );

            if !result.failed_clients.is_empty() {
                for (client, error) in result.failed_clients {
                    tracing::warn!("Failed to sync config for client {}: {}", client, error);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to sync client configurations: {}", e);
            return Err(ApiError::InternalError(format!(
                "Failed to sync client configurations: {}",
                e
            )));
        }
    }

    Ok(())
}

/// Get database reference from app state
pub async fn get_database(
    state: &std::sync::Arc<crate::api::routes::AppState>
) -> Result<std::sync::Arc<Database>, ApiError> {
    state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))
        .cloned()
}

/// Check if the identifier is a database ID (vs tool name)
pub fn is_database_id(identifier: &str) -> bool {
    // Database IDs typically start with specific prefixes like "cstool_" or are long UUIDs
    identifier.starts_with("cstool_") || identifier.len() > 20
}

/// Check if the identifier is a tool name (vs database ID)
pub fn is_tool_name(identifier: &str) -> bool {
    // Tool names are typically simple strings without special prefixes
    !is_database_id(identifier)
}

/// Get or create a config suit tool by tool name
/// This function handles the case where a tool name is passed instead of a database ID
pub async fn get_or_create_tool_by_name(
    db: &Database,
    suit_id: &str,
    tool_name: &str,
) -> Result<crate::config::models::ConfigSuitTool, ApiError> {
    // First, try to find existing config suit tool by tool name
    let existing_tool = sqlx::query_as::<_, crate::config::models::ConfigSuitTool>(
        r#"
        SELECT cst.* FROM config_suit_tool cst
        JOIN server_tools st ON cst.server_tool_id = st.id
        WHERE cst.config_suit_id = ? AND st.tool_name = ?
        "#,
    )
    .bind(suit_id)
    .bind(tool_name)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to query existing tool: {e}")))?;

    if let Some(tool) = existing_tool {
        return Ok(tool);
    }

    // If not found, we need to create a new config suit tool
    // But first we need to find which server this tool belongs to
    // We'll look for the tool in any server that's enabled in this suit
    let server_info = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT DISTINCT st.server_id, st.server_name
        FROM server_tools st
        JOIN config_suit_server css ON st.server_id = css.server_id
        WHERE css.config_suit_id = ? AND css.enabled = true AND st.tool_name = ?
        LIMIT 1
        "#,
    )
    .bind(suit_id)
    .bind(tool_name)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to find server for tool: {e}")))?;

    let (server_id, _server_name) = server_info.ok_or_else(|| {
        ApiError::NotFound(format!(
            "Tool '{}' not found in any enabled server for configuration suit '{}'",
            tool_name, suit_id
        ))
    })?;

    // Create the config suit tool using the existing function
    let tool_id = crate::config::suit::add_tool_to_config_suit(
        &db.pool, suit_id, &server_id, tool_name, false, // Start with disabled state
    )
    .await
    .map_err(|e| {
        ApiError::InternalError(format!("Failed to create config suit tool: {e}"))
    })?;

    // Now fetch the created tool to return it
    let created_tool = sqlx::query_as::<_, crate::config::models::ConfigSuitTool>(
        "SELECT * FROM config_suit_tool WHERE id = ?",
    )
    .bind(&tool_id)
    .fetch_one(&db.pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch created tool: {e}")))?;

    Ok(created_tool)
}

/// Resolve tool identifier to ConfigSuitTool for batch operations
/// This function handles both database IDs and tool names
pub async fn resolve_tool_for_batch_operation(
    db: &Database,
    suit_id: &str,
    tool_identifier: &str,
) -> Result<crate::config::models::ConfigSuitTool, String> {
    // First try to get by ID
    match get_tool_or_error(db, tool_identifier).await {
        Ok(tool) => {
            // Verify the tool belongs to the specified suit
            if tool.config_suit_id != suit_id {
                return Err("Tool does not belong to the specified configuration suit".to_string());
            }
            Ok(tool)
        }
        Err(_) => {
            // If ID lookup failed, try to find or create by tool name
            match get_or_create_tool_by_name(db, suit_id, tool_identifier).await {
                Ok(tool) => Ok(tool),
                Err(e) => Err(format!("Failed to resolve tool '{}': {}", tool_identifier, e)),
            }
        }
    }
}
