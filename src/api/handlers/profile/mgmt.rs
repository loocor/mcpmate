// MCPMate Proxy API handlers for Profile management operations
// Contains handler functions for activating and deactivating Profile

use super::{common::*, helpers, helpers::get_profile_or_error};
use crate::api::models::profile::{
    ProfileAction, ProfileCreateReq, ProfileDeleteReq, ProfileDetailsData, ProfileDetailsReq, ProfileDetailsResp,
    ProfileListData, ProfileListReq, ProfileListResp, ProfileManageData, ProfileManageReq, ProfileManageResp,
    ProfileOperationResult, ProfileResp, ProfileUpdateReq,
};
use chrono::Utc;
use std::str::FromStr;

// ==========================================
// INTERNAL HELPER FUNCTIONS
// ==========================================

/// Validate and parse profile type
///
/// Validates the profile type string and returns the parsed enum
fn validate_profile_type(profile_type: &str) -> Result<crate::common::profile::ProfileType, ApiError> {
    crate::common::profile::ProfileType::from_str(profile_type).map_err(|_| {
        ApiError::BadRequest(format!(
            "Invalid profile type: {}. Must be one of: host_app, scenario, shared",
            profile_type
        ))
    })
}

/// Validate default profile rules
///
/// Ensures business rules for default profile are followed
fn validate_default_profile_rules(
    profile: &crate::config::models::Profile,
    is_update: bool,
) -> Result<(), ApiError> {
    // For now, we don't have specific default profile rules
    // This function is a placeholder for future business logic
    // such as "only one default profile per type" etc.
    let _ = (profile, is_update);
    Ok(())
}

/// Validate profile name uniqueness
///
/// Checks if a profile with the given name already exists, optionally excluding a specific ID
async fn validate_name_uniqueness(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    name: &str,
    exclude_id: Option<&str>,
) -> Result<(), ApiError> {
    let existing_profile = crate::config::profile::get_profile_by_name(pool, name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to check existing profile: {e}")))?;

    if let Some(existing) = existing_profile {
        // If we're excluding an ID (for updates), check if it's the same profile
        if let Some(exclude) = exclude_id {
            if existing.id.as_ref() == Some(&exclude.to_string()) {
                return Ok(()); // Same profile, name is valid
            }
        }
        return Err(ApiError::BadRequest(format!(
            "Profile with name '{}' already exists",
            name
        )));
    }

    Ok(())
}

// ==========================================
// STANDARDIZED HANDLERS
// ==========================================

/// List all profile with filtering
///
/// **Endpoint:** `GET /mcp/profile/list?filter_type={type}&profile_type={type}&limit={limit}&offset={offset}`
pub async fn profile_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileListReq>,
) -> Result<Json<ProfileListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Apply filters and pagination (simplified for now)
    let profile = crate::config::profile::get_all_profile(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile: {e}")))?;

    // Apply filters
    let filtered_profile: Vec<_> = profile
        .into_iter()
        .filter(|profile| {
            if let Some(filter_type) = &request.filter_type {
                match filter_type.as_str() {
                    "active" => profile.is_active,
                    "inactive" => !profile.is_active,
                    "all" => true,
                    _ => true,
                }
            } else {
                true
            }
        })
        .filter(|profile| {
            if let Some(profile_type) = &request.profile_type {
                profile.profile_type.to_string() == *profile_type
            } else {
                true
            }
        })
        .collect();

    let total = filtered_profile.len();

    // Apply pagination
    let limit = request.limit.unwrap_or(50).min(100);
    let offset = request.offset.unwrap_or(0);
    let paginated_profile: Vec<_> = filtered_profile.into_iter().skip(offset).take(limit).collect();

    let profile_responses = paginated_profile.iter().map(profile_to_response).collect();

    let response = ProfileListData {
        profile: profile_responses,
        total,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(ProfileListResp::success(response)))
}

