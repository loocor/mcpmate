// MCP Proxy API handlers for system management
// Contains handler functions for system endpoints

use axum::{extract::State, Json};
use std::sync::Arc;

use crate::api::{
    models::system::{MetricsResponse, StatusResponse},
    routes::AppState,
};

use super::ApiError;

/// Get system status
pub async fn get_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatusResponse>, ApiError> {
    let pool = state.connection_pool.lock().await;
    let statuses = pool.get_all_server_statuses();

    let total_servers = statuses.len();
    let connected_servers = statuses
        .values()
        .filter(|status| status == &"Connected")
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
    State(state): State<Arc<AppState>>,
) -> Result<Json<MetricsResponse>, ApiError> {
    let pool = state.connection_pool.lock().await;
    let statuses = pool.get_all_server_statuses();

    let server_statuses = statuses
        .into_iter()
        .map(|(name, status)| (name, status))
        .collect();

    Ok(Json(MetricsResponse {
        uptime: get_uptime_seconds(),
        server_statuses,
    }))
}

/// Get system uptime in seconds
fn get_uptime_seconds() -> u64 {
    // In a real implementation, this would track the actual server start time
    // For now, we'll just return 0 as a placeholder
    0
}
