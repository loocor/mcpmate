// MCPMate Proxy API handlers for Config Suit helpers
// Contains helper functions for Config Suit handlers

use crate::{
    api::handlers::ApiError,
    config::{
        database::Database,
        models::{ConfigSuit, ConfigSuitTool, Server},
    },
};

use super::common::get_database;

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
        None => Err(ApiError::NotFound(format!("Server with ID '{server_id}' not found"))),
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
        None => Err(ApiError::NotFound(format!("Tool with ID '{tool_id}' not found"))),
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
        None => Err(ApiError::NotFound(format!("Tool with ID '{tool_id}' not found"))),
    }
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
                .map_err(|e| ApiError::InternalError(format!("Failed to get active config suits: {}", e)))?;

            match active_suits.first() {
                Some(suit) => Ok(suit.id.clone().unwrap_or_else(|| "default".to_string())),
                None => {
                    tracing::warn!("No active config suits found");
                    Err(ApiError::NotFound("No active config suits found".to_string()))
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
    let mut client_manager = crate::config::client::manager::ClientManager::new(std::sync::Arc::new(db.pool.clone()));

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
