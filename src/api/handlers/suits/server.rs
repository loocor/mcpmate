// MCPMate Proxy API handlers for Config Suit server management
// Contains handler functions for managing servers in Config Suits

use super::{common::*, get_suit_or_error};
use crate::api::models::suits::{
    SuitComponentAction, SuitComponentListReq, SuitComponentManageReq, SuitServerManageData, SuitServerManageResp,
    SuitServerResp, SuitServersListData, SuitServersListResp,
};
use sqlx::{Pool, Sqlite};

// Shared semaphore to limit concurrent capability sync operations (max 2 concurrent syncs)
static CAPABILITY_SYNC_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();

/// Get the capability sync semaphore, initializing it if needed
fn get_capability_sync_semaphore() -> &'static tokio::sync::Semaphore {
    CAPABILITY_SYNC_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(2))
}

/// Spawn async capability sync task with semaphore protection
fn spawn_capability_sync(
    pool: Pool<Sqlite>,
    suit_id: String,
    server_id: String,
) {
    let semaphore = get_capability_sync_semaphore();

    tokio::spawn(async move {
        // Acquire semaphore permit
        let _permit = match semaphore.try_acquire() {
            Ok(permit) => permit,
            Err(_) => {
                tracing::warn!(
                    "Too many concurrent capability sync operations. Skipping sync for server {} to suit {}",
                    server_id,
                    suit_id
                );
                return;
            }
        };

        if let Err(e) = crate::config::suit::sync_server_capabilities_to_suit(&pool, &suit_id, &server_id).await {
            tracing::warn!(
                "Failed to sync capabilities for server {} to suit {}: {}",
                server_id,
                suit_id,
                e
            );
        } else {
            tracing::debug!(
                "Successfully synced capabilities for server {} to suit {}",
                server_id,
                suit_id
            );
        }
    });
}

/// Invalidate suit cache if merge service is available
async fn invalidate_suit_cache(state: &Arc<AppState>) {
    if let Some(merge_service) = &state.suit_merge_service {
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated suit service cache to sync server connections");
    }
}

/// List servers in a configuration suit (standardized version)
///
/// **Endpoint:** `GET /mcp/suits/servers/list?suit_id={suit_id}&enabled_only={bool}`
pub async fn servers_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitComponentListReq>,
) -> Result<Json<SuitServersListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify suit exists
    let suit = crate::config::suit::get_config_suit(&db.pool, &request.suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                request.suit_id
            )));
        }
    };

    // Get servers in the suit
    let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &request.suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get suit servers: {e}")))?;

    // Convert to response format (simplified for now)
    let mut servers = Vec::new();
    for server_config in server_configs {
        // Get server details from server_config table
        if let Ok(Some(server)) = crate::config::server::get_server_by_id(&db.pool, &server_config.server_id).await {
            servers.push(SuitServerResp {
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
    let response = SuitServersListData {
        suit_id: request.suit_id,
        suit_name: suit.name,
        servers,
        total,
    };

    Ok(Json(SuitServersListResp::success(response)))
}

/// Manage server operations (enable/disable) in configuration suits
///
/// **Endpoint:** `POST /mcp/suits/servers/manage`
pub async fn server_manage(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuitComponentManageReq>,
) -> Result<Json<SuitServerManageResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify suit exists
    let _suit = get_suit_or_error(&db, &request.suit_id).await?;

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
    let (result, status) = match request.action {
        SuitComponentAction::Enable => {
            // Add server to suit (this enables it)
            crate::config::suit::add_server_to_config_suit(&db.pool, &request.suit_id, component_id, true)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to enable server: {e}")))?;

            // Sync server capabilities asynchronously
            spawn_capability_sync(db.pool.clone(), request.suit_id.clone(), component_id.clone());

            ("enabled", "active")
        }
        SuitComponentAction::Disable => {
            // Disable server in suit
            crate::config::suit::add_server_to_config_suit(&db.pool, &request.suit_id, component_id, false)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to disable server: {e}")))?;

            ("disabled", "inactive")
        }
    };

    // Invalidate cache
    invalidate_suit_cache(&state).await;

    let response = SuitServerManageData {
        suit_id: request.suit_id,
        results: vec![crate::api::models::suits::ComponentOperationResult {
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

    Ok(Json(SuitServerManageResp::success(response)))
}