/// Get details for a specific profile
///
/// **Endpoint:** `GET /mcp/profile/details?id={profile_id}`
pub async fn profile_details(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileDetailsReq>,
) -> Result<Json<ProfileDetailsResp>, ApiError> {
    let db = get_database(&state).await?;

    // Get the profile
    let profile = crate::config::profile::get_profile(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile: {e}")))?;

    let profile = match profile {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Profile with ID '{}' not found",
                request.id
            )));
        }
    };

    // Get component counts
    let servers_count = crate::config::profile::get_profile_servers(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers count: {e}")))?
        .into_iter()
        .filter(|s| s.enabled)
        .count();

    let tools_count = crate::config::profile::get_profile_tools(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tools count: {e}")))?
        .into_iter()
        .filter(|t| t.enabled)
        .count();

    // For now, set resources and prompts counts to 0 (implement later)
    let resources_count = 0;
    let prompts_count = 0;

    let response = ProfileDetailsData {
        profile: profile_to_response(&profile),
        servers_count,
        tools_count,
        resources_count,
        prompts_count,
    };

    Ok(Json(ProfileDetailsResp::success(response)))
}

/// Delete a profile
///
/// **Endpoint:** `DELETE /mcp/profile/delete`
pub async fn profile_delete(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileDeleteReq>,
) -> Result<Json<ProfileManageResp>, ApiError> {
    let db = get_database(&state).await?;

    // Get the existing profile to check if it exists and get its name
    let existing_profile = crate::config::profile::get_profile(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile: {e}")))?;

    let profile = match existing_profile {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Profile with ID '{}' not found",
                request.id
            )));
        }
    };

    // Check if it's the default profile (prevent deletion)
    if profile.is_default {
        return Err(ApiError::BadRequest("Cannot delete the default profile".to_string()));
    }

    // Delete the profile (cascade will handle related records)
    crate::config::profile::delete_profile(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete profile: {e}")))?;

    let response = ProfileManageData {
        success_count: 1,
        failed_count: 0,
        results: vec![ProfileOperationResult {
            id: request.id.clone(),
            name: profile.name,
            result: "deleted".to_string(),
            status: "inactive".to_string(),
            error: None,
        }],
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(ProfileManageResp::success(response)))
}

/// Create a new profile
///
/// **Endpoint:** `POST /mcp/profile/create`
pub async fn profile_create(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileCreateReq>,
) -> Result<Json<ProfileResp>, ApiError> {
    let db = get_database(&state).await?;

    // Validate name uniqueness
    validate_name_uniqueness(&db.pool, &request.name, None).await?;

    // Validate and parse profile type
    let profile_type = validate_profile_type(&request.profile_type)?;

    // Create new profile
    let mut new_profile = crate::config::models::Profile::new_with_description(
        request.name.clone(),
        request.description.clone(),
        profile_type,
    );

    // Set optional fields
    if let Some(multi_select) = request.multi_select {
        new_profile.multi_select = multi_select;
    }
    if let Some(priority) = request.priority {
        new_profile.priority = priority;
    }
    if let Some(is_active) = request.is_active {
        new_profile.is_active = is_active;
    }
    if let Some(is_default) = request.is_default {
        new_profile.is_default = is_default;
    }

    // Insert the new profile and get the ID
    let profile_id = crate::config::profile::upsert_profile(&db.pool, &new_profile)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create profile: {e}")))?;

    // If cloning from existing profile, copy server and tool associations
    if let Some(clone_from_id) = request.clone_from_id {
        profile_cloning_core(&db.pool, &profile_id, &clone_from_id).await?;
    }

    // Get the created profile
    let created_profile = get_profile_or_error(&db, &profile_id).await?;

    // Convert to response format
    let response = profile_to_response(&created_profile);

    Ok(Json(ProfileResp::success(response)))
}

/// Update an existing profile
///
/// **Endpoint:** `POST /mcp/profile/update`
pub async fn profile_update(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileUpdateReq>,
) -> Result<Json<ProfileResp>, ApiError> {
    let db = get_database(&state).await?;

    // 1. Get existing profile by ID
    let mut existing_profile = get_profile_or_error(&db, &request.id).await?;

    // 2. Validate name uniqueness if name is being updated
    if let Some(ref name) = request.name {
        validate_name_uniqueness(&db.pool, name, Some(&request.id)).await?;
    }

    // 3. Apply partial updates to the profile
    profile_updates_core(&mut existing_profile, &request)?;

    // 4. Validate business rules
    validate_default_profile_rules(&existing_profile, true)?;

    // 5. Save updated profile to database using dedicated update function
    crate::config::profile::update_profile(&db.pool, &existing_profile)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update profile: {e}")))?;

    // 6. Get the updated profile for response
    let updated_profile = get_profile_or_error(&db, &request.id).await?;

    // 7. Convert to response format
    let response = profile_to_response(&updated_profile);

    Ok(Json(ProfileResp::success(response)))
}

