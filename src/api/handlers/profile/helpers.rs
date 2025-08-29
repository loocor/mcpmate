// MCPMate Proxy API handlers for Profile helpers
// Contains helper functions for Profile handlers

use crate::{
    api::handlers::ApiError,
    config::{
        database::Database,
        models::{Profile, ProfileTool},
    },
};

use super::common::get_database;

/// Get a profile by ID or return an error
pub async fn get_profile_or_error(
    db: &Database,
    profile_id: &str,
) -> Result<Profile, ApiError> {
    let profile = crate::config::profile::get_profile(&db.pool, profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile: {e}")))?;

    match profile {
        Some(s) => Ok(s),
        None => Err(ApiError::NotFound(format!("Profile with ID '{profile_id}' not found"))),
    }
}

/// Get a tool by ID or return an error (new architecture)
pub async fn get_tool_or_error(
    db: &Database,
    tool_id: &str,
) -> Result<ProfileTool, ApiError> {
    let tool = sqlx::query_as::<_, ProfileTool>("SELECT * FROM profile_tool WHERE id = ?")
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
) -> Result<crate::config::models::ProfileToolWithDetails, ApiError> {
    let query = crate::config::profile::tool::build_tool_details_query(Some("cst.id = ?"));
    let tool = sqlx::query_as::<_, crate::config::models::ProfileToolWithDetails>(&query)
        .bind(tool_id)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool with details: {e}")))?;

    match tool {
        Some(t) => Ok(t),
        None => Err(ApiError::NotFound(format!("Tool with ID '{tool_id}' not found"))),
    }
}

/// Get the profile ID to use, either from parameter or first active profile
pub async fn get_profile_id(
    profile_id: Option<String>,
    db_pool: &sqlx::SqlitePool,
) -> Result<String, ApiError> {
    match profile_id {
        Some(id) => Ok(id),
        None => {
            // Get the first active profile
            let active_profile = crate::config::profile::get_active_profile(db_pool)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to get active  profile: {}", e)))?;

            match active_profile.first() {
                Some(profile) => Ok(profile.id.clone().unwrap_or_else(|| "default".to_string())),
                None => {
                    tracing::warn!("No active profile found");
                    Err(ApiError::NotFound("No active profile found".to_string()))
                }
            }
        }
    }
}

/// Sync client configurations using the specified or first active profile
pub async fn sync_client_configurations(
    state: &std::sync::Arc<crate::api::routes::AppState>,
    profile_id: Option<String>,
) -> Result<(), ApiError> {
    // Get database reference
    let db = get_database(state).await?;

    // Get the profile ID to use
    let profile_id = get_profile_id(profile_id, &db.pool).await?;

    // Create client manager
    let mut client_manager = crate::config::client::manager::ClientManager::new(std::sync::Arc::new(db.pool.clone()));

    // Apply configuration to all enabled clients
    match client_manager.apply_config_batch(Some(profile_id)).await {
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
