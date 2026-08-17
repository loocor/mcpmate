// MCPMate Proxy API handlers for Profile management operations
// Contains handler functions for activating and deactivating Profile

use super::{common::*, helpers};
use crate::api::models::profile::{
    ProfileAction, ProfileDeleteReq, ProfileDetailsData, ProfileDetailsReq, ProfileDetailsResp, ProfileListData,
    ProfileListReq, ProfileListResp, ProfileManageData, ProfileManageReq, ProfileManageResp, ProfileOperationResult,
};
use crate::core::profile::materials::WorkflowMaterialsService;
use chrono::Utc;
use serde_json::{Map, Value};

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
    let mut transaction = db.pool.begin().await.map_err(profile_details_error)?;
    let profile: crate::config::models::Profile = sqlx::query_as("SELECT * FROM profile WHERE id = ?")
        .bind(&request.id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(profile_details_error)?
        .ok_or_else(|| ApiError::NotFound(format!("Profile with ID '{}' not found", request.id)))?;
    let servers_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM profile_server_relationships WHERE profile_id = ? AND enabled = 1")
            .bind(&request.id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(profile_details_error)?;
    let tools_count = crate::config::profile::tool::get_profile_tools_in_transaction(&mut transaction, &request.id)
        .await
        .map_err(|error| ApiError::InternalError(format!("Failed to load Profile Tool projection: {error}")))?
        .into_iter()
        .filter(|tool| tool.enabled)
        .count();
    transaction.commit().await.map_err(profile_details_error)?;

    // For now, set resources and prompts counts to 0 (implement later)
    let resources_count = 0;
    let prompts_count = 0;
    let response = ProfileDetailsData {
        profile: profile_to_response(&profile),
        servers_count: servers_count as usize,
        tools_count,
        resources_count,
        prompts_count,
    };

    Ok(Json(ProfileDetailsResp::success(response)))
}

fn profile_details_error(error: sqlx::Error) -> ApiError {
    ApiError::InternalError(format!("Failed to load Profile details projection: {error}"))
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
        request.expected_authoring_generation,
        "profile_management",
    )
    .await;
    let management = match management {
        Ok(management) => management,
        Err(error) => {
            return Err(super::map_profile_management_error(&db.pool, &request.id, Vec::new(), error).await);
        }
    };
    if let Some(skill_name) = management.workflow_skill_name.clone() {
        if let Err(error) = WorkflowMaterialsService::trash_managed_skill_directory(
            db.path.parent().unwrap_or(std::path::Path::new(".")).join("skills"),
            skill_name,
        )
        .await
        {
            tracing::warn!(error = %error, profile_id = %request.id, "Failed to move deleted workflow Skill directory to trash");
        }
    }

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
        request.expected_authoring_generations.clone().into_iter().collect(),
        "profile_management",
    )
    .await;
    let management = match management {
        Ok(management) => management,
        Err(error) => {
            return Err(super::map_profile_management_error(
                &db.pool,
                request.ids.first().map(String::as_str).unwrap_or_default(),
                Vec::new(),
                error,
            )
            .await);
        }
    };
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
