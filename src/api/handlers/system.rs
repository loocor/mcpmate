// MCP Proxy API handlers for system management
// Contains handler functions for system endpoints

use axum::{extract::State, Json};
use std::sync::Arc;

use crate::api::{
    models::system::{StatusResponse, SystemMetricsResponse},
    routes::AppState,
};

use super::ApiError;

/// Get system status
pub async fn get_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatusResponse>, ApiError> {
    // Use timeout to avoid blocking indefinitely
    let pool_result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        state.connection_pool.lock(),
    )
    .await;

    let instances_map = match pool_result {
        Ok(pool) => pool.get_all_server_instances(),
        Err(_) => {
            // If we can't get the lock within the timeout, return a partial response
            tracing::warn!("Timed out waiting for connection pool lock in get_status");
            return Ok(Json(StatusResponse {
                status: "running".to_string(),
                uptime: get_uptime_seconds(),
                total_servers: 0,
                connected_servers: 0,
            }));
        }
    };

    let total_servers = instances_map.len();
    let connected_servers = instances_map
        .values()
        .filter(|instances| instances.iter().any(|(_, conn)| conn.is_connected()))
        .count();

    Ok(Json(StatusResponse {
        status: "running".to_string(),
        uptime: get_uptime_seconds(),
        total_servers,
        connected_servers,
    }))
}

/// Get system metrics
pub async fn get_metrics(
    _state: State<Arc<AppState>>,
) -> Result<Json<SystemMetricsResponse>, ApiError> {
    // In a real implementation, this would collect actual system metrics
    // For now, we'll just return placeholder values
    Ok(Json(SystemMetricsResponse {
        cpu_usage: 0.0,
        memory_usage_mb: 0.0,
        requests_processed: 0,
        avg_response_time_ms: 0.0,
    }))
}

use std::sync::atomic::{AtomicU64, Ordering};

// Static variable to store the server start time
static SERVER_START_TIME: AtomicU64 = AtomicU64::new(0);

/// Initialize the server start time
/// This should be called once when the server starts
pub fn initialize_server_start_time() {
    // Only set if not already set
    if SERVER_START_TIME.load(Ordering::Relaxed) == 0 {
        // Get current time as seconds since UNIX epoch
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        SERVER_START_TIME.store(now, Ordering::Relaxed);
        tracing::info!("Server start time initialized: {}", now);
    }
}

/// Get system uptime in seconds
fn get_uptime_seconds() -> u64 {
    let start_time = SERVER_START_TIME.load(Ordering::Relaxed);

    // If start time is not initialized, return 0
    if start_time == 0 {
        return 0;
    }

    // Get current time as seconds since UNIX epoch
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Calculate uptime
    now.saturating_sub(start_time)
}
