// MCPMate Proxy API handlers for Profile server management
// Contains handler functions for managing servers in Profile

use super::{common::*, get_profile_or_error};
use crate::api::models::profile::{
    ProfileComponentAction, ProfileComponentListReq, ProfileComponentManageReq, ProfileServerManageData,
    ProfileServerManageResp, ProfileServerResp, ProfileServersListData, ProfileServersListResp,
};
use crate::audit::{AuditAction, AuditStatus};
use serde_json::{Map, Value};

/// Invalidate profile cache if merge service is available
async fn invalidate_profile_cache(state: &Arc<AppState>) {
    if let Some(merge_service) = &state.profile_merge_service {
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated profile service cache to sync server connections");
    }
}

/// List servers in a profile (standardized version)
///
/// **Endpoint:** `GET /mcp/profile/servers/list?profile_id={profile_id}&enabled_only={bool}`
pub async fn servers_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileComponentListReq>,
) -> Result<Json<ProfileServersListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify profile exists
    let profile = crate::config::profile::get_profile(&db.pool, &request.profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile: {e}")))?;

    let profile = match profile {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Profile with ID '{}' not found",
                request.profile_id
            )));
        }
    };

    // Get servers in the profile
    let server_configs = crate::config::profile::get_profile_servers(&db.pool, &request.profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile servers: {e}")))?;

    // Convert to response format (simplified for now)
    let mut servers = Vec::new();
    for server_config in server_configs {
        // Get server details from server_config table
        if let Ok(Some(server)) = crate::config::server::get_server_by_id(&db.pool, &server_config.server_id).await {
            servers.push(ProfileServerResp {
                id: server_config.server_id.clone(),
                name: server.name,
                enabled: server_config.enabled,
                allowed_operations: vec!["enable".to_string(), "disable".to_string()],
            });
        }
    }

    // Apply enabled filter if requested
    if request.enabled_only.unwrap_or(false) {
        servers.retain(|s| s.enabled);
    }

    let total = servers.len();
    let response = ProfileServersListData {
        profile_id: request.profile_id,
        profile_name: profile.name,
        servers,
        total,
        source_revision_set: crate::core::capability::materializer::SurfaceAuthoringLoader::load_catalog_revision_set(
            &db.pool,
        )
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?
        .into_iter()
        .collect(),
    };

    Ok(Json(ProfileServersListResp::success(response)))
}

/// Manage server operations (enable/disable) in profile
///
/// **Endpoint:** `POST /mcp/profile/servers/manage`
pub async fn server_manage(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileComponentManageReq>,
) -> Result<Json<ProfileServerManageResp>, ApiError> {
    let started_at = std::time::Instant::now();
    let db = get_database(&state).await?;

    // Verify profile exists
    let _profile = get_profile_or_error(&db, &request.profile_id).await?;

    if request.component_ids.is_empty() && !matches!(request.action, ProfileComponentAction::Replace) {
        return Err(ApiError::BadRequest("component_ids cannot be empty".to_string()));
    }

    let (audit_action, result, status) = match request.action {
        ProfileComponentAction::Enable => (AuditAction::ProfileServerEnable, "enabled", "active"),
        ProfileComponentAction::Disable => (AuditAction::ProfileServerDisable, "disabled", "inactive"),
        ProfileComponentAction::Remove => (AuditAction::ProfileServerRemove, "removed", "removed"),
        ProfileComponentAction::Replace => (AuditAction::ProfileServerReplace, "replaced", "active"),
    };
    let materializations = match request.action {
        ProfileComponentAction::Replace => {
            crate::core::capability::management::ProfileSurfaceManagement::replace_servers(
                &db.pool,
                &request.profile_id,
                &request.component_ids,
                request.source_revision_set.clone().into_iter().collect(),
                "profile_management",
            )
            .await
        }
        action => {
            let relationship_action = match action {
                ProfileComponentAction::Enable => {
                    crate::core::capability::management::ProfileRelationshipAction::Enable
                }
                ProfileComponentAction::Disable => {
                    crate::core::capability::management::ProfileRelationshipAction::Disable
                }
                ProfileComponentAction::Remove => {
                    crate::core::capability::management::ProfileRelationshipAction::Remove
                }
                ProfileComponentAction::Replace => unreachable!(),
            };
            crate::core::capability::management::ProfileSurfaceManagement::mutate_servers(
                &db.pool,
                &request.profile_id,
                &request.component_ids,
                relationship_action,
                request.source_revision_set.clone().into_iter().collect(),
                "profile_management",
            )
            .await
        }
    }
    .map_err(map_catalog_error)?;

    invalidate_profile_cache(&state).await;
    super::emit_surface_publication_audits(
        &state,
        "profile_management",
        Some(&request.profile_id),
        "/api/mcp/profile/servers/manage",
        materializations,
    )
    .await;

    let response = ProfileServerManageData {
        profile_id: request.profile_id.clone(),
        results: request
            .component_ids
            .iter()
            .map(|component_id| crate::api::models::profile::ComponentOperationResult {
                component_id: component_id.clone(),
                component_type: "server".to_string(),
                success: true,
                result: result.to_string(),
                error: None,
            })
            .collect(),
        summary: format!("{} server(s) {result}", request.component_ids.len()),
        status: status.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let mut data = Map::new();
    data.insert("profile_id".to_string(), Value::String(request.profile_id.clone()));
    data.insert(
        "server_ids".to_string(),
        Value::Array(request.component_ids.iter().cloned().map(Value::String).collect()),
    );
    data.insert("action".to_string(), Value::String(result.to_string()));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            audit_action,
            AuditStatus::Success,
            "POST",
            "/api/mcp/profile/servers/manage",
            Some(started_at.elapsed().as_millis() as u64),
            (request.component_ids.len() == 1).then(|| request.component_ids[0].clone()),
            Some(request.profile_id.clone()),
            Some(data),
            None,
        ),
    )
    .await;

    Ok(Json(ProfileServerManageResp::success(response)))
}

fn map_catalog_error(error: mcpmate_capability_store::CatalogError) -> ApiError {
    match error {
        mcpmate_capability_store::CatalogError::ConcurrencyConflict { .. } => ApiError::Conflict(error.to_string()),
        _ => ApiError::InternalError(error.to_string()),
    }
}
