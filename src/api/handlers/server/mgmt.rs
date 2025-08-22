// MCPMate Proxy API handlers for MCP server management operations
// Contains handler functions for enabling and disabling servers
//
// Server Status Synchronization Policy:
// 1. API operations have priority over config suit settings
// 2. When a server is disabled via API, it is disabled in all config suits
// 3. When a server is enabled via API, it is only enabled in the default config suit
// 4. Changes to server status in config suits trigger connection/disconnection operations
// 5. This creates a one-way synchronization where API operations take priority

use crate::api::models::server::{OperationResp, ServerManageReq, ManageAction};
use super::{common, shared::*};

// Helper functions for server management operations

/// Sync server connections by invalidating suit service cache
#[inline]
async fn sync_server_connections(state: &Arc<AppState>) -> Result<(), ApiError> {
    if let Some(merge_service) = &state.suit_merge_service {
        // Invalidate cache to force re-merging of configurations
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated suit service cache to sync server connections");
    }

    Ok(())
}

/// Sync client configurations using the client manager
#[inline]
async fn sync_client_configurations(
    state: &Arc<AppState>,
    config_suit_id: Option<String>,
) -> Result<(), ApiError> {
    // Use the helper function from suits::helpers
    crate::api::handlers::suits::helpers::sync_client_configurations(state, config_suit_id).await
}

