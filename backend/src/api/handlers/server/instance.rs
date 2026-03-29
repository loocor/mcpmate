// MCPMate Proxy API handlers for MCP server management
// Contains handler functions for MCP server endpoints

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::api::handlers::ApiError;
use crate::audit::{AuditAction, AuditStatus};
use crate::{
    api::{
        models::server::{
            InstanceAction, InstanceDetailsData, InstanceDetailsReq, InstanceDetailsResp, InstanceHealthData,
            InstanceHealthReq, InstanceHealthResp, InstanceManageReq, ServerOperationData,
        },
        routes::AppState,
    },
    common::server::ServerType,
    core::{
        foundation::types::{ConnectionStatus, ErrorType},
        pool::UpstreamConnection,
    },
};
use serde_json::{Map, Value};

/// Get the allowed operations for a connection
fn get_allowed_operations(conn: &UpstreamConnection) -> Vec<String> {
    conn.allowed_typed_operations()
        .into_iter()
        .map(|op| op.to_string())
        .collect()
}

/// Get a specific instance for a specific MCP server (updated for query parameters)
///
/// **Endpoint:** `GET /mcp/servers/instances/details?server={server_id}&instance={instance_id}`
pub async fn get_instance(
    State(state): State<Arc<AppState>>,
    Query(request): Query<InstanceDetailsReq>,
) -> Result<Json<InstanceDetailsResp>, ApiError> {
    get_instance_core(State(state), request.server, request.instance).await
}

