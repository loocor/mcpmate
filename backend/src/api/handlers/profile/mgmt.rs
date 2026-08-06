// MCPMate Proxy API handlers for Profile management operations
// Contains handler functions for activating and deactivating Profile

use super::{common::*, helpers, helpers::get_profile_or_error};
use crate::{
    api::models::profile::{
        ProfileAction, ProfileCreateReq, ProfileDeleteReq, ProfileDetailsData, ProfileDetailsReq, ProfileDetailsResp,
        ProfileListData, ProfileListReq, ProfileListResp, ProfileManageData, ProfileManageReq, ProfileManageResp,
        ProfileOperationResult, ProfileResp, ProfileUpdateReq,
    },
    config::profile::is_default_anchor_profile,
};
use chrono::Utc;
use serde_json::{Map, Value};
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
    let _ = is_update;

    if profile.is_default && !profile.is_active {
        return Err(ApiError::BadRequest("Default profiles must remain active".to_string()));
    }

    if is_default_anchor_profile(profile) {
        if !profile.is_default {
            return Err(ApiError::BadRequest(
                "Default anchor profile must stay in the default bundle".to_string(),
            ));
        }

        if !profile.is_active {
            return Err(ApiError::BadRequest(
                "Default anchor profile must stay active".to_string(),
            ));
        }
    }

    Ok(())
}

fn reconcile_default_flags(profile: &mut crate::config::models::Profile) {
    if crate::config::profile::is_default_anchor_profile(profile) {
        profile.is_active = true;
        profile.is_default = true;
    } else {
        // No automatic coupling required for user profiles; keep caller's intent.
    }
}

fn validate_profile_create_activation_contract(request: &ProfileCreateReq) -> Result<(), ApiError> {
    if request.is_active == Some(true) {
        return Err(ApiError::BadRequest(
            "Create profiles as inactive, then activate them through /api/mcp/profile/manage".to_string(),
        ));
    }
    if request.is_default == Some(true) {
        return Err(ApiError::BadRequest(
            "Create profiles as non-default, activate them through /api/mcp/profile/manage, then assign the default flag"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_profile_update_activation_contract(request: &ProfileUpdateReq) -> Result<(), ApiError> {
    if request.is_active.is_some() {
        return Err(ApiError::BadRequest(
            "Update profile activation through /api/mcp/profile/manage".to_string(),
        ));
    }
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
                // By default, exclude host_app profiles (internal use only)
                profile.profile_type != crate::common::profile::ProfileType::HostApp
            }
        })
        .collect();

    let total = filtered_profile.len();

    // Apply pagination
    let limit = request.limit.unwrap_or(50).min(100);
    let offset = request.offset.unwrap_or(0);
    let paginated_profile: Vec<_> = filtered_profile.into_iter().skip(offset).take(limit).collect();

    let profile_responses = paginated_profile.iter().map(profile_to_response).collect();
    let source_revision_set = sqlx::query_as::<_, (String, i64)>(
        "SELECT server_id, catalog_revision FROM capability_server_snapshots ORDER BY server_id",
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|error| ApiError::InternalError(format!("Failed to load catalog revisions: {error}")))?
    .into_iter()
    .collect();

    let response = ProfileListData {
        profile: profile_responses,
        total,
        timestamp: Utc::now().to_rfc3339(),
        source_revision_set,
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
    let source_revision_set = sqlx::query_as::<_, (String, i64)>(
        "SELECT server_id, catalog_revision FROM capability_server_snapshots ORDER BY server_id",
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|error| ApiError::InternalError(format!("Failed to load catalog revisions: {error}")))?
    .into_iter()
    .collect();

    let response = ProfileDetailsData {
        profile: profile_to_response(&profile),
        servers_count,
        tools_count,
        resources_count,
        prompts_count,
        source_revision_set,
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
    let started_at = std::time::Instant::now();
    let db = get_database(&state).await?;
    let management = crate::core::capability::management::ProfileSurfaceManagement::delete_profile(
        &db.pool,
        &request.id,
        request.source_revision_set.clone().into_iter().collect(),
        "profile_management",
    )
    .await
    .map_err(profile_management_error)?;

    let response = ProfileManageData {
        success_count: 1,
        failed_count: 0,
        results: vec![ProfileOperationResult {
            id: request.id.clone(),
            name: management.profile_name,
            result: "deleted".to_string(),
            status: "inactive".to_string(),
            error: None,
        }],
        timestamp: Utc::now().to_rfc3339(),
    };

    let response = Json(ProfileManageResp::success(response));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ProfileDelete,
            crate::audit::AuditStatus::Success,
            "DELETE",
            "/api/mcp/profile/delete",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            Some(request.id),
            None,
            None,
        ),
    )
    .await;
    super::emit_surface_publication_audits(
        &state,
        "profile_management",
        response
            .0
            .data
            .as_ref()
            .and_then(|data| data.results.first())
            .map(|result| result.id.as_str()),
        "/api/mcp/profile/delete",
        management.materializations,
    )
    .await;
    Ok(response)
}

/// Create a new profile
///
/// **Endpoint:** `POST /mcp/profile/create`
pub async fn profile_create(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileCreateReq>,
) -> Result<Json<ProfileResp>, ApiError> {
    let started_at = std::time::Instant::now();
    validate_profile_create_activation_contract(&request)?;
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

    let response = Json(ProfileResp::success(response));
    let mut data = Map::new();
    data.insert("profile_name".to_string(), Value::String(created_profile.name));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ProfileCreate,
            crate::audit::AuditStatus::Success,
            "POST",
            "/api/mcp/profile/create",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            response.0.data.as_ref().map(|profile| profile.id.clone()),
            Some(data),
            None,
        ),
    )
    .await;
    Ok(response)
}