/// Manage profile operations (activate/deactivate) - supports single or multiple profile
///
/// **Endpoint:** `POST /mcp/profile/manage`
pub async fn profile_manage(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileManageReq>,
) -> Result<Json<ProfileManageResp>, ApiError> {
    let db = get_database(&state).await?;

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failed_count = 0;

    // Process each profile ID
    for profile_id in &request.ids {
        match profile_operation_core(&db.pool, profile_id, &request.action).await {
            Ok(result) => {
                success_count += 1;
                results.push(result);
            }
            Err(e) => {
                failed_count += 1;
                results.push(ProfileOperationResult {
                    id: profile_id.clone(),
                    name: "Unknown".to_string(),
                    result: "failed".to_string(),
                    status: "unknown".to_string(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Sync server connections if merge service is available and any profile were processed successfully
    if success_count > 0 {
        if let Some(merge_service) = &state.profile_merge_service {
            merge_service.invalidate_cache().await;
            tracing::debug!("Invalidated profile service cache to sync server connections");
        }
    }

    // Check if sync parameter is true and trigger client configuration synchronization
    let should_sync = request.sync.unwrap_or(false);
    if should_sync && success_count > 0 {
        // Spawn async task to sync client configurations
        let state_clone = state.clone();
        let successful_profile_ids: Vec<String> = results
            .iter()
            .filter(|r| r.error.is_none())
            .map(|r| r.id.clone())
            .collect();

        tokio::spawn(async move {
            // For activation, pass the first successful profile ID; for deactivation, pass None
            let profile_id = match request.action {
                ProfileAction::Activate => successful_profile_ids.first().cloned(),
                ProfileAction::Deactivate => None,
            };

            if let Err(e) = helpers::sync_client_configurations(&state_clone, profile_id).await {
                tracing::warn!("Failed to sync client configurations: {}", e);
            }
        });
    }

    let response = ProfileManageData {
        success_count,
        failed_count,
        results,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(ProfileManageResp::success(response)))
}

/// Process a single profile operation with complete functionality
async fn profile_operation_core(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    profile_id: &str,
    action: &ProfileAction,
) -> Result<ProfileOperationResult, ApiError> {
    // Get the existing profile
    let existing_profile = crate::config::profile::get_profile(pool, profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile: {e}")))?;

    let mut profile = match existing_profile {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Profile with ID '{}' not found",
                profile_id
            )));
        }
    };

    // Perform the action
    let (result, new_status) = match action {
        ProfileAction::Activate => {
            profile.is_active = true;
            ("activated", "active")
        }
        ProfileAction::Deactivate => {
            // Prevent deactivation of default profile
            if profile.is_default {
                return Err(ApiError::BadRequest(
                    "Cannot deactivate the default profile".to_string(),
                ));
            }
            profile.is_active = false;
            ("deactivated", "inactive")
        }
    };

    // Update the profile in database
    crate::config::profile::upsert_profile(pool, &profile)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update profile: {e}")))?;

    // Publish event to trigger server synchronization
    let enabled = matches!(action, ProfileAction::Activate);
    crate::core::events::EventBus::global().publish(crate::core::events::Event::ProfileStatusChanged {
        profile_id: profile_id.to_string(),
        enabled,
    });
    tracing::info!(
        "Published ProfileStatusChanged event for profile {}: {}",
        profile_id,
        if enabled { "activation" } else { "deactivation" }
    );

    Ok(ProfileOperationResult {
        id: profile_id.to_string(),
        name: profile.name,
        result: result.to_string(),
        status: new_status.to_string(),
        error: None,
    })
}

/// Apply partial updates to a profile
///
/// Updates only the fields that are provided in the request
fn profile_updates_core(
    profile: &mut crate::config::models::Profile,
    updates: &ProfileUpdateReq,
) -> Result<(), ApiError> {
    // Update name if provided
    if let Some(ref name) = updates.name {
        profile.name = name.clone();
    }

    // Update description if provided
    if let Some(ref description) = updates.description {
        profile.description = Some(description.clone());
    }

    // Update profile type if provided
    if let Some(ref profile_type_str) = updates.profile_type {
        profile.profile_type = validate_profile_type(profile_type_str)?;
    }

    // Update optional fields if provided
    if let Some(multi_select) = updates.multi_select {
        profile.multi_select = multi_select;
    }
    if let Some(priority) = updates.priority {
        profile.priority = priority;
    }
    if let Some(is_active) = updates.is_active {
        profile.is_active = is_active;
    }
    if let Some(is_default) = updates.is_default {
        profile.is_default = is_default;
    }

    // Update timestamp
    profile.updated_at = Some(Utc::now());

    Ok(())
}

/// Handle profile cloning operations
///
/// Copies server and tool associations from source profile to target profile
async fn profile_cloning_core(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    target_profile_id: &str,
    source_profile_id: &str,
) -> Result<(), ApiError> {
    // Check if the source profile exists
    let source_profile = crate::config::profile::get_profile(pool, source_profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get source profile: {e}")))?;

    if source_profile.is_none() {
        return Err(ApiError::NotFound(format!(
            "Source profile with ID '{}' not found",
            source_profile_id
        )));
    }

    // Copy server associations
    let server_configs = crate::config::profile::get_profile_servers(pool, source_profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server configs: {e}")))?;

    for server_config in server_configs {
        crate::config::profile::add_server_to_profile(
            pool,
            target_profile_id,
            &server_config.server_id,
            server_config.enabled,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to copy server association: {e}")))?;
    }

    // Copy tool associations
    let tool_configs = crate::config::profile::get_profile_tools(pool, source_profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool configs: {e}")))?;

    for tool_config in tool_configs {
        crate::config::profile::add_tool_to_profile(
            pool,
            target_profile_id,
            &tool_config.server_id,
            &tool_config.tool_name,
            tool_config.enabled,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to copy tool association: {e}")))?;
    }

    Ok(())
}
