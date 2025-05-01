// MCP Proxy API handlers for system management
// Contains handler functions for system endpoints

use axum::{extract::State, Json};
use std::{collections::HashMap, sync::Arc};

use crate::api::{
    models::system::{MetricsResponse, StatusResponse},
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

    let statuses = match pool_result {
        Ok(pool) => pool.get_all_server_statuses(),
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
    // Use timeout to avoid blocking indefinitely
    let pool_result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        state.connection_pool.lock(),
    )
    .await;

    let server_statuses = match pool_result {
        Ok(pool) => pool.get_all_server_statuses(),
        Err(_) => {
            // If we can't get the lock within the timeout, return a partial response
            tracing::warn!("Timed out waiting for connection pool lock in get_metrics");
            return Ok(Json(MetricsResponse {
                uptime: get_uptime_seconds(),
                server_statuses: HashMap::new(),
            }));
        }
    };

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