/// Update an existing profile
///
/// **Endpoint:** `POST /mcp/profile/update`
pub async fn profile_update(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileUpdateReq>,
) -> Result<Json<ProfileResp>, ApiError> {
    let started_at = std::time::Instant::now();
    validate_profile_update_activation_contract(&request)?;
    let db = get_database(&state).await?;

    // 1. Get existing profile by ID
    let mut existing_profile = get_profile_or_error(&db, &request.id).await?;

    // 2. Validate name uniqueness if name is being updated
    if let Some(ref name) = request.name {
        validate_name_uniqueness(&db.pool, name, Some(&request.id)).await?;
    }

    // 3. Apply partial updates to the profile
    profile_updates_core(&mut existing_profile, &request)?;
    reconcile_default_flags(&mut existing_profile);

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

    let response = Json(ProfileResp::success(response));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ProfileUpdate,
            crate::audit::AuditStatus::Success,
            "POST",
            "/api/mcp/profile/update",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            Some(request.id),
            None,
            None,
        ),
    )
    .await;
    Ok(response)
}

/// Manage profile operations (activate/deactivate) - supports single or multiple profile
///
/// **Endpoint:** `POST /mcp/profile/manage`
pub async fn profile_manage(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileManageReq>,
) -> Result<Json<ProfileManageResp>, ApiError> {
    let started_at = std::time::Instant::now();
    let db = get_database(&state).await?;
    let activation_action = match &request.action {
        ProfileAction::Activate => crate::core::capability::management::ProfileActivationAction::Activate,
        ProfileAction::Deactivate => crate::core::capability::management::ProfileActivationAction::Deactivate,
    };
    let management = crate::core::capability::management::ProfileSurfaceManagement::set_profiles_active(
        &db.pool,
        &request.ids,
        activation_action,
        request.source_revision_set.clone().into_iter().collect(),
        "profile_management",
    )
    .await
    .map_err(profile_management_error)?;
    let results = management
        .mutations
        .iter()
        .map(|mutation| ProfileOperationResult {
            id: mutation.profile_id.clone(),
            name: mutation.name.clone(),
            result: if mutation.is_active {
                "activated".to_string()
            } else {
                "deactivated".to_string()
            },
            status: if mutation.is_active {
                "active".to_string()
            } else {
                "inactive".to_string()
            },
            error: None,
        })
        .collect::<Vec<_>>();
    let success_count = results.len();
    let failed_count = 0;

    // Sync server connections if merge service is available and any profile were processed successfully
    if success_count > 0 {
        if let Some(merge_service) = &state.profile_merge_service {
            merge_service.invalidate_cache().await;
            tracing::debug!("Invalidated profile service cache to sync server connections");
        }
    }

    // Check if sync parameter is true and trigger client configuration synchronization
    let should_sync = request.sync.unwrap_or(false);
    let requested_action = request.action.clone();
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
            let profile_id = match requested_action {
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

    let response = Json(ProfileManageResp::success(response));
    let audit_action = match request.action {
        ProfileAction::Activate => crate::audit::AuditAction::ProfileActivate,
        ProfileAction::Deactivate => crate::audit::AuditAction::ProfileDeactivate,
    };
    let mut data = Map::new();
    data.insert("profile_count".to_string(), Value::from(request.ids.len() as u64));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            audit_action,
            crate::audit::AuditStatus::Success,
            "POST",
            "/api/mcp/profile/manage",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            request.ids.first().cloned(),
            Some(data),
            None,
        ),
    )
    .await;
    for mutation in &management.mutations {
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ProfileStatusChanged {
            profile_id: mutation.profile_id.clone(),
            enabled: mutation.is_active,
        });
    }
    super::emit_surface_publication_audits(
        &state,
        "profile_management",
        request.ids.first().map(String::as_str),
        "/api/mcp/profile/manage",
        management.materializations,
    )
    .await;
    Ok(response)
}

