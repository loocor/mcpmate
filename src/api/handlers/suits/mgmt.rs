// MCPMate Proxy API handlers for Config Suit management operations
// Contains handler functions for activating and deactivating Config Suits

use super::common::*;
use std::collections::HashMap;

/// Activate a configuration suit
pub async fn activate_suit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = crate::conf::operations::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {}", e)))?;

    // Check if the suit exists
    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                id
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
    crate::conf::operations::suit::set_config_suit_active(&db.pool, &id, true)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to activate configuration suit: {}", e))
        })?;

    // Update Config Suit merge service cache
    if let Some(merge_service) = &state.suit_merge_service {
        if let Err(e) = merge_service.update_cache().await {
            tracing::error!("Failed to update Config Suit merge cache: {}", e);
        } else {
            // Sync server connections
            if let Err(e) = merge_service.sync_server_connections(&state).await {
                tracing::error!("Failed to sync server connections: {}", e);
            }
        }
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
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = crate::conf::operations::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {}", e)))?;

    // Check if the suit exists
    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                id
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
    crate::conf::operations::suit::set_config_suit_active(&db.pool, &id, false)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to deactivate configuration suit: {}", e))
        })?;

    // Update Config Suit merge service cache
    if let Some(merge_service) = &state.suit_merge_service {
        if let Err(e) = merge_service.update_cache().await {
            tracing::error!("Failed to update Config Suit merge cache: {}", e);
        } else {
            // Sync server connections
            if let Err(e) = merge_service.sync_server_connections(&state).await {
                tracing::error!("Failed to sync server connections: {}", e);
            }
        }
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
        let suit = crate::conf::operations::suit::get_config_suit(&db.pool, &id)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to get configuration suit: {}", e))
            })?;

        // Check if the suit exists
        match suit {
            Some(s) => {
                // Skip if already active
                if s.is_active {
                    continue;
                }

                // Activate the suit
                match crate::conf::operations::suit::set_config_suit_active(&db.pool, &id, true)
                    .await
                {
                    Ok(_) => {
                        successful_ids.push(id.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(id.clone(), format!("Failed to activate: {}", e));
                    }
                }
            }
            None => {
                failed_ids.insert(id.clone(), "Configuration suit not found".to_string());
            }
        }
    }

    // Update Config Suit merge service cache if any suits were activated
    if !successful_ids.is_empty() {
        if let Some(merge_service) = &state.suit_merge_service {
            if let Err(e) = merge_service.update_cache().await {
                tracing::error!("Failed to update Config Suit merge cache: {}", e);
            } else {
                // Sync server connections
                if let Err(e) = merge_service.sync_server_connections(&state).await {
                    tracing::error!("Failed to sync server connections: {}", e);
                }
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
        let suit = crate::conf::operations::suit::get_config_suit(&db.pool, &id)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to get configuration suit: {}", e))
            })?;

        // Check if the suit exists
        match suit {
            Some(s) => {
                // Skip if already inactive
                if !s.is_active {
                    continue;
                }

                // Deactivate the suit
                match crate::conf::operations::suit::set_config_suit_active(&db.pool, &id, false)
                    .await
                {
                    Ok(_) => {
                        successful_ids.push(id.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(id.clone(), format!("Failed to deactivate: {}", e));
                    }
                }
            }
            None => {
                failed_ids.insert(id.clone(), "Configuration suit not found".to_string());
            }
        }
    }

    // Update Config Suit merge service cache if any suits were deactivated
    if !successful_ids.is_empty() {
        if let Some(merge_service) = &state.suit_merge_service {
            if let Err(e) = merge_service.update_cache().await {
                tracing::error!("Failed to update Config Suit merge cache: {}", e);
            } else {
                // Sync server connections
                if let Err(e) = merge_service.sync_server_connections(&state).await {
                    tracing::error!("Failed to sync server connections: {}", e);
                }
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
