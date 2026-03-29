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

    // Get component ID (server.rs only supports single server operations)
    if request.component_ids.len() != 1 {
        return Err(ApiError::BadRequest(
            "Server operations only support single component ID".to_string(),
        ));
    }
    let component_id = &request.component_ids[0];

    // Get server details (verify server exists)
    let _server = crate::api::handlers::server::common::get_server_or_error(&db.pool, component_id).await?;

    // Perform the action
    let (audit_action, result, status) = match request.action {
        ProfileComponentAction::Enable => {
            // Add server to profile (this enables it)
            crate::config::profile::add_server_to_profile(&db.pool, &request.profile_id, component_id, true)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to enable server: {e}")))?;

            // Sync server capabilities: add if not exists, then enable
            crate::config::profile::sync_server_capabilities(
                &db.pool,
                &request.profile_id,
                component_id,
                crate::config::profile::ServerCapabilityAction::Add,
            )
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to add server capabilities: {e}")))?;

            // Enable all capabilities for this server
            crate::config::profile::sync_server_capabilities(
                &db.pool,
                &request.profile_id,
                component_id,
                crate::config::profile::ServerCapabilityAction::Enable,
            )
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to add server capabilities: {e}")))?;

            (AuditAction::ProfileServerEnable, "enabled", "active")
        }
        ProfileComponentAction::Disable => {
            // Disable server in profile
            crate::config::profile::add_server_to_profile(&db.pool, &request.profile_id, component_id, false)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to disable server: {e}")))?;

            // Sync capabilities: disable all tools, resources, and prompts for this server
            crate::config::profile::sync_server_capabilities(
                &db.pool,
                &request.profile_id,
                component_id,
                crate::config::profile::ServerCapabilityAction::Disable,
            )
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to disable server capabilities: {e}")))?;

            (AuditAction::ProfileServerDisable, "disabled", "inactive")
        }
        ProfileComponentAction::Remove => {
            // Remove server from profile completely
            crate::config::profile::remove_server_from_profile(&db.pool, &request.profile_id, component_id)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to remove server: {e}")))?;

            // Remove all capabilities: delete all tools, resources, and prompts for this server
            crate::config::profile::sync_server_capabilities(
                &db.pool,
                &request.profile_id,
                component_id,
                crate::config::profile::ServerCapabilityAction::Remove,
            )
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to remove server capabilities: {e}")))?;

            (AuditAction::ProfileServerRemove, "removed", "removed")
        }
    };

    // Invalidate cache
    invalidate_profile_cache(&state).await;

    let response = ProfileServerManageData {
        profile_id: request.profile_id.clone(),
        results: vec![crate::api::models::profile::ComponentOperationResult {
            component_id: component_id.clone(),
            component_type: "server".to_string(),
            success: true,
            result: result.to_string(),
            error: None,
        }],
        summary: format!("Server {}", result),
        status: status.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    // Emit audit event
    let mut data = Map::new();
    data.insert("profile_id".to_string(), Value::String(request.profile_id.clone()));
    data.insert("server_id".to_string(), Value::String(component_id.clone()));
    data.insert("action".to_string(), Value::String(result.to_string()));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            audit_action,
            AuditStatus::Success,
            "POST",
            "/api/mcp/profile/servers/manage",
            Some(started_at.elapsed().as_millis() as u64),
            Some(component_id.clone()),
            Some(request.profile_id.clone()),
            Some(data),
            None,
        ),
    )
    .await;

    Ok(Json(ProfileServerManageResp::success(response)))
}
