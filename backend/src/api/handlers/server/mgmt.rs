// MCPMate Proxy API handlers for MCP server management operations
// Contains handler functions for enabling and disabling servers
//
// Server Status Synchronization Policy:
// 1. API operations have priority over profile settings
// 2. When a server is disabled via API, it is disabled in all profile
// 3. When a server is enabled via API, target profiles must be explicitly specified
// 4. Changes to server status in profile trigger connection/disconnection operations
// 5. This creates a one-way synchronization where API operations take priority

use super::{common, shared::*};
use crate::api::models::server::{ServerManageAction, ServerManageReq, ServerOperationData};
use serde_json::{Map, Value};

// Helper functions for server management operations

/// Sync server connections by invalidating profile service cache
#[inline]
async fn sync_server_connections(state: &Arc<AppState>) -> Result<(), ApiError> {
    if let Some(merge_service) = &state.profile_merge_service {
        // Invalidate cache to force re-merging of configurations
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated profile service cache to sync server connections");
    }

    Ok(())
}

/// Sync client configurations using the client manager
#[inline]
async fn sync_client_configurations(
    state: &Arc<AppState>,
    profile_id: Option<String>,
) -> Result<(), ApiError> {
    // Use the helper function from profile::helpers
    crate::api::handlers::profile::helpers::sync_client_configurations(state, profile_id).await
}

/// Create operation response
#[inline]
fn create_operation_response(
    id: String,
    name: String,
    result: String,
    status: String,
    allowed_operation: &str,
) -> Result<Json<ServerOperationData>, ApiError> {
    Ok(Json(ServerOperationData {
        id,
        name,
        result,
        status,
        allowed_operations: vec![allowed_operation.to_owned()],
    }))
}

/// Unified server management function that handles enable/disable operations
/// based on the action specified in the request payload
///
/// **Endpoint:** `POST /mcp/servers/manage`
#[tracing::instrument(skip(state), level = "debug")]
pub async fn manage_server(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ServerManageReq>,
) -> Result<Json<ServerOperationData>, ApiError> {
    let started_at = std::time::Instant::now();
    let request_id = request.id.clone();
    match request.action {
        ServerManageAction::Enable => {
            // Convert to the format expected by enable_server
            let id = request.id.clone();
            let sync_query = if request.sync {
                [("sync".to_string(), "true".to_string())].iter().cloned().collect()
            } else {
                std::collections::HashMap::new()
            };

            // Call the existing enable_server logic
            let result = enable_server_core(State(state.clone()), id, sync_query).await;
            emit_server_manage_audit(
                &state,
                &request_id,
                &ServerManageAction::Enable,
                request.sync,
                started_at.elapsed().as_millis() as u64,
                result.as_ref().err(),
            )
            .await;
            result
        }
        ServerManageAction::Disable => {
            // Convert to the format expected by disable_server
            let id = request.id.clone();
            let sync_query = if request.sync {
                [("sync".to_string(), "true".to_string())].iter().cloned().collect()
            } else {
                std::collections::HashMap::new()
            };

            // Call the existing disable_server logic
            let result = disable_server_core(State(state.clone()), id, sync_query).await;
            emit_server_manage_audit(
                &state,
                &request_id,
                &ServerManageAction::Disable,
                request.sync,
                started_at.elapsed().as_millis() as u64,
                result.as_ref().err(),
            )
            .await;
            result
        }
        ServerManageAction::AllowDirectExposure => {
            let result =
                set_direct_exposure_eligibility(&state, &request.id, true, request.source_revision_set.clone()).await;
            emit_server_manage_audit(
                &state,
                &request_id,
                &ServerManageAction::AllowDirectExposure,
                request.sync,
                started_at.elapsed().as_millis() as u64,
                result.as_ref().err(),
            )
            .await;
            result
        }
        ServerManageAction::DenyDirectExposure => {
            let result =
                set_direct_exposure_eligibility(&state, &request.id, false, request.source_revision_set.clone()).await;
            emit_server_manage_audit(
                &state,
                &request_id,
                &ServerManageAction::DenyDirectExposure,
                request.sync,
                started_at.elapsed().as_millis() as u64,
                result.as_ref().err(),
            )
            .await;
            result
        }
    }
}

