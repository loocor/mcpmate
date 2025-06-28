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

    // Get all available servers in the system
    let all_servers = crate::config::server::get_all_servers(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers: {e}")))?;

    // Get servers currently configured in this suit
    let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &id)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to get server configurations: {e}"))
        })?;

    // Create a map of server ID to enabled status in this suit
    let suit_server_status: HashMap<String, bool> = server_configs
        .into_iter()
        .map(|config| (config.server_id, config.enabled))
        .collect();

    // Convert to response format - include ALL servers with their suit status
    let mut server_responses = Vec::new();
    for server in all_servers {
        if let Some(server_id) = &server.id {
            // Check if this server is configured in the suit and get its enabled status
            let enabled = suit_server_status.get(server_id).copied().unwrap_or(false);

            server_responses.push(ResponseConverter::server_to_suit_response(&server, enabled));
        }
    }

    tracing::debug!(
        "Listed {} total servers for configuration suit '{}' ({})",
        server_responses.len(),
        suit.name,
        id
    );

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

    // Sync server capabilities to the configuration suit (async, non-blocking)
    // Use a semaphore to limit concurrent capability sync operations
    if let Some(inspect_service) = &state.inspect_service {
        let pool_clone = db.pool.clone();
        let suit_id_clone = suit_id.clone();
        let server_id_clone = server_id.clone();
        let inspect_service_clone = inspect_service.clone();

        // Create a semaphore to limit concurrent operations (max 2 concurrent syncs)
        static CAPABILITY_SYNC_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
        let semaphore = CAPABILITY_SYNC_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(2));

        tokio::spawn(async move {
            // Acquire semaphore permit
            let _permit = match semaphore.try_acquire() {
                Ok(permit) => permit,
                Err(_) => {
                    tracing::warn!(
                        "Too many concurrent capability sync operations. Skipping sync for server {} to suit {}",
                        server_id_clone,
                        suit_id_clone
                    );
                    return;
                }
            };

            if let Err(e) = crate::config::suit::sync_server_capabilities_to_suit(
                &pool_clone,
                &suit_id_clone,
                &server_id_clone,
                &inspect_service_clone,
            )
            .await
            {
                tracing::warn!(
                    "Failed to sync capabilities for server {} to suit {}: {}",
                    server_id_clone,
                    suit_id_clone,
                    e
                );
            } else {
                tracing::info!(
                    "Successfully synced capabilities for server {} to suit {}",
                    server_id_clone,
                    suit_id_clone
                );
            }
        });
    }

    // Sync server connections if merge service is available
    if let Some(merge_service) = &state.suit_merge_service {
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated suit service cache to sync server connections");
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
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated suit service cache to sync server connections");
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
                        // Sync server capabilities to the configuration suit (async, non-blocking)
                        if let Some(inspect_service) = &state.inspect_service {
                            let pool_clone = db.pool.clone();
                            let suit_id_clone = suit_id.clone();
                            let server_id_clone = server_id.clone();
                            let inspect_service_clone = inspect_service.clone();

                            // Use the same semaphore to limit concurrent operations
                            static CAPABILITY_SYNC_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
                            let semaphore = CAPABILITY_SYNC_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(2));

                            tokio::spawn(async move {
                                // Acquire semaphore permit
                                let _permit = match semaphore.try_acquire() {
                                    Ok(permit) => permit,
                                    Err(_) => {
                                        tracing::warn!(
                                            "Too many concurrent capability sync operations. Skipping sync for server {} to suit {}",
                                            server_id_clone,
                                            suit_id_clone
                                        );
                                        return;
                                    }
                                };

                                if let Err(e) = crate::config::suit::sync_server_capabilities_to_suit(
                                    &pool_clone,
                                    &suit_id_clone,
                                    &server_id_clone,
                                    &inspect_service_clone,
                                )
                                .await
                                {
                                    tracing::warn!(
                                        "Failed to sync capabilities for server {} to suit {}: {}",
                                        server_id_clone,
                                        suit_id_clone,
                                        e
                                    );
                                } else {
                                    tracing::info!(
                                        "Successfully synced capabilities for server {} to suit {}",
                                        server_id_clone,
                                        suit_id_clone
                                    );
                                }
                            });
                        }
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
            merge_service.invalidate_cache().await;
            tracing::debug!("Invalidated suit service cache to sync server connections");
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
            merge_service.invalidate_cache().await;
            tracing::debug!("Invalidated suit service cache to sync server connections");
        }
    }

    // Return response
    Ok(Json(BatchOperationResponse {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}
