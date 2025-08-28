// MCPMate Proxy API handlers for Config Suit management operations
// Contains handler functions for activating and deactivating Config Suits

use super::{common::*, helpers, helpers::get_suit_or_error};
use crate::api::models::suits::{
    SuitAction, SuitCreateReq, SuitDeleteReq, SuitDetailsData, SuitDetailsReq, SuitDetailsResp, SuitManageData,
    SuitManageReq, SuitManageResp, SuitOperationResult, SuitResp, SuitUpdateReq, SuitsListData, SuitsListReq,
    SuitsListResp,
};
use chrono::Utc;
use std::str::FromStr;

// ==========================================
// INTERNAL HELPER FUNCTIONS
// ==========================================

/// Validate and parse suit type
///
/// Validates the suit type string and returns the parsed enum
fn validate_suit_type(suit_type: &str) -> Result<crate::common::config::ConfigSuitType, ApiError> {
    crate::common::config::ConfigSuitType::from_str(suit_type).map_err(|_| {
        ApiError::BadRequest(format!(
            "Invalid configuration suit type: {}. Must be one of: host_app, scenario, shared",
            suit_type
        ))
    })
}

/// Validate default suit rules
///
/// Ensures business rules for default suits are followed
fn validate_default_suit_rules(
    suit: &crate::config::models::ConfigSuit,
    is_update: bool,
) -> Result<(), ApiError> {
    // For now, we don't have specific default suit rules
    // This function is a placeholder for future business logic
    // such as "only one default suit per type" etc.
    let _ = (suit, is_update);
    Ok(())
}

/// Validate suit name uniqueness
///
/// Checks if a suit with the given name already exists, optionally excluding a specific ID
async fn validate_name_uniqueness(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    name: &str,
    exclude_id: Option<&str>,
) -> Result<(), ApiError> {
    let existing_suit = crate::config::suit::get_config_suit_by_name(pool, name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to check existing suit: {e}")))?;

    if let Some(existing) = existing_suit {
        // If we're excluding an ID (for updates), check if it's the same suit
        if let Some(exclude) = exclude_id {
            if existing.id.as_ref() == Some(&exclude.to_string()) {
                return Ok(()); // Same suit, name is valid
            }
        }
        return Err(ApiError::BadRequest(format!(
            "Configuration suit with name '{}' already exists",
            name
        )));
    }

    Ok(())
}

// ==========================================
// STANDARDIZED HANDLERS
// ==========================================

/// List all configuration suits with filtering
///
/// **Endpoint:** `GET /mcp/suits/list?filter_type={type}&suit_type={type}&limit={limit}&offset={offset}`
pub async fn suits_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitsListReq>,
) -> Result<Json<SuitsListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Apply filters and pagination (simplified for now)
    let suits = crate::config::suit::get_all_config_suits(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suits: {e}")))?;

    // Apply filters
    let filtered_suits: Vec<_> = suits
        .into_iter()
        .filter(|suit| {
            if let Some(filter_type) = &request.filter_type {
                match filter_type.as_str() {
                    "active" => suit.is_active,
                    "inactive" => !suit.is_active,
                    "all" => true,
                    _ => true,
                }
            } else {
                true
            }
        })
        .filter(|suit| {
            if let Some(suit_type) = &request.suit_type {
                suit.suit_type.to_string() == *suit_type
            } else {
                true
            }
        })
        .collect();

    let total = filtered_suits.len();

    // Apply pagination
    let limit = request.limit.unwrap_or(50).min(100);
    let offset = request.offset.unwrap_or(0);
    let paginated_suits: Vec<_> = filtered_suits.into_iter().skip(offset).take(limit).collect();

    let suit_responses = paginated_suits.iter().map(suit_to_response).collect();

    let response = SuitsListData {
        suits: suit_responses,
        total,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(SuitsListResp::success(response)))
}