async fn emit_server_manage_audit(
    state: &Arc<AppState>,
    server_id: &str,
    action: &ServerManageAction,
    sync_requested: bool,
    duration_ms: u64,
    error: Option<&ApiError>,
) {
    let mut data = Map::new();
    data.insert("sync_requested".to_string(), Value::Bool(sync_requested));
    data.insert(
        "action".to_string(),
        Value::String(
            match action {
                ServerManageAction::Enable => "enable",
                ServerManageAction::Disable => "disable",
                ServerManageAction::AllowDirectExposure => "allow_direct_exposure",
                ServerManageAction::DenyDirectExposure => "deny_direct_exposure",
            }
            .to_string(),
        ),
    );
    let audit_action = match action {
        ServerManageAction::Enable => crate::audit::AuditAction::ServerEnable,
        ServerManageAction::Disable => crate::audit::AuditAction::ServerDisable,
        ServerManageAction::AllowDirectExposure | ServerManageAction::DenyDirectExposure => {
            crate::audit::AuditAction::ServerUpdate
        }
    };
    let status = if error.is_some() {
        crate::audit::AuditStatus::Failed
    } else {
        crate::audit::AuditStatus::Success
    };
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            audit_action,
            status,
            "POST",
            "/api/mcp/servers/manage",
            Some(duration_ms),
            Some(server_id.to_string()),
            None,
            Some(data),
            error.map(ToString::to_string),
        ),
    )
    .await;
}

async fn set_direct_exposure_eligibility(
    state: &Arc<AppState>,
    id: &str,
    eligible: bool,
    source_revision_set: crate::api::models::CatalogRevisionSet,
) -> Result<Json<ServerOperationData>, ApiError> {
    let db = common::get_database_from_state(state)?;
    let management = crate::core::capability::management::ServerSurfaceManagement::set_direct_exposure_eligible(
        &db.pool,
        id,
        eligible,
        source_revision_set.into_iter().collect(),
        "server_management",
    )
    .await
    .map_err(server_management_error)?;
    crate::api::handlers::profile::emit_surface_publication_audits(
        state,
        "server_management",
        None,
        "/api/mcp/servers/manage",
        management.materializations,
    )
    .await;
    create_operation_response(
        management.server_id,
        management.server_name,
        if eligible {
            "Server allowed in direct exposure surfaces"
        } else {
            "Server removed from direct exposure surfaces"
        }
        .to_string(),
        if eligible {
            "DirectExposureAllowed"
        } else {
            "DirectExposureDenied"
        }
        .to_string(),
        if eligible {
            "deny_direct_exposure"
        } else {
            "allow_direct_exposure"
        },
    )
}
/// Enable a server by setting its global availability to enabled
/// (Legacy function for backwards compatibility - consider using manage_server instead)
///
/// **Endpoint:** `POST /mcp/servers/{id}/enable`
pub async fn enable_server(
    State(_): State<Arc<AppState>>,
    Path(_): Path<String>,
    Query(_): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ServerOperationData>, ApiError> {
    Err(ApiError::BadRequest(
        "Use POST /api/mcp/servers/manage with the displayed source_revision_set".to_string(),
    ))
}

/// Core enable server logic extracted for reuse
async fn enable_server_core(
    State(state): State<Arc<AppState>>,
    id: String,
    query: std::collections::HashMap<String, String>,
) -> Result<Json<ServerOperationData>, ApiError> {
    // Get database reference and server info
    let db = common::get_database_from_state(&state)?;
    let (server_id, server_name) = common::get_server_info_by_id(&db.pool, &id).await?;

    let management = crate::core::capability::management::ServerSurfaceManagement::set_server_enabled(
        &db.pool,
        &server_id,
        true,
        "server_management",
    )
    .await
    .map_err(server_management_error)?;
    crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerGlobalStatusChanged {
        server_id: server_id.clone(),
        server_name: server_name.clone(),
        enabled: true,
    });
    crate::api::handlers::profile::emit_surface_publication_audits(
        &state,
        "server_management",
        None,
        "/api/mcp/servers/manage",
        management.materializations,
    )
    .await;

    // Sync connections and client configurations
    handle_server_sync(&state, &query).await?;

    // Minimal behavior: only update SQLite enabled state, do not start a connection
    // Keep cache invalidation/sync above, but skip connection pool operations.
    create_operation_response(
        "none".to_string(),
        server_name,
        "Server globally enabled (DB only; no connection started)".to_string(),
        "Enabled".to_string(),
        "disable",
    )
}

