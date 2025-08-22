// MCPMate Proxy API handlers for Config Suit resource management
// Contains handler functions for managing resources in Config Suits

use std::collections::HashMap;

use super::{check_resource_belongs_to_suit, common::*, get_resource_by_id, get_resource_or_error, get_suit_or_error};

/// List resources in a configuration suit
pub async fn list_resources(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigSuitResourcesResp>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = get_suit_or_error(&db, &id).await?;

    // Get all resources in the suit
    let resource_configs = crate::config::suit::get_resources_for_config_suit(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get resource configurations: {e}")))?;

    tracing::debug!(
        "Found {} resources in configuration suit '{}' ({})",
        resource_configs.len(),
        suit.name,
        id
    );

    // Convert to response format
    let mut resource_responses = Vec::new();
    for config in resource_configs {
        let mut allowed_operations = Vec::new();
        if config.enabled {
            allowed_operations.push("disable".to_string());
        } else {
            allowed_operations.push("enable".to_string());
        }

        resource_responses.push(ConfigSuitResourceResp {
            id: config.id.unwrap_or_default(),
            server_id: config.server_id.clone(),
            server_name: config.server_name.clone(),
            resource_uri: config.resource_uri.clone(),
            enabled: config.enabled,
            allowed_operations,
        });
    }

    // Return response
    Ok(Json(ConfigSuitResourcesResp {
        suit_id: id,
        suit_name: suit.name,
        resources: resource_responses,
    }))
}

/// Enable a resource in a configuration suit
pub async fn enable_resource(
    State(state): State<Arc<AppState>>,
    Path((suit_id, resource_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResp>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit and resource
    let _suit = get_suit_or_error(&db, &suit_id).await?;
    let resource = get_resource_or_error(&db, &resource_id).await?;

    // Check if the resource belongs to the specified suit
    check_resource_belongs_to_suit(&resource, &suit_id)?;

    // Check if the resource is already enabled
    if resource.enabled {
        return Ok(Json(SuitOperationResp {
            id: resource_id,
            name: format!("{}/{}", resource.server_name, resource.resource_uri),
            result: "Resource is already enabled in this configuration suit".to_string(),
            status: "Enabled".to_string(),
            allowed_operations: vec!["disable".to_string()],
        }));
    }

    // Enable the resource
    crate::config::suit::update_resource_enabled_status(
        &db.pool,
        &suit_id,
        &resource.server_id,
        &resource.resource_uri,
        true,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to enable resource in configuration suit: {e}")))?;

    // Return success response
    Ok(Json(SuitOperationResp {
        id: resource_id,
        name: format!("{}/{}", resource.server_name, resource.resource_uri),
        result: "Successfully enabled resource in configuration suit".to_string(),
        status: "Enabled".to_string(),
        allowed_operations: vec!["disable".to_string()],
    }))
}

/// Disable a resource in a configuration suit
pub async fn disable_resource(
    State(state): State<Arc<AppState>>,
    Path((suit_id, resource_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResp>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit and resource
    let _suit = get_suit_or_error(&db, &suit_id).await?;
    let resource = get_resource_or_error(&db, &resource_id).await?;

    // Check if the resource belongs to the specified suit
    check_resource_belongs_to_suit(&resource, &suit_id)?;

    // Check if the resource is already disabled
    if !resource.enabled {
        return Ok(Json(SuitOperationResp {
            id: resource_id,
            name: format!("{}/{}", resource.server_name, resource.resource_uri),
            result: "Resource is already disabled in this configuration suit".to_string(),
            status: "Disabled".to_string(),
            allowed_operations: vec!["enable".to_string()],
        }));
    }

    // Disable the resource
    crate::config::suit::update_resource_enabled_status(
        &db.pool,
        &suit_id,
        &resource.server_id,
        &resource.resource_uri,
        false,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to disable resource in configuration suit: {e}")))?;

    // Return success response
    Ok(Json(SuitOperationResp {
        id: resource_id,
        name: format!("{}/{}", resource.server_name, resource.resource_uri),
        result: "Successfully disabled resource in configuration suit".to_string(),
        status: "Disabled".to_string(),
        allowed_operations: vec!["enable".to_string()],
    }))
}

/// Batch enable resources in a configuration suit
pub async fn batch_enable_resources(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<BatchOperationReq>,
) -> Result<Json<BatchOperationResp>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each resource ID
    for resource_id in payload.ids {
        // Get the resource to check if it exists
        let resource = get_resource_by_id(&db, &resource_id).await;

        // Check if the resource exists and belongs to the specified suit
        match resource {
            Ok(r) => {
                if r.config_suit_id != suit_id {
                    failed_ids.insert(
                        resource_id.clone(),
                        "Resource does not belong to the specified configuration suit".to_string(),
                    );
                    continue;
                }

                // Skip if already enabled
                if r.enabled {
                    continue;
                }

                // Enable the resource
                match crate::config::suit::update_resource_enabled_status(
                    &db.pool,
                    &suit_id,
                    &r.server_id,
                    &r.resource_uri,
                    true,
                )
                .await
                {
                    Ok(_) => {
                        successful_ids.push(resource_id.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(resource_id.clone(), format!("Failed to enable resource: {e}"));
                    }
                }
            }
            Err(e) => {
                failed_ids.insert(resource_id.clone(), format!("Resource not found: {e}"));
            }
        }
    }

    // Return response
    Ok(Json(BatchOperationResp {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}

/// Batch disable resources in a configuration suit
pub async fn batch_disable_resources(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<BatchOperationReq>,
) -> Result<Json<BatchOperationResp>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each resource ID
    for resource_id in payload.ids {
        // Get the resource to check if it exists
        let resource = get_resource_by_id(&db, &resource_id).await;

        // Check if the resource exists and belongs to the specified suit
        match resource {
            Ok(r) => {
                if r.config_suit_id != suit_id {
                    failed_ids.insert(
                        resource_id.clone(),
                        "Resource does not belong to the specified configuration suit".to_string(),
                    );
                    continue;
                }

                // Skip if already disabled
                if !r.enabled {
                    continue;
                }

                // Disable the resource
                match crate::config::suit::update_resource_enabled_status(
                    &db.pool,
                    &suit_id,
                    &r.server_id,
                    &r.resource_uri,
                    false,
                )
                .await
                {
                    Ok(_) => {
                        successful_ids.push(resource_id.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(resource_id.clone(), format!("Failed to disable resource: {e}"));
                    }
                }
            }
            Err(e) => {
                failed_ids.insert(resource_id.clone(), format!("Resource not found: {e}"));
            }
        }
    }

    // Return response
    Ok(Json(BatchOperationResp {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}