/// Get details for a specific configuration suit
///
/// **Endpoint:** `GET /mcp/suits/details?id={suit_id}`
pub async fn suit_details(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitDetailsReq>,
) -> Result<Json<SuitDetailsResp>, ApiError> {
    let db = get_database(&state).await?;

    // Get the configuration suit
    let suit = crate::config::suit::get_config_suit(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                request.id
            )));
        }
    };

    // Get component counts
    let servers_count = crate::config::suit::get_config_suit_servers(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers count: {e}")))?
        .into_iter()
        .filter(|s| s.enabled)
        .count();

    let tools_count = crate::config::suit::get_config_suit_tools(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tools count: {e}")))?
        .into_iter()
        .filter(|t| t.enabled)
        .count();

    // For now, set resources and prompts counts to 0 (implement later)
    let resources_count = 0;
    let prompts_count = 0;

    let response = SuitDetailsData {
        suit: suit_to_response(&suit),
        servers_count,
        tools_count,
        resources_count,
        prompts_count,
    };

    Ok(Json(SuitDetailsResp::success(response)))
}

/// Delete a configuration suit
///
/// **Endpoint:** `DELETE /mcp/suits/delete`
pub async fn suit_delete(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuitDeleteReq>,
) -> Result<Json<SuitManageResp>, ApiError> {
    let db = get_database(&state).await?;

    // Get the existing suit to check if it exists and get its name
    let existing_suit = crate::config::suit::get_config_suit(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    let suit = match existing_suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                request.id
            )));
        }
    };

    // Check if it's the default suit (prevent deletion)
    if suit.is_default {
        return Err(ApiError::BadRequest(
            "Cannot delete the default configuration suit".to_string(),
        ));
    }

    // Delete the suit (cascade will handle related records)
    crate::config::suit::delete_config_suit(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete configuration suit: {e}")))?;

    let response = SuitManageData {
        success_count: 1,
        failed_count: 0,
        results: vec![SuitOperationResult {
            id: request.id.clone(),
            name: suit.name,
            result: "deleted".to_string(),
            status: "inactive".to_string(),
            error: None,
        }],
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(SuitManageResp::success(response)))
}

/// Create a new configuration suit
///
/// **Endpoint:** `POST /mcp/suits/create`
pub async fn suit_create(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuitCreateReq>,
) -> Result<Json<SuitResp>, ApiError> {
    let db = get_database(&state).await?;

    // Validate name uniqueness
    validate_name_uniqueness(&db.pool, &request.name, None).await?;

    // Validate and parse suit type
    let suit_type = validate_suit_type(&request.suit_type)?;

    // Create new configuration suit
    let mut new_suit = crate::config::models::ConfigSuit::new_with_description(
        request.name.clone(),
        request.description.clone(),
        suit_type,
    );

    // Set optional fields
    if let Some(multi_select) = request.multi_select {
        new_suit.multi_select = multi_select;
    }
    if let Some(priority) = request.priority {
        new_suit.priority = priority;
    }
    if let Some(is_active) = request.is_active {
        new_suit.is_active = is_active;
    }
    if let Some(is_default) = request.is_default {
        new_suit.is_default = is_default;
    }

    // Insert the new suit and get the ID
    let suit_id = crate::config::suit::upsert_config_suit(&db.pool, &new_suit)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create configuration suit: {e}")))?;

    // If cloning from existing suit, copy server and tool associations
    if let Some(clone_from_id) = request.clone_from_id {
        suit_cloning_core(&db.pool, &suit_id, &clone_from_id).await?;
    }

    // Get the created suit
    let created_suit = get_suit_or_error(&db, &suit_id).await?;

    // Convert to response format
    let response = suit_to_response(&created_suit);

    Ok(Json(SuitResp::success(response)))
}

/// Update an existing configuration suit
///
/// **Endpoint:** `POST /mcp/suits/update`
pub async fn suit_update(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuitUpdateReq>,
) -> Result<Json<SuitResp>, ApiError> {
    let db = get_database(&state).await?;

    // 1. Get existing suit by ID
    let mut existing_suit = get_suit_or_error(&db, &request.id).await?;

    // 2. Validate name uniqueness if name is being updated
    if let Some(ref name) = request.name {
        validate_name_uniqueness(&db.pool, name, Some(&request.id)).await?;
    }

    // 3. Apply partial updates to the suit
    suit_updates_core(&mut existing_suit, &request)?;

    // 4. Validate business rules
    validate_default_suit_rules(&existing_suit, true)?;

    // 5. Save updated suit to database using dedicated update function
    crate::config::suit::update_config_suit(&db.pool, &existing_suit)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update configuration suit: {e}")))?;

    // 6. Get the updated suit for response
    let updated_suit = get_suit_or_error(&db, &request.id).await?;

    // 7. Convert to response format
    let response = suit_to_response(&updated_suit);

    Ok(Json(SuitResp::success(response)))
}