fn profile_management_error(error: mcpmate_capability_store::CatalogError) -> ApiError {
    match error {
        mcpmate_capability_store::CatalogError::ConcurrencyConflict { .. } => ApiError::Conflict(error.to_string()),
        mcpmate_capability_store::CatalogError::InvalidSurfaceValue { .. } => ApiError::BadRequest(error.to_string()),
        _ => ApiError::InternalError(error.to_string()),
    }
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
    if let Some(is_default) = updates.is_default {
        if is_default_anchor_profile(profile) && !is_default {
            return Err(ApiError::BadRequest(
                "Default anchor profile must stay in the default bundle".to_string(),
            ));
        }
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
            &tool_config.ref_id,
            tool_config.enabled,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to copy tool association: {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcpmate_capability_store::{
        CapabilityCatalog, CapabilityKind, CapabilityObservation, CapabilityPayload, CatalogRecord, DeclarationState,
        InventoryState, KindObservation, SqliteCapabilityCatalog,
    };
    use rmcp::model::{InitializeResult, Tool};
    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn profile_create_requires_activation_through_management_endpoint() {
        let request = ProfileCreateReq {
            name: "Research".to_string(),
            description: None,
            profile_type: "scenario".to_string(),
            multi_select: None,
            priority: None,
            is_active: Some(true),
            is_default: None,
            clone_from_id: None,
        };

        let error = validate_profile_create_activation_contract(&request).unwrap_err();

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[test]
    fn profile_create_requires_default_assignment_after_activation() {
        let request = ProfileCreateReq {
            name: "Research".to_string(),
            description: None,
            profile_type: "scenario".to_string(),
            multi_select: None,
            priority: None,
            is_active: Some(false),
            is_default: Some(true),
            clone_from_id: None,
        };

        let error = validate_profile_create_activation_contract(&request).unwrap_err();

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[test]
    fn profile_update_requires_activation_through_management_endpoint() {
        let request = ProfileUpdateReq {
            id: "profile-1".to_string(),
            name: None,
            description: None,
            profile_type: None,
            multi_select: None,
            priority: None,
            is_active: Some(false),
            is_default: None,
        };

        let error = validate_profile_update_activation_contract(&request).unwrap_err();

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn profile_clone_preserves_tool_capability_ref_identity() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::test_helpers::prepare_config_database(&pool).await;
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .unwrap();
        crate::config::client::init::initialize_client_table(&pool)
            .await
            .unwrap();
        crate::config::database::initialize_capability_catalog(&pool)
            .await
            .unwrap();
        crate::config::profile::init::initialize_profile_tables(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO server_config (id, name, server_type, command, enabled) VALUES ('server-a', 'Server A', 'stdio', '', 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            INSERT INTO profile (id, name, description, type, role)
            VALUES ('profile-source', 'Source', '', 'shared', 'user'),
                   ('profile-target', 'Target', '', 'shared', 'user')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let tool: Tool = serde_json::from_value(json!({
            "name": "analyze",
            "description": "Analyze",
            "inputSchema": {"type": "object"}
        }))
        .unwrap();
        let record = CatalogRecord::materialize(
            "server-a",
            "analyze",
            "server_a__analyze",
            CapabilityPayload::Tool(tool),
        )
        .unwrap();
        let initialize: InitializeResult = serde_json::from_value(json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "Server A", "version": "1.0.0"}
        }))
        .unwrap();
        SqliteCapabilityCatalog::new(pool.clone())
            .commit_observation(CapabilityObservation::new(
                "server-a",
                "Server A",
                "config-v1",
                initialize,
                vec![KindObservation::new(
                    CapabilityKind::Tools,
                    DeclarationState::Supported,
                    InventoryState::Complete,
                )],
                vec![record.clone()],
            ))
            .await
            .unwrap();
        crate::config::profile::add_tool_to_profile(&pool, "profile-source", "server-a", record.ref_id.as_str(), true)
            .await
            .unwrap();

        profile_cloning_core(&pool, "profile-target", "profile-source")
            .await
            .unwrap();

        let cloned = crate::config::profile::get_profile_tools(&pool, "profile-target")
            .await
            .unwrap();
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned[0].ref_id, record.ref_id.to_string());
    }
}
