// MCPMate Proxy API handlers for Config Suit management operations
// Contains handler functions for activating and deactivating Config Suits

use std::collections::HashMap;

use super::common::*;

/// Sync client configurations using the client manager
async fn sync_client_configurations(
    state: &Arc<AppState>,
    config_suit_id: Option<String>,
) -> Result<(), ApiError> {
    // Get database reference
    let db = get_database(state).await?;

    // Create client manager
    let mut client_manager =
        crate::config::client::manager::ClientManager::new(Arc::new(db.pool.clone()));

    // Apply configuration to all enabled clients
    match client_manager.apply_config_batch(config_suit_id).await {
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

/// Activate a configuration suit
pub async fn activate_suit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = crate::config::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    // Check if the suit exists
    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{id}' not found"
            )));
        }
    };

    // Check if the suit is already active
    if suit.is_active {
        return Ok(Json(SuitOperationResponse {
            id: id.clone(),
            name: suit.name.clone(),
            result: "Configuration suit is already active".to_string(),
            status: "Active".to_string(),
            allowed_operations: vec![
                "deactivate".to_string(),
                "update".to_string(),
                "delete".to_string(),
            ],
        }));
    }

    // Activate the suit
    crate::config::suit::set_config_suit_active(&db.pool, &id, true)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to activate configuration suit: {e}"))
        })?;

    // Sync server connections if merge service is available
    if let Some(merge_service) = &state.suit_merge_service {
        if let Err(e) = merge_service.sync_server_connections(&state).await {
            tracing::error!("Failed to sync server connections: {}", e);
        }
    }

    // Check if sync parameter is true
    let should_sync = query.get("sync").map(|v| v == "true").unwrap_or(false);
    if should_sync {
        // Spawn async task to sync client configurations
        let state_clone = state.clone();
        let suit_id = id.clone();
        tokio::spawn(async move {
            if let Err(e) = sync_client_configurations(&state_clone, Some(suit_id)).await {
                tracing::warn!("Failed to sync client configurations: {}", e);
            }
        });
    }

    // Return success response
    Ok(Json(SuitOperationResponse {
        id,
        name: suit.name,
        result: "Successfully activated configuration suit".to_string(),
        status: "Active".to_string(),
        allowed_operations: vec![
            "deactivate".to_string(),
            "update".to_string(),
            "delete".to_string(),
        ],
    }))
}

/// Deactivate a configuration suit
pub async fn deactivate_suit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = crate::config::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    // Check if the suit exists
    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{id}' not found"
            )));
        }
    };

    // Check if the suit is already inactive
    if !suit.is_active {
        return Ok(Json(SuitOperationResponse {
            id: id.clone(),
            name: suit.name.clone(),
            result: "Configuration suit is already inactive".to_string(),
            status: "Inactive".to_string(),
            allowed_operations: vec![
                "activate".to_string(),
                "update".to_string(),
                "delete".to_string(),
            ],
        }));
    }

    // Deactivate the suit
    crate::config::suit::set_config_suit_active(&db.pool, &id, false)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to deactivate configuration suit: {e}"))
        })?;

    // Sync server connections if merge service is available
    if let Some(merge_service) = &state.suit_merge_service {
        if let Err(e) = merge_service.sync_server_connections(&state).await {
            tracing::error!("Failed to sync server connections: {}", e);
        }
    }

    // Check if sync parameter is true
    let should_sync = query.get("sync").map(|v| v == "true").unwrap_or(false);
    if should_sync {
        // Spawn async task to sync client configurations
        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Err(e) = sync_client_configurations(&state_clone, None).await {
                tracing::warn!("Failed to sync client configurations: {}", e);
            }
        });
    }

    // Return success response
    Ok(Json(SuitOperationResponse {
        id,
        name: suit.name,
        result: "Successfully deactivated configuration suit".to_string(),
        status: "Inactive".to_string(),
        allowed_operations: vec![
            "activate".to_string(),
            "update".to_string(),
            "delete".to_string(),
        ],
    }))
}

/// Batch activate configuration suits
pub async fn batch_activate_suits(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each suit ID
    for id in payload.ids {
        // Get the suit to check if it exists
        let suit = crate::config::suit::get_config_suit(&db.pool, &id)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to get configuration suit: {e}"))
            })?;

        // Check if the suit exists
        match suit {
            Some(s) => {
                // Skip if already active
                if s.is_active {
                    continue;
                }

                // Activate the suit
                match crate::config::suit::set_config_suit_active(&db.pool, &id, true).await {
                    Ok(_) => {
                        successful_ids.push(id.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(id.clone(), format!("Failed to activate: {e}"));
                    }
                }
            }
            None => {
                failed_ids.insert(id.clone(), "Configuration suit not found".to_string());
            }
        }
    }

    // Sync server connections if merge service is available and any suits were activated
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

/// Batch deactivate configuration suits
pub async fn batch_deactivate_suits(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each suit ID
    for id in payload.ids {
        // Get the suit to check if it exists
        let suit = crate::config::suit::get_config_suit(&db.pool, &id)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to get configuration suit: {e}"))
            })?;

        // Check if the suit exists
        match suit {
            Some(s) => {
                // Skip if already inactive
                if !s.is_active {
                    continue;
                }

                // Deactivate the suit
                match crate::config::suit::set_config_suit_active(&db.pool, &id, false).await {
                    Ok(_) => {
                        successful_ids.push(id.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(id.clone(), format!("Failed to deactivate: {e}"));
                    }
                }
            }
            None => {
                failed_ids.insert(id.clone(), "Configuration suit not found".to_string());
            }
        }
    }

    // Sync server connections if merge service is available and any suits were deactivated
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