/// Disable a server by setting its global availability to disabled
/// (Legacy function for backwards compatibility - consider using manage_server instead)
///
/// **Endpoint:** `POST /mcp/servers/{id}/disable`
pub async fn disable_server(
    State(_): State<Arc<AppState>>,
    Path(_): Path<String>,
    Query(_): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ServerOperationData>, ApiError> {
    Err(ApiError::BadRequest(
        "Use POST /api/mcp/servers/manage with the displayed source_revision_set".to_string(),
    ))
}

/// Core disable server logic extracted for reuse
async fn disable_server_core(
    State(state): State<Arc<AppState>>,
    id: String,
    query: std::collections::HashMap<String, String>,
) -> Result<Json<ServerOperationData>, ApiError> {
    // Get database reference and server info
    let db = common::get_database_from_state(&state)?;
    let (server_id, server_name) = common::get_server_info_by_id(&db.pool, &id).await?;

    let management = crate::core::capability::management::ServerSurfaceManagement::set_server_enabled(
        &db.pool,
        &server_id,
        false,
        "server_management",
    )
    .await
    .map_err(server_management_error)?;
    crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerGlobalStatusChanged {
        server_id: server_id.clone(),
        server_name: server_name.clone(),
        enabled: false,
    });
    crate::api::handlers::profile::emit_surface_publication_audits(
        &state,
        "server_management",
        None,
        "/api/mcp/servers/manage",
        management.materializations,
    )
    .await;

    // Sync connections and client configurations
    handle_server_sync(&state, &query).await?;

    // Handle connection pool operations
    handle_connection_pool_disable(&state, &server_id).await
}

fn server_management_error(error: mcpmate_capability_store::CatalogError) -> ApiError {
    match error {
        mcpmate_capability_store::CatalogError::ConcurrencyConflict { .. } => ApiError::Conflict(error.to_string()),
        mcpmate_capability_store::CatalogError::InvalidSurfaceValue { .. } => ApiError::BadRequest(error.to_string()),
        _ => ApiError::InternalError(error.to_string()),
    }
}

/// Helper function to handle server sync operations
#[inline]
async fn handle_server_sync(
    state: &Arc<AppState>,
    query: &std::collections::HashMap<String, String>,
) -> Result<(), ApiError> {
    // Sync server connections
    sync_server_connections(state).await?;

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

    Ok(())
}

// (removed) connection-setup helper is no longer used; enabling is DB-only now

/// Helper function to handle connection pool disable operations
#[inline]
async fn handle_connection_pool_disable(
    state: &Arc<AppState>,
    server_id: &str,
) -> Result<Json<ServerOperationData>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    let (success_count, total_count) = pool.disable_server_globally(server_id).await;

    let status = if success_count == total_count {
        "Disabled"
    } else {
        "Partially Disabled"
    };

    create_operation_response(
        "all".to_string(),
        server_id.to_string(),
        format!("Successfully disabled server ({success_count} of {total_count} instances disconnected)"),
        status.to_string(),
        "enable",
    )
}