/// Core get instance logic for reuse
async fn get_instance_core(
    State(state): State<Arc<AppState>>,
    name: String,
    id: String,
) -> Result<Json<InstanceDetailsResp>, ApiError> {
    // Use the standardized connection pool manager for health checks
    let pool = crate::api::handlers::server::common::ConnectionPoolManager::get_pool_for_health_check(&state).await?;

    // Get the instance
    let conn = pool.get_instance(&name, &id)?;

    // Create instance details
    let details = crate::api::models::server::InstanceDetails {
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

    Ok(Json(InstanceDetailsResp::success(InstanceDetailsData {
        id,
        name,
        status: conn.status_string(),
        allowed_operations: get_allowed_operations(conn),
        details,
    })))
}

/// Check the health of a specific instance (updated for query parameters)
///
/// **Endpoint:** `GET /mcp/servers/instances/health?server={server_id}&instance={instance_id}`
pub async fn check_health(
    State(state): State<Arc<AppState>>,
    Query(request): Query<InstanceHealthReq>,
) -> Result<Json<InstanceHealthResp>, ApiError> {
    check_health_core(State(state), request.server, request.instance).await
}

/// Core check health logic for reuse
async fn check_health_core(
    State(state): State<Arc<AppState>>,
    name: String,
    id: String,
) -> Result<Json<InstanceHealthResp>, ApiError> {
    // Use the standardized connection pool manager for health checks
    let pool = crate::api::handlers::server::common::ConnectionPoolManager::get_pool_for_health_check(&state).await?;

    // Get the instance
    let conn = pool.get_instance(&name, &id)?;

    // Determine if the instance is healthy
    let healthy = matches!(conn.status, ConnectionStatus::Ready);

    // Create message based on health status
    let message = match conn.status {
        ConnectionStatus::Idle => "Instance is idle (placeholder, not connected)".to_string(),
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
        ConnectionStatus::Disabled(ref details) => {
            format!(
                "Instance is disabled: {} (total failures: {})",
                details.reason, details.total_failures
            )
        }
        ConnectionStatus::Validating => "Instance is running as a temporary validation instance".to_string(),
    };

    // Get current time as ISO 8601 string
    let checked_at = chrono::Local::now().to_rfc3339();

    // Create resource metrics
    let resource_metrics = Some(crate::api::models::server::ServerResourceMetrics {
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

    Ok(Json(InstanceHealthResp::success(InstanceHealthData {
        id,
        name,
        healthy,
        message,
        status: conn.status_string(),
        checked_at,
        resource_metrics,
        connection_stability,
    })))
}

/// Unified instance management function that handles all instance operations
/// based on the action specified in the request payload
///
/// **Endpoint:** `POST /mcp/servers/instances/manage`
#[tracing::instrument(skip(state), level = "debug")]
pub async fn manage_instance(
    State(state): State<Arc<AppState>>,
    Json(request): Json<InstanceManageReq>,
) -> Result<Json<ServerOperationData>, ApiError> {
    let started_at = std::time::Instant::now();
    let name = request.server.clone();
    let id = request.instance.clone();

    let audit_action = match request.action {
        InstanceAction::Disconnect => AuditAction::ServerInstanceDisconnect,
        InstanceAction::ForceDisconnect => AuditAction::ServerInstanceForceDisconnect,
        InstanceAction::Reconnect => AuditAction::ServerInstanceReconnect,
        InstanceAction::ResetReconnect => AuditAction::ServerInstanceResetReconnect,
        InstanceAction::Recover => AuditAction::ServerInstanceRecover,
        InstanceAction::Cancel => AuditAction::ServerInstanceCancel,
    };

    let result = match request.action {
        InstanceAction::Disconnect => disconnect_core(State(state.clone()), name.clone(), id.clone()).await,
        InstanceAction::ForceDisconnect => force_disconnect_core(State(state.clone()), name.clone(), id.clone()).await,
        InstanceAction::Reconnect => reconnect_core(State(state.clone()), name.clone(), id.clone()).await,
        InstanceAction::ResetReconnect => reset_reconnect_core(State(state.clone()), name.clone(), id.clone()).await,
        InstanceAction::Recover => recover_instance_core(State(state.clone()), name.clone(), id.clone()).await,
        InstanceAction::Cancel => cancel_core(State(state.clone()), name.clone(), id.clone()).await,
    };

    let (audit_status, audit_error) = match &result {
        Ok(_response) => (AuditStatus::Success, None),
        Err(e) => (AuditStatus::Failed, Some(e.to_string())),
    };

    let mut data = Map::new();
    data.insert("server_name".to_string(), Value::String(name.clone()));
    data.insert("instance_id".to_string(), Value::String(id.clone()));
    data.insert("action".to_string(), Value::String(request.action.to_string()));

    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            audit_action,
            audit_status,
            "POST",
            "/api/mcp/servers/instances/manage",
            Some(started_at.elapsed().as_millis() as u64),
            Some(name),
            None,
            Some(data),
            audit_error,
        ),
    )
    .await;

    result
}

/// Disconnect an instance (legacy function for backwards compatibility)
pub async fn disconnect(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ServerOperationData>, ApiError> {
    disconnect_core(State(state), name, id).await
}

/// Core disconnect logic for reuse
async fn disconnect_core(
    State(state): State<Arc<AppState>>,
    name: String,
    id: String,
) -> Result<Json<ServerOperationData>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Use regular disconnect operation
    let operation = "disconnect";

    // Perform the operation
    match pool.perform_instance_operation(&name, &id, operation).await {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(ServerOperationData {
                id,
                name,
                result: "Successfully disconnected instance".to_string(),
                status: conn.status_string(),
                allowed_operations: get_allowed_operations(conn),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!("Failed to disconnect instance: {e}"))),
    }
}

/// Force disconnect an instance (legacy function for backwards compatibility)
pub async fn force_disconnect(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ServerOperationData>, ApiError> {
    force_disconnect_core(State(state), name, id).await
}

/// Core force disconnect logic for reuse
async fn force_disconnect_core(
    State(state): State<Arc<AppState>>,
    name: String,
    id: String,
) -> Result<Json<ServerOperationData>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Perform the operation
    match pool.perform_instance_operation(&name, &id, "force_disconnect").await {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(ServerOperationData {
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

/// Reconnect an instance (legacy function for backwards compatibility)
pub async fn reconnect(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ServerOperationData>, ApiError> {
    reconnect_core(State(state), name, id).await
}

/// Core reconnect logic for reuse
async fn reconnect_core(
    State(state): State<Arc<AppState>>,
    name: String,
    id: String,
) -> Result<Json<ServerOperationData>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Use regular reconnect operation
    let operation = "reconnect";

    // Perform the operation
    match pool.perform_instance_operation(&name, &id, operation).await {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(ServerOperationData {
                id,
                name,
                result: "Successfully reconnected instance".to_string(),
                status: conn.status_string(),
                allowed_operations: get_allowed_operations(conn),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!("Failed to reconnect instance: {e}"))),
    }
}

/// Reset and reconnect an instance (legacy function for backwards compatibility)
pub async fn reset_reconnect(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ServerOperationData>, ApiError> {
    reset_reconnect_core(State(state), name, id).await
}

/// Core reset reconnect logic for reuse
async fn reset_reconnect_core(
    State(state): State<Arc<AppState>>,
    name: String,
    id: String,
) -> Result<Json<ServerOperationData>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Perform the operation
    match pool.perform_instance_operation(&name, &id, "reset_reconnect").await {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(ServerOperationData {
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

/// Manually recover a disabled instance (legacy function for backwards compatibility)
pub async fn recover_instance(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ServerOperationData>, ApiError> {
    recover_instance_core(State(state), name, id).await
}

/// Core recover instance logic for reuse
async fn recover_instance_core(
    State(state): State<Arc<AppState>>,
    name: String,
    id: String,
) -> Result<Json<ServerOperationData>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Get the instance
    let conn = pool.get_instance_mut(&name, &id)?;

    // Check if the instance is disabled
    if !conn.is_disabled() {
        return Err(ApiError::BadRequest(format!(
            "Instance '{}' of server '{}' is not disabled (current status: {})",
            id, name, conn.status
        )));
    }

    // Manually recover the instance
    match conn.manual_re_enable() {
        Ok(_) => {
            tracing::info!(
                "Manually recovered disabled instance '{}' of server '{}' via API",
                id,
                name
            );

            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(ServerOperationData {
                id,
                name,
                result: "Successfully recovered disabled instance".to_string(),
                status: conn.status_string(),
                allowed_operations: get_allowed_operations(conn),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!("Failed to recover instance: {e}"))),
    }
}

/// Cancel an initializing instance (legacy function for backwards compatibility)
pub async fn cancel(
    State(state): State<Arc<AppState>>,
    Path((name, id)): Path<(String, String)>,
) -> Result<Json<ServerOperationData>, ApiError> {
    cancel_core(State(state), name, id).await
}

/// Core cancel logic for reuse
async fn cancel_core(
    State(state): State<Arc<AppState>>,
    name: String,
    id: String,
) -> Result<Json<ServerOperationData>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    // Perform the operation
    match pool.perform_instance_operation(&name, &id, "cancel").await {
        Ok(_) => {
            // Get the updated instance
            let conn = pool.get_instance(&name, &id)?;

            Ok(Json(ServerOperationData {
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
