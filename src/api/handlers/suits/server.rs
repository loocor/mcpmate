// MCPMate Proxy API handlers for Config Suit server management
// Contains handler functions for managing servers in Config Suits

use std::collections::HashMap;

use super::{common::*, get_server_or_error, get_suit_or_error};
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

/// Find server configuration status in the suit configurations
fn find_server_config_status(
    server_configs: &[ConfigSuitServer],
    server_id: &str,
) -> Option<bool> {
    server_configs
        .iter()
        .find(|config| config.server_id == server_id)
        .map(|config| config.enabled)
}

/// Invalidate suit cache if merge service is available
async fn invalidate_suit_cache(state: &Arc<AppState>) {
    if let Some(merge_service) = &state.suit_merge_service {
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated suit service cache to sync server connections");
    }
}

/// List servers in a configuration suit
pub async fn list_servers(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigSuitServersResponse>, ApiError> {
    let db = get_database(&state).await?;
    let suit = get_suit_or_error(&db, &id).await?;

    // Get all available servers in the system
    let all_servers = crate::config::server::get_all_servers(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers: {e}")))?;

    // Get servers currently configured in this suit
    let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server configurations: {e}")))?;

    // Create a map of server ID to enabled status in this suit
    let suit_server_status: HashMap<String, bool> = server_configs
        .into_iter()
        .map(|config| (config.server_id, config.enabled))
        .collect();

    // Convert to response format using functional approach
    let servers = all_servers
        .into_iter()
        .filter_map(|server| {
            server.id.as_ref().map(|server_id| {
                let enabled = suit_server_status.get(server_id).copied().unwrap_or(false);
                ResponseConverter::server_to_suit_response(&server, enabled)
            })
        })
        .collect::<Vec<_>>();

    tracing::debug!(
        "Listed {} total servers for configuration suit '{}' ({})",
        servers.len(),
        suit.name,
        id
    );

    Ok(Json(ConfigSuitServersResponse {
        suit_id: id,
        suit_name: suit.name,
        servers,
    }))
}

/// Enable a server in a configuration suit
pub async fn enable_server(
    State(state): State<Arc<AppState>>,
    Path((suit_id, server_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    let db = get_database(&state).await?;
    let server = get_server_or_error(&db, &server_id).await?;

    // Check if the server is already enabled in the suit
    let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server configurations: {e}")))?;

    // Early return if server is already enabled
    if let Some(true) = find_server_config_status(&server_configs, &server_id) {
        return Ok(Json(SuitOperationResponse {
            id: server_id,
            name: server.name,
            result: "Server is already enabled in this configuration suit".to_string(),
            status: "Enabled".to_string(),
            allowed_operations: vec!["disable".to_string()],
        }));
    }

    // Enable the server in the suit
    crate::config::suit::add_server_to_config_suit(&db.pool, &suit_id, &server_id, true)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to enable server in configuration suit: {e}")))?;

    // Sync server capabilities asynchronously
    spawn_capability_sync(db.pool.clone(), suit_id.clone(), server_id.clone());

    // Invalidate cache
    invalidate_suit_cache(&state).await;

    Ok(Json(SuitOperationResponse {
        id: server_id,
        name: server.name,
        result: "Successfully enabled server in configuration suit".to_string(),
        status: "Enabled".to_string(),
        allowed_operations: vec!["disable".to_string()],
    }))
}

/// Disable a server in a configuration suit
pub async fn disable_server(
    State(state): State<Arc<AppState>>,
    Path((suit_id, server_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    let db = get_database(&state).await?;
    let server = get_server_or_error(&db, &server_id).await?;

    // Check if the server is already disabled in the suit
    let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server configurations: {e}")))?;

    // Early return if server is already disabled
    if let Some(false) = find_server_config_status(&server_configs, &server_id) {
        return Ok(Json(SuitOperationResponse {
            id: server_id,
            name: server.name,
            result: "Server is already disabled in this configuration suit".to_string(),
            status: "Disabled".to_string(),
            allowed_operations: vec!["enable".to_string()],
        }));
    }

    // Disable the server in the suit
    crate::config::suit::add_server_to_config_suit(&db.pool, &suit_id, &server_id, false)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to disable server in configuration suit: {e}")))?;

    // Invalidate cache
    invalidate_suit_cache(&state).await;

    Ok(Json(SuitOperationResponse {
        id: server_id,
        name: server.name,
        result: "Successfully disabled server in configuration suit".to_string(),
        status: "Disabled".to_string(),
        allowed_operations: vec!["enable".to_string()],
    }))
}

/// Process a single server for batch enable operation
async fn process_server_enable(
    pool: &Pool<Sqlite>,
    suit_id: &str,
    server_id: String,
) -> Result<String, String> {
    // Check if server exists
    let server = crate::config::server::get_server_by_id(pool, &server_id)
        .await
        .map_err(|e| format!("Failed to get server: {e}"))?;

    if server.is_none() {
        return Err("Server not found".to_string());
    }

    // Enable the server in the suit
    crate::config::suit::add_server_to_config_suit(pool, suit_id, &server_id, true)
        .await
        .map_err(|e| format!("Failed to enable server: {e}"))?;

    Ok(server_id)
}

/// Batch enable servers in a configuration suit
pub async fn batch_enable_servers(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    let db = get_database(&state).await?;
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    // Process all servers using functional approach with parallel processing
    let results = futures::future::join_all(
        payload
            .ids
            .into_iter()
            .map(|server_id| process_server_enable(&db.pool, &suit_id, server_id)),
    )
    .await;

    // Partition results into successful and failed
    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    for result in results {
        match result {
            Ok(server_id) => {
                // Sync capabilities asynchronously for successful servers
                spawn_capability_sync(db.pool.clone(), suit_id.clone(), server_id.clone());
                successful_ids.push(server_id);
            }
            Err(e) => {
                // Extract server_id from error context if possible, otherwise use a placeholder
                let server_id = e.split(':').next().unwrap_or("unknown").to_string();
                failed_ids.insert(server_id, e);
            }
        }
    }

    // Invalidate cache if any servers were successfully enabled
    if !successful_ids.is_empty() {
        invalidate_suit_cache(&state).await;
    }

    Ok(Json(BatchOperationResponse {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}

/// Process a single server for batch disable operation
async fn process_server_disable(
    pool: &Pool<Sqlite>,
    suit_id: &str,
    server_id: String,
) -> Result<String, String> {
    // Check if server exists
    let server = crate::config::server::get_server_by_id(pool, &server_id)
        .await
        .map_err(|e| format!("Failed to get server: {e}"))?;

    if server.is_none() {
        return Err("Server not found".to_string());
    }

    // Disable the server in the suit
    crate::config::suit::add_server_to_config_suit(pool, suit_id, &server_id, false)
        .await
        .map_err(|e| format!("Failed to disable server: {e}"))?;

    Ok(server_id)
}

/// Batch disable servers in a configuration suit
pub async fn batch_disable_servers(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    let db = get_database(&state).await?;
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    // Process all servers using functional approach with parallel processing
    let results = futures::future::join_all(
        payload
            .ids
            .into_iter()
            .map(|server_id| process_server_disable(&db.pool, &suit_id, server_id)),
    )
    .await;

    // Partition results into successful and failed
    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    for result in results {
        match result {
            Ok(server_id) => successful_ids.push(server_id),
            Err(e) => {
                // Extract server_id from error context if possible, otherwise use a placeholder
                let server_id = e.split(':').next().unwrap_or("unknown").to_string();
                failed_ids.insert(server_id, e);
            }
        }
    }

    // Invalidate cache if any servers were successfully disabled
    if !successful_ids.is_empty() {
        invalidate_suit_cache(&state).await;
    }

    Ok(Json(BatchOperationResponse {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}
