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
    is_enabled: bool,
) -> Result<Json<ServerOperationData>, ApiError> {
    let allowed_operations = vec![if is_enabled { "disable" } else { "enable" }.to_owned()];

    Ok(Json(ServerOperationData {
        id,
        name,
        result,
        status,
        allowed_operations,
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
    duration_ms: u64,
    error: Option<&ApiError>,
) {
    let mut data = Map::new();
    data.insert("sync_requested".to_string(), Value::Bool(false));
    let audit_action = match action {
        ServerManageAction::Enable => crate::audit::AuditAction::ServerEnable,
        ServerManageAction::Disable => crate::audit::AuditAction::ServerDisable,
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
/// Enable a server by setting its global availability to enabled
/// (Legacy function for backwards compatibility - consider using manage_server instead)
///
/// **Endpoint:** `POST /mcp/servers/{id}/enable`
#[tracing::instrument(skip(state), level = "debug")]
pub async fn enable_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ServerOperationData>, ApiError> {
    enable_server_core(State(state), id, query).await
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

    // Update global status (early return on failure)
    update_server_global_status_wrapper(&db, &server_id, &server_name, true).await?;

    super::common::reconcile_client_direct_exposure_after_server_constraint_change(&state, &server_id).await?;

    // Sync connections and client configurations
    handle_server_sync(&state, &query).await?;

    // Minimal behavior: only update SQLite enabled state, do not start a connection
    // Keep cache invalidation/sync above, but skip connection pool operations.
    create_operation_response(
        "none".to_string(),
        server_name,
        "Server globally enabled (DB only; no connection started)".to_string(),
        "Enabled".to_string(),
        true,
    )
}

/// Disable a server by setting its global availability to disabled
/// (Legacy function for backwards compatibility - consider using manage_server instead)
///
/// **Endpoint:** `POST /mcp/servers/{id}/disable`
#[tracing::instrument(skip(state), level = "debug")]
pub async fn disable_server(
    state: State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ServerOperationData>, ApiError> {
    disable_server_core(state, id, query).await
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

    // Update global status (early return on failure)
    update_server_global_status_wrapper(&db, &server_id, &server_name, false).await?;

    super::common::reconcile_client_direct_exposure_after_server_constraint_change(&state, &server_id).await?;

    // Sync connections and client configurations
    handle_server_sync(&state, &query).await?;

    // Handle connection pool operations
    handle_connection_pool_disable(&state, &server_id).await
}

/// Helper function to update server global status with error handling
#[inline]
async fn update_server_global_status_wrapper(
    db: &Arc<crate::config::database::Database>,
    server_id: &str,
    server_name: &str,
    enabled: bool,
) -> Result<(), ApiError> {
    let action = if enabled { "enabled" } else { "disabled" };

    match crate::config::server::update_server_global_status(&db.pool, server_id, enabled).await {
        Ok(true) => {
            tracing::info!("Set server '{}' global availability to {}", server_name, action);
            Ok(())
        }
        Ok(false) => Err(ApiError::NotFound(format!(
            "Server '{}' not found when updating global status",
            server_name
        ))),
        Err(e) => Err(ApiError::InternalError(format!(
            "Failed to update server '{}' global status: {}",
            server_name, e
        ))),
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
    // Handle connection pool timeout (early return)
    let pool_result = tokio::time::timeout(
        std::time::Duration::from_secs(crate::common::constants::timeouts::POOL_DISABLE_SEC),
        state.connection_pool.lock(),
    )
    .await;

    let mut pool = match pool_result {
        Ok(pool) => pool,
        Err(_) => {
            return create_operation_response(
                "all".to_string(),
                server_id.to_string(),
                "Server disabled in configuration (connection pool unavailable)".to_string(),
                "Disabled".to_string(),
                false,
            );
        }
    };

    // Early return if server not in connection pool
    if !pool.connections.contains_key(server_id) {
        return create_operation_response(
            "all".to_string(),
            server_id.to_string(),
            "Server already disabled (not in connection pool)".to_string(),
            "Disabled".to_string(),
            false,
        );
    }

    // Early return if no instances
    let instance_ids: Vec<String> = pool.connections.get(server_id).unwrap().keys().cloned().collect();
    if instance_ids.is_empty() {
        return create_operation_response(
            "all".to_string(),
            server_id.to_string(),
            "Server already disabled (no instances)".to_string(),
            "Disabled".to_string(),
            false,
        );
    }

    // Disconnect instances and clean up
    let (success_count, total_count) = disconnect_server_instances(&mut pool, server_id, &instance_ids).await;

    // Remove server from pool to enforce global disable
    pool.connections.remove(server_id);
    pool.cancellation_tokens.remove(server_id);

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
        false,
    )
}

/// Helper function to disconnect server instances
#[inline]
async fn disconnect_server_instances(
    pool: &mut tokio::sync::MutexGuard<'_, crate::core::pool::UpstreamConnectionPool>,
    server_id: &str,
    instance_ids: &[String],
) -> (usize, usize) {
    let total_count = instance_ids.len();
    let mut success_count = 0;

    for instance_id in instance_ids {
        match pool.disconnect(server_id, instance_id).await {
            Ok(()) => {
                success_count += 1;
                tracing::info!(
                    "Successfully disconnected server '{}' instance '{}'",
                    server_id,
                    instance_id
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to disconnect server '{}' instance '{}': {}",
                    server_id,
                    instance_id,
                    e
                );
            }
        }
    }

    (success_count, total_count)
}