/// Manage suit operations (activate/deactivate) - supports single or multiple suits
///
/// **Endpoint:** `POST /mcp/suits/manage`
pub async fn suit_manage(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuitManageReq>,
) -> Result<Json<SuitManageResp>, ApiError> {
    let db = get_database(&state).await?;

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failed_count = 0;

    // Process each suit ID
    for suit_id in &request.ids {
        match suit_operation_core(&db.pool, suit_id, &request.action).await {
            Ok(result) => {
                success_count += 1;
                results.push(result);
            }
            Err(e) => {
                failed_count += 1;
                results.push(SuitOperationResult {
                    id: suit_id.clone(),
                    name: "Unknown".to_string(),
                    result: "failed".to_string(),
                    status: "unknown".to_string(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Sync server connections if merge service is available and any suits were processed successfully
    if success_count > 0 {
        if let Some(merge_service) = &state.suit_merge_service {
            merge_service.invalidate_cache().await;
            tracing::debug!("Invalidated suit service cache to sync server connections");
        }
    }

    // Check if sync parameter is true and trigger client configuration synchronization
    let should_sync = request.sync.unwrap_or(false);
    if should_sync && success_count > 0 {
        // Spawn async task to sync client configurations
        let state_clone = state.clone();
        let successful_suit_ids: Vec<String> = results
            .iter()
            .filter(|r| r.error.is_none())
            .map(|r| r.id.clone())
            .collect();

        tokio::spawn(async move {
            // For activation, pass the first successful suit ID; for deactivation, pass None
            let suit_id = match request.action {
                SuitAction::Activate => successful_suit_ids.first().cloned(),
                SuitAction::Deactivate => None,
            };

            if let Err(e) = helpers::sync_client_configurations(&state_clone, suit_id).await {
                tracing::warn!("Failed to sync client configurations: {}", e);
            }
        });
    }

    let response = SuitManageData {
        success_count,
        failed_count,
        results,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(SuitManageResp::success(response)))
}

/// Process a single suit operation with complete functionality
async fn suit_operation_core(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    suit_id: &str,
    action: &SuitAction,
) -> Result<SuitOperationResult, ApiError> {
    // Get the existing suit
    let existing_suit = crate::config::suit::get_config_suit(pool, suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    let mut suit = match existing_suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                suit_id
            )));
        }
    };

    // Perform the action
    let (result, new_status) = match action {
        SuitAction::Activate => {
            suit.is_active = true;
            ("activated", "active")
        }
        SuitAction::Deactivate => {
            // Prevent deactivation of default suit
            if suit.is_default {
                return Err(ApiError::BadRequest(
                    "Cannot deactivate the default configuration suit".to_string(),
                ));
            }
            suit.is_active = false;
            ("deactivated", "inactive")
        }
    };

    // Update the suit in database
    crate::config::suit::upsert_config_suit(pool, &suit)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update configuration suit: {e}")))?;

    // Publish event to trigger server synchronization
    let enabled = matches!(action, SuitAction::Activate);
    crate::core::events::EventBus::global().publish(crate::core::events::Event::ConfigSuitStatusChanged {
        suit_id: suit_id.to_string(),
        enabled,
    });
    tracing::info!(
        "Published ConfigSuitStatusChanged event for suit {}: {}",
        suit_id,
        if enabled { "activation" } else { "deactivation" }
    );

    Ok(SuitOperationResult {
        id: suit_id.to_string(),
        name: suit.name,
        result: result.to_string(),
        status: new_status.to_string(),
        error: None,
    })
}

/// Apply partial updates to a suit
///
/// Updates only the fields that are provided in the request
fn suit_updates_core(
    suit: &mut crate::config::models::ConfigSuit,
    updates: &SuitUpdateReq,
) -> Result<(), ApiError> {
    // Update name if provided
    if let Some(ref name) = updates.name {
        suit.name = name.clone();
    }

    // Update description if provided
    if let Some(ref description) = updates.description {
        suit.description = Some(description.clone());
    }

    // Update suit type if provided
    if let Some(ref suit_type_str) = updates.suit_type {
        suit.suit_type = validate_suit_type(suit_type_str)?;
    }

    // Update optional fields if provided
    if let Some(multi_select) = updates.multi_select {
        suit.multi_select = multi_select;
    }
    if let Some(priority) = updates.priority {
        suit.priority = priority;
    }
    if let Some(is_active) = updates.is_active {
        suit.is_active = is_active;
    }
    if let Some(is_default) = updates.is_default {
        suit.is_default = is_default;
    }

    // Update timestamp
    suit.updated_at = Some(Utc::now());

    Ok(())
}

/// Handle suit cloning operations
///
/// Copies server and tool associations from source suit to target suit
async fn suit_cloning_core(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    target_suit_id: &str,
    source_suit_id: &str,
) -> Result<(), ApiError> {
    // Check if the source suit exists
    let source_suit = crate::config::suit::get_config_suit(pool, source_suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get source suit: {e}")))?;

    if source_suit.is_none() {
        return Err(ApiError::NotFound(format!(
            "Source configuration suit with ID '{}' not found",
            source_suit_id
        )));
    }

    // Copy server associations
    let server_configs = crate::config::suit::get_config_suit_servers(pool, source_suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server configs: {e}")))?;

    for server_config in server_configs {
        crate::config::suit::add_server_to_config_suit(
            pool,
            target_suit_id,
            &server_config.server_id,
            server_config.enabled,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to copy server association: {e}")))?;
    }

    // Copy tool associations
    let tool_configs = crate::config::suit::get_config_suit_tools(pool, source_suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool configs: {e}")))?;

    for tool_config in tool_configs {
        crate::config::suit::add_tool_to_config_suit(
            pool,
            target_suit_id,
            &tool_config.server_id,
            &tool_config.tool_name,
            tool_config.enabled,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to copy tool association: {e}")))?;
    }

    Ok(())
}
