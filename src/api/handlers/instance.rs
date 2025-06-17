// MCPMate Proxy API handlers for MCP server management
// Contains handler functions for MCP server endpoints

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};

use super::ApiError;
use crate::{
    api::{
        models::server::{InstanceHealthResponse, OperationResponse, ServerInstanceResponse},
        routes::AppState,
    },
    common::server::ServerType,
    core::{
        connection::UpstreamConnection,
        foundation::types::{ConnectionStatus, ErrorType},
    },
};

/// Get the allowed operations for a connection
fn get_allowed_operations(conn: &UpstreamConnection) -> Vec<String> {
    conn.allowed_typed_operations()
        .into_iter()
        .map(|op| op.to_string())
        .collect()
}

/// Get a specific instance for a specific MCP server
pub async fn get_instance(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ServerInstanceResponse>, ApiError> {
    // Use a timeout to avoid blocking indefinitely
    let pool_result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        state.connection_pool.lock(),
    )
    .await;

    let pool = match pool_result {
        Ok(pool) => pool,
        Err(_) => {
            return Err(ApiError::InternalError(
                "Timed out waiting for connection pool lock".to_string(),
            ));
        }
    };

    // Get the instance
    let conn = pool.get_instance(&name, &id)?;

    // Create instance details
    let details = crate::api::models::server::ServerInstanceDetails {
        connection_attempts: conn.connection_attempts,
        last_connected_seconds: if conn.is_connected() {
            Some(conn.time_since_last_connection().as_secs())
        } else {
            None
        },
        tools_count: conn.tools.len(),
        error_message: match &conn.status {
            ConnectionStatus::Error(err) => Some(err.message.clone()),
            _ => None,
        },
        server_type: pool
            .get_server_type(&name)
            .unwrap_or_default()
            .parse()
            .unwrap_or(ServerType::Stdio),
        process_id: conn.process_id,
        cpu_usage: conn.cpu_usage,
        memory_usage: conn.memory_usage,
        last_health_check: Some(chrono::Local::now().to_rfc3339()),
    };

    Ok(Json(ServerInstanceResponse {
        id,
        name,
        status: conn.status_string(),
        allowed_operations: get_allowed_operations(conn),
        details,
    }))
}

/// Check the health of a specific instance
pub async fn check_health(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<InstanceHealthResponse>, ApiError> {
    // Use a timeout to avoid blocking indefinitely
    let pool_result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        state.connection_pool.lock(),
    )
    .await;

    let pool = match pool_result {
        Ok(pool) => pool,
        Err(_) => {
            return Err(ApiError::InternalError(
                "Timed out waiting for connection pool lock".to_string(),
            ));
        }
    };

    // Get the instance
    let conn = pool.get_instance(&name, &id)?;

    // Determine if the instance is healthy
    let healthy = matches!(conn.status, ConnectionStatus::Ready);

    // Create message based on health status
    let message = match conn.status {
        ConnectionStatus::Ready => "Instance is ready and healthy".to_string(),
        ConnectionStatus::Busy => "Instance is busy processing a request".to_string(),
        ConnectionStatus::Initializing => "Instance is initializing".to_string(),
        ConnectionStatus::Error(ref err) => {
            let error_type = match err.error_type {
                ErrorType::Temporary => "temporary",
                ErrorType::Permanent => "permanent",
                ErrorType::Unknown => "unknown",
            };
            format!(
                "Instance has a {} error: {} (failure count: {})",
                error_type, err.message, err.failure_count
            )
        }
        ConnectionStatus::Shutdown => "Instance is shut down".to_string(),
    };

    // Get current time as ISO 8601 string
    let checked_at = chrono::Local::now().to_rfc3339();

    // Create resource metrics
    let resource_metrics = Some(crate::api::models::server::ResourceMetrics {
        cpu_usage: conn.cpu_usage,
        memory_usage: conn.memory_usage,
        process_id: conn.process_id,
    });

    // Calculate connection stability score
    let connection_stability = if let ConnectionStatus::Error(err) = &conn.status {
        // Higher failure count means lower stability
        // We use an exponential decay formula: stability = e^(-k * failure_count)
        // where k is a constant that controls how quickly stability decays
        let k = 0.2; // This can be adjusted based on desired sensitivity
        Some((-(k * err.failure_count as f32)).exp())
    } else if conn.connection_attempts == 0 {
        // If no connection attempts, we don't have enough data
        None
    } else {
        // For non-error states, base stability on connection attempts
        // More connection attempts could indicate previous issues
        let base_score = 1.0f32;
        let penalty_per_attempt = 0.05f32;
        let max_penalty = 0.5f32; // Maximum penalty from connection attempts

        let penalty = (conn.connection_attempts as f32 * penalty_per_attempt).min(max_penalty);
        Some((base_score - penalty).max(0.0))
    };

    Ok(Json(InstanceHealthResponse {
        id,
        name,
        healthy,
        message,
        status: conn.status_string(),
        checked_at,
        resource_metrics,
        connection_stability,
    }))
}

/// Disconnect an instance
pub async fn disconnect(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<OperationResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Use regular disconnect operation
    let operation = "disconnect";

    // Perform the operation
    match pool.perform_instance_operation(&name, &id, operation).await {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(OperationResponse {
                id,
                name,
                result: "Successfully disconnected instance".to_string(),
                status: conn.status_string(),
                allowed_operations: get_allowed_operations(conn),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to disconnect instance: {e}"
        ))),
    }
}

/// Force disconnect an instance
pub async fn force_disconnect(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<OperationResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Perform the operation
    match pool
        .perform_instance_operation(&name, &id, "force_disconnect")
        .await
    {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(OperationResponse {
                id,
                name,
                result: "Successfully force disconnected instance".to_string(),
                status: conn.status_string(),
                allowed_operations: get_allowed_operations(conn),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to force disconnect instance: {e}"
        ))),
    }
}

/// Reconnect an instance
pub async fn reconnect(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<OperationResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Use regular reconnect operation
    let operation = "reconnect";

    // Perform the operation
    match pool.perform_instance_operation(&name, &id, operation).await {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(OperationResponse {
                id,
                name,
                result: "Successfully reconnected instance".to_string(),
                status: conn.status_string(),
                allowed_operations: get_allowed_operations(conn),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to reconnect instance: {e}"
        ))),
    }
}

/// Reset and reconnect an instance
pub async fn reset_reconnect(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<OperationResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Perform the operation
    match pool
        .perform_instance_operation(&name, &id, "reset_reconnect")
        .await
    {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(OperationResponse {
                id,
                name,
                result: "Successfully reset and reconnected instance".to_string(),
                status: conn.status_string(),
                allowed_operations: get_allowed_operations(conn),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to reset and reconnect instance: {e}"
        ))),
    }
}

/// Cancel an initializing instance
pub async fn cancel(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<OperationResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Perform the operation
    match pool.perform_instance_operation(&name, &id, "cancel").await {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(OperationResponse {
                id,
                name,
                result: "Successfully cancelled instance initialization".to_string(),
                status: conn.status_string(),
                allowed_operations: get_allowed_operations(conn),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to cancel instance initialization: {e}"
        ))),
    }
}