/// Create operation response
#[inline]
fn create_operation_response(
    id: String,
    name: String,
    result: String,
    status: String,
    is_enabled: bool,
) -> Result<Json<OperationResp>, ApiError> {
    let allowed_operations = vec![if is_enabled { "disable" } else { "enable" }.to_owned()];

    Ok(Json(OperationResp {
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
) -> Result<Json<OperationResp>, ApiError> {
    match request.action {
        ManageAction::Enable => {
            // Convert to the format expected by enable_server
            let id = request.id.clone();
            let sync_query = if request.sync {
                [("sync".to_string(), "true".to_string())]
                    .iter()
                    .cloned()
                    .collect()
            } else {
                std::collections::HashMap::new()
            };

            // Call the existing enable_server logic
            enable_server_core(State(state), id, sync_query).await
        }
        ManageAction::Disable => {
            // Convert to the format expected by disable_server
            let id = request.id.clone();
            let sync_query = if request.sync {
                [("sync".to_string(), "true".to_string())]
                    .iter()
                    .cloned()
                    .collect()
            } else {
                std::collections::HashMap::new()
            };

            // Call the existing disable_server logic
            disable_server_core(State(state), id, sync_query).await
        }
    }
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
) -> Result<Json<OperationResp>, ApiError> {
    enable_server_core(State(state), id, query).await
}

/// Core enable server logic extracted for reuse
async fn enable_server_core(
    State(state): State<Arc<AppState>>,
    id: String,
    query: std::collections::HashMap<String, String>,
) -> Result<Json<OperationResp>, ApiError> {
    // Get database reference and server info
    let db = common::get_database_from_state(&state)?;
    let (server_id, server_name) = get_server_info_by_id(&db, &id).await?;

    // Update global status (early return on failure)
    update_server_global_status_wrapper(&db, &server_id, &server_name, true).await?;

    // Sync connections and client configurations
    handle_server_sync(&state, &query).await?;

    // Check if server is needed by active suits (early return if idle)
    let enabled_in_active_suits = crate::config::server::is_server_enabled_in_any_active_suit(&db.pool, &server_id)
        .await
        .unwrap_or(false);

    if !enabled_in_active_suits {
        return create_operation_response(
            "idle".to_string(),
            server_name,
            "Server globally enabled but not used by any active config suit".to_string(),
            "Idle".to_string(),
            true,
        );
    }

    // Handle connection setup
    handle_server_connection_setup(&state, &server_name).await
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
) -> Result<Json<OperationResp>, ApiError> {
    disable_server_core(state, id, query).await
}

/// Core disable server logic extracted for reuse
async fn disable_server_core(
    State(state): State<Arc<AppState>>,
    id: String,
    query: std::collections::HashMap<String, String>,
) -> Result<Json<OperationResp>, ApiError> {
    // Get database reference and server info
    let db = common::get_database_from_state(&state)?;
    let (server_id, server_name) = get_server_info_by_id(&db, &id).await?;

    // Update global status (early return on failure)
    update_server_global_status_wrapper(&db, &server_id, &server_name, false).await?;

    // Sync connections and client configurations
    handle_server_sync(&state, &query).await?;

    // Handle connection pool operations
    handle_connection_pool_disable(&state, &server_name).await
}

/// Helper function to get server info by ID with early return on error
#[inline]
async fn get_server_info_by_id(
    db: &Arc<crate::config::database::Database>,
    id: &str,
) -> Result<(String, String), ApiError> {
    let server_row = crate::config::server::get_server_by_id(&db.pool, id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{id}' not found")))?;

    let server_id = server_row.id.unwrap_or_default();
    let server_name = server_row.name;

    Ok((server_id, server_name))
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

/// Helper function to handle server connection setup
#[inline]
async fn handle_server_connection_setup(
    state: &Arc<AppState>,
    server_name: &str,
) -> Result<Json<OperationResp>, ApiError> {
    let mut pool = common::get_connection_pool_with_timeout(state).await?;

    match pool.update_server_status(server_name, true).await {
        Ok(()) => {
            let instance_id = pool
                .get_default_instance_id(server_name)
                .unwrap_or_else(|_| "default".to_string());
            create_operation_response(
                instance_id,
                server_name.to_string(),
                "Successfully enabled server with new connection".to_string(),
                "Enabled".to_string(),
                true,
            )
        }
        Err(e) => {
            tracing::warn!(
                "Failed to start server '{}' after enabling globally: {}",
                server_name,
                e
            );
            let instance_id = pool
                .get_default_instance_id(server_name)
                .unwrap_or_else(|_| "default".to_string());
            create_operation_response(
                instance_id,
                server_name.to_string(),
                format!("Server enabled in configuration but connection failed: {}", e),
                "Enabled (Connection Failed)".to_string(),
                true,
            )
        }
    }
}

/// Helper function to handle connection pool disable operations
#[inline]
async fn handle_connection_pool_disable(
    state: &Arc<AppState>,
    server_name: &str,
) -> Result<Json<OperationResp>, ApiError> {
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
                server_name.to_string(),
                "Server disabled in configuration (connection pool unavailable)".to_string(),
                "Disabled".to_string(),
                false,
            );
        }
    };

    // Early return if server not in connection pool
    if !pool.connections.contains_key(server_name) {
        return create_operation_response(
            "all".to_string(),
            server_name.to_string(),
            "Server already disabled (not in connection pool)".to_string(),
            "Disabled".to_string(),
            false,
        );
    }

    // Early return if no instances
    let instance_ids: Vec<String> = pool.connections.get(server_name).unwrap().keys().cloned().collect();
    if instance_ids.is_empty() {
        return create_operation_response(
            "all".to_string(),
            server_name.to_string(),
            "Server already disabled (no instances)".to_string(),
            "Disabled".to_string(),
            false,
        );
    }

    // Disconnect instances and clean up
    let (success_count, total_count) = disconnect_server_instances(&mut pool, server_name, &instance_ids).await;

    // Remove server from pool to enforce global disable
    pool.connections.remove(server_name);
    pool.cancellation_tokens.remove(server_name);

    let status = if success_count == total_count {
        "Disabled"
    } else {
        "Partially Disabled"
    };

    create_operation_response(
        "all".to_string(),
        server_name.to_string(),
        format!("Successfully disabled server ({success_count} of {total_count} instances disconnected)"),
        status.to_string(),
        false,
    )
}

/// Helper function to disconnect server instances
#[inline]
async fn disconnect_server_instances(
    pool: &mut tokio::sync::MutexGuard<'_, crate::core::pool::UpstreamConnectionPool>,
    server_name: &str,
    instance_ids: &[String],
) -> (usize, usize) {
    let total_count = instance_ids.len();
    let mut success_count = 0;

    for instance_id in instance_ids {
        match pool.disconnect(server_name, instance_id).await {
            Ok(()) => {
                success_count += 1;
                tracing::info!(
                    "Successfully disconnected server '{}' instance '{}'",
                    server_name,
                    instance_id
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to disconnect server '{}' instance '{}': {}",
                    server_name,
                    instance_id,
                    e
                );
            }
        }
    }

    (success_count, total_count)
}
