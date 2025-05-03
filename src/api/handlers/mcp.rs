// MCPMan Proxy API handlers for MCP server management
// Contains handler functions for MCP server endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    api::{
        models::mcp::{
            InstanceHealthResponse, OperationResponse, ServerInstanceResponse,
            ServerInstanceSummary, ServerInstancesResponse, ServerListResponse, ServerResponse,
        },
        routes::AppState,
    },
    core::types::{ConnectionStatus, ErrorType},
};

use super::ApiError;

/// List all MCP servers
pub async fn list_servers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServerListResponse>, ApiError> {
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

    let instances_map = pool.get_all_server_instances();

    let servers = instances_map
        .into_iter()
        .map(|(name, instances)| {
            // Check if server is enabled in configuration
            let enabled = pool.rule_config.get(&name).copied().unwrap_or(false);

            // Create instance summaries
            let instances = instances
                .into_iter()
                .map(|(id, conn)| {
                    // Format connected time if available
                    let connected_at = if conn.is_connected() {
                        Some(
                            chrono::DateTime::<chrono::Utc>::from(
                                std::time::SystemTime::now() - conn.time_since_last_connection(),
                            )
                            .to_rfc3339(),
                        )
                    } else {
                        None
                    };

                    // Format started time
                    let started_at = Some(
                        chrono::DateTime::<chrono::Utc>::from(
                            std::time::SystemTime::now() - conn.time_since_creation(),
                        )
                        .to_rfc3339(),
                    );

                    ServerInstanceSummary {
                        id,
                        status: conn.status_string(),
                        started_at,
                        connected_at,
                    }
                })
                .collect();

            // Create server response
            ServerResponse {
                name,
                enabled,
                instances,
            }
        })
        .collect();

    Ok(Json(ServerListResponse { servers }))
}

/// Get a specific MCP server
pub async fn get_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
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

    // Check if the server exists
    if !pool.connections.contains_key(&name) {
        return Err(ApiError::NotFound(format!("Server '{}' not found", name)));
    }

    // Get all instances for this server
    let instances = pool.connections.get(&name).unwrap();

    // Check if server is enabled in configuration
    let enabled = pool.rule_config.get(&name).copied().unwrap_or(false);

    // Create instance summaries
    let instances = instances
        .iter()
        .map(|(id, conn)| {
            // Format connected time if available
            let connected_at = if conn.is_connected() {
                Some(
                    chrono::DateTime::<chrono::Utc>::from(
                        std::time::SystemTime::now() - conn.time_since_last_connection(),
                    )
                    .to_rfc3339(),
                )
            } else {
                None
            };

            ServerInstanceSummary {
                id: id.clone(),
                status: conn.status_string(),
                started_at: Some(
                    chrono::DateTime::<chrono::Utc>::from(
                        std::time::SystemTime::now() - conn.time_since_creation(),
                    )
                    .to_rfc3339(),
                ),
                connected_at,
            }
        })
        .collect();

    Ok(Json(ServerResponse {
        name,
        enabled,
        instances,
    }))
}

/// List all instances for a specific MCP server
pub async fn list_instances(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerInstancesResponse>, ApiError> {
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

    // Check if the server exists
    if !pool.connections.contains_key(&name) {
        return Err(ApiError::NotFound(format!("Server '{}' not found", name)));
    }

    // Get all instances for this server
    let instances = pool.connections.get(&name).unwrap();

    // Create instance summary list
    let instance_summaries = instances
        .iter()
        .map(|(id, conn)| {
            // Format connected time if available
            let connected_at = if conn.is_connected() {
                Some(
                    chrono::DateTime::<chrono::Utc>::from(
                        std::time::SystemTime::now() - conn.time_since_last_connection(),
                    )
                    .to_rfc3339(),
                )
            } else {
                None
            };

            crate::api::models::mcp::ServerInstanceSummary {
                id: id.clone(),
                status: conn.status_string(),
                started_at: Some(
                    chrono::DateTime::<chrono::Utc>::from(
                        std::time::SystemTime::now() - conn.time_since_creation(),
                    )
                    .to_rfc3339(),
                ),
                connected_at,
            }
        })
        .collect();

    Ok(Json(ServerInstancesResponse {
        name,
        instances: instance_summaries,
    }))
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
    let details = crate::api::models::mcp::ServerInstanceDetails {
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
        server_type: pool.get_server_type(&name).unwrap_or_default(),
        process_id: conn.process_id,
        cpu_usage: conn.cpu_usage,
        memory_usage: conn.memory_usage,
        last_health_check: Some(chrono::Local::now().to_rfc3339()),
    };

    // Get allowed operations
    let allowed_operations = conn.allowed_operations();

    Ok(Json(ServerInstanceResponse {
        id,
        name,
        status: conn.status_string(),
        allowed_operations,
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

    Ok(Json(InstanceHealthResponse {
        id,
        name,
        healthy,
        message,
        status: conn.status_string(),
        checked_at,
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
                result: format!("Successfully disconnected instance"),
                status: conn.status_string(),
                allowed_operations: conn.allowed_operations(),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to disconnect instance: {}",
            e
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
                result: format!("Successfully force disconnected instance"),
                status: conn.status_string(),
                allowed_operations: conn.allowed_operations(),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to force disconnect instance: {}",
            e
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
                result: format!("Successfully reconnected instance"),
                status: conn.status_string(),
                allowed_operations: conn.allowed_operations(),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to reconnect instance: {}",
            e
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
                result: format!("Successfully reset and reconnected instance"),
                status: conn.status_string(),
                allowed_operations: conn.allowed_operations(),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to reset and reconnect instance: {}",
            e
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
                result: format!("Successfully cancelled instance initialization"),
                status: conn.status_string(),
                allowed_operations: conn.allowed_operations(),
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to cancel instance initialization: {}",
            e
        ))),
    }
}
