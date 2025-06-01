// MCPMate Proxy API handlers for Config Suit server management
// Contains handler functions for managing servers in Config Suits

use std::collections::HashMap;

use super::{common::*, get_server_or_error, get_suit_or_error};

/// List servers in a configuration suit
pub async fn list_servers(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigSuitServersResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = get_suit_or_error(&db, &id).await?;

    // Get all servers in the suit
    let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &id)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to get server configurations: {e}"))
        })?;

    // Get all servers
    let all_servers = crate::config::server::get_all_servers(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers: {e}")))?;

    // Create a map of server ID to server
    let server_map: HashMap<String, crate::config::models::Server> = all_servers
        .into_iter()
        .filter_map(|s| {
            if let Some(id) = &s.id {
                Some((id.clone(), s))
            } else {
                None
            }
        })
        .collect();

    // Convert to response format
    let mut server_responses = Vec::new();
    for config in server_configs {
        if let Some(server) = server_map.get(&config.server_id) {
            let mut allowed_operations = Vec::new();
            if config.enabled {
                allowed_operations.push("disable".to_string());
            } else {
                allowed_operations.push("enable".to_string());
            }

            server_responses.push(ConfigSuitServerResponse {
                id: config.server_id.clone(),
                name: server.name.clone(),
                enabled: config.enabled,
                allowed_operations,
            });
        }
    }

    // Return response
    Ok(Json(ConfigSuitServersResponse {
        suit_id: id,
        suit_name: suit.name,
        servers: server_responses,
    }))
}

/// Enable a server in a configuration suit
pub async fn enable_server(
    State(state): State<Arc<AppState>>,
    Path((suit_id, server_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the server to check if it exists and get its name
    let server = get_server_or_error(&db, &server_id).await?;

    // Check if the server is already enabled in the suit
    let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &suit_id)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to get server configurations: {e}"))
        })?;

    for config in server_configs {
        if config.server_id == server_id && config.enabled {
            return Ok(Json(SuitOperationResponse {
                id: server_id,
                name: server.name,
                result: "Server is already enabled in this configuration suit".to_string(),
                status: "Enabled".to_string(),
                allowed_operations: vec!["disable".to_string()],
            }));
        }
    }

    // Enable the server in the suit
    crate::config::suit::add_server_to_config_suit(&db.pool, &suit_id, &server_id, true)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!(
                "Failed to enable server in configuration suit: {e}"
            ))
        })?;

    // Sync server connections if merge service is available
    if let Some(merge_service) = &state.suit_merge_service {
        if let Err(e) = merge_service.sync_server_connections(&state).await {
            tracing::error!("Failed to sync server connections: {}", e);
        }
    }

    // Return success response
    Ok(Json(SuitOperationResponse {
        id: server_id,
        name: server.name,
        result: "Successfully enabled server in configuration suit".to_string(),
        status: "Enabled".to_string(),
        allowed_operations: vec!["disable".to_string()],
    }))
}

/// Disable a server in a configuration suit
pub async fn disable_server(
    State(state): State<Arc<AppState>>,
    Path((suit_id, server_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the server to check if it exists and get its name
    let server = get_server_or_error(&db, &server_id).await?;

    // Check if the server is already disabled in the suit
    let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &suit_id)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to get server configurations: {e}"))
        })?;

    for config in server_configs {
        if config.server_id == server_id && !config.enabled {
            return Ok(Json(SuitOperationResponse {
                id: server_id,
                name: server.name,
                result: "Server is already disabled in this configuration suit".to_string(),
                status: "Disabled".to_string(),
                allowed_operations: vec!["enable".to_string()],
            }));
        }
    }

    // Disable the server in the suit
    crate::config::suit::add_server_to_config_suit(&db.pool, &suit_id, &server_id, false)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!(
                "Failed to disable server in configuration suit: {e}"
            ))
        })?;

    // Sync server connections if merge service is available
    if let Some(merge_service) = &state.suit_merge_service {
        if let Err(e) = merge_service.sync_server_connections(&state).await {
            tracing::error!("Failed to sync server connections: {}", e);
        }
    }

    // Return success response
    Ok(Json(SuitOperationResponse {
        id: server_id,
        name: server.name,
        result: "Successfully disabled server in configuration suit".to_string(),
        status: "Disabled".to_string(),
        allowed_operations: vec!["enable".to_string()],
    }))
}

/// Batch enable servers in a configuration suit
pub async fn batch_enable_servers(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each server ID
    for server_id in payload.ids {
        // Get the server to check if it exists
        let server = crate::config::server::get_server_by_id(&db.pool, &server_id)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

        // Check if the server exists
        match server {
            Some(_) => {
                // Enable the server in the suit
                match crate::config::suit::add_server_to_config_suit(
                    &db.pool, &suit_id, &server_id, true,
                )
                .await
                {
                    Ok(_) => {
                        successful_ids.push(server_id.clone());
                    }
                    Err(e) => {
                        failed_ids
                            .insert(server_id.clone(), format!("Failed to enable server: {e}"));
                    }
                }
            }
            None => {
                failed_ids.insert(server_id.clone(), "Server not found".to_string());
            }
        }
    }

    // Sync server connections if merge service is available and any servers were enabled
    if !successful_ids.is_empty() {
        if let Some(merge_service) = &state.suit_merge_service {
            if let Err(e) = merge_service.sync_server_connections(&state).await {
                tracing::error!("Failed to sync server connections: {}", e);
            }
        }
    }

    // Return response
    Ok(Json(BatchOperationResponse {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}

/// Batch disable servers in a configuration suit
pub async fn batch_disable_servers(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each server ID
    for server_id in payload.ids {
        // Get the server to check if it exists
        let server = crate::config::server::get_server_by_id(&db.pool, &server_id)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

        // Check if the server exists
        match server {
            Some(_) => {
                // Disable the server in the suit
                match crate::config::suit::add_server_to_config_suit(
                    &db.pool, &suit_id, &server_id, false,
                )
                .await
                {
                    Ok(_) => {
                        successful_ids.push(server_id.clone());
                    }
                    Err(e) => {
                        failed_ids
                            .insert(server_id.clone(), format!("Failed to disable server: {e}"));
                    }
                }
            }
            None => {
                failed_ids.insert(server_id.clone(), "Server not found".to_string());
            }
        }
    }

    // Sync server connections if merge service is available and any servers were disabled
    if !successful_ids.is_empty() {
        if let Some(merge_service) = &state.suit_merge_service {
            if let Err(e) = merge_service.sync_server_connections(&state).await {
                tracing::error!("Failed to sync server connections: {}", e);
            }
        }
    }

    // Return response
    Ok(Json(BatchOperationResponse {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}
