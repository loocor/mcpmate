// MCP Proxy API handlers for MCP server management
// Contains handler functions for MCP server endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::{
    api::{
        models::mcp::{
            InstanceDetails, InstanceStatus, ServerDetailsResponse, ServerHealthResponse,
            ServerListResponse, ServerResponse, ServerStatusResponse,
        },
        routes::AppState,
    },
    proxy::types::ConnectionStatus,
};

use super::ApiError;

/// Helper function to get instance statuses for a server
fn get_instance_statuses(
    pool: &crate::proxy::pool::UpstreamConnectionPool,
    server_name: &str,
) -> Result<(String, Vec<InstanceStatus>), ApiError> {
    // Check if the server exists
    if !pool.connections.contains_key(server_name) {
        return Err(ApiError::NotFound(format!(
            "Server '{}' not found",
            server_name
        )));
    }

    // Get all instances for this server
    let instances = pool.connections.get(server_name).unwrap();

    // Create a summary status
    let status = if instances.is_empty() {
        "No instances".to_string()
    } else {
        let instance_count = instances.len();
        let ready_count = instances
            .iter()
            .filter(|(_, conn)| matches!(conn.status, ConnectionStatus::Ready))
            .count();

        format!("{}/{} instances ready", ready_count, instance_count)
    };

    // Create instance status list
    let instance_statuses = instances
        .iter()
        .map(|(id, conn)| InstanceStatus {
            id: id.clone(),
            status: conn.status_string(),
        })
        .collect();

    Ok((status, instance_statuses))
}

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

    let statuses = pool.get_all_server_statuses();

    let servers = statuses
        .into_iter()
        .map(|(name, instances)| {
            // Create a summary status string
            let status = if instances.is_empty() {
                "No instances".to_string()
            } else {
                let instance_count = instances.len();
                let ready_count = instances
                    .iter()
                    .filter(|(_, status)| status.contains("Ready"))
                    .count();

                format!("{}/{} instances ready", ready_count, instance_count)
            };

            // Create instance status list
            let instance_statuses = instances
                .iter()
                .map(|(id, status)| InstanceStatus {
                    id: id.clone(),
                    status: status.clone(),
                })
                .collect();

            ServerStatusResponse {
                name,
                status,
                instances: instance_statuses,
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

    // Get instance statuses
    let (status, instances) = get_instance_statuses(&pool, &name)?;

    Ok(Json(ServerResponse {
        name,
        status,
        instances,
    }))
}

/// Enable a server
pub async fn enable_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    match pool.enable_server(&name).await {
        Ok(_) => {
            // Get instance statuses
            let (status, instances) = get_instance_statuses(&pool, &name)?;

            Ok(Json(ServerResponse {
                name,
                status,
                instances,
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to enable server '{}': {}",
            name, e
        ))),
    }
}

/// Disable a server
pub async fn disable_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    match pool.disable_server(&name).await {
        Ok(_) => {
            // Get instance statuses
            let (status, instances) = get_instance_statuses(&pool, &name)?;

            Ok(Json(ServerResponse {
                name,
                status,
                instances,
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to disable server '{}': {}",
            name, e
        ))),
    }
}

/// Pause a server
pub async fn pause_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    match pool.pause_server(&name).await {
        Ok(_) => {
            // Get instance statuses
            let (status, instances) = get_instance_statuses(&pool, &name)?;

            Ok(Json(ServerResponse {
                name,
                status,
                instances,
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to pause server '{}': {}",
            name, e
        ))),
    }
}

/// Connect to a server
pub async fn connect_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    match pool.trigger_connect_default(&name).await {
        Ok(_) => {
            // Get instance statuses
            let (status, instances) = get_instance_statuses(&pool, &name)?;

            Ok(Json(ServerResponse {
                name,
                status,
                instances,
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to connect to server '{}': {}",
            name, e
        ))),
    }
}

/// Disconnect from a server
pub async fn disconnect_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    match pool.disconnect_default(&name).await {
        Ok(_) => {
            // Get instance statuses
            let (status, instances) = get_instance_statuses(&pool, &name)?;

            Ok(Json(ServerResponse {
                name,
                status,
                instances,
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to disconnect from server '{}': {}",
            name, e
        ))),
    }
}

/// Reconnect to a server
pub async fn reconnect_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    match pool.reconnect_default(&name).await {
        Ok(_) => {
            // Get instance statuses
            let (status, instances) = get_instance_statuses(&pool, &name)?;

            Ok(Json(ServerResponse {
                name,
                status,
                instances,
            }))
        }
        Err(e) => Err(ApiError::BadRequest(format!(
            "Failed to reconnect to server '{}': {}",
            name, e
        ))),
    }
}

/// Get detailed information about a server
pub async fn get_server_details(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerDetailsResponse>, ApiError> {
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

    // Get server configuration
    let server_config = pool.config.mcp_servers.get(&name).ok_or_else(|| {
        ApiError::NotFound(format!("Server configuration for '{}' not found", name))
    })?;

    // Get all instances for this server
    let instances = pool.connections.get(&name).unwrap();

    // Create a summary status
    let status = if instances.is_empty() {
        "No instances".to_string()
    } else {
        let instance_count = instances.len();
        let ready_count = instances
            .iter()
            .filter(|(_, conn)| matches!(conn.status, ConnectionStatus::Ready))
            .count();

        format!("{}/{} instances ready", ready_count, instance_count)
    };

    // Create instance details list
    let instance_details = instances
        .iter()
        .map(|(id, conn)| {
            // Get error message if status is Error
            let error_message = match &conn.status {
                ConnectionStatus::Error(msg) => Some(msg.clone()),
                _ => None,
            };

            // Calculate time since last connection
            let last_connected_seconds = if conn.is_connected() {
                let now = std::time::Instant::now();
                if now > conn.last_connected {
                    Some(now.duration_since(conn.last_connected).as_secs())
                } else {
                    Some(0)
                }
            } else {
                None
            };

            InstanceDetails {
                id: id.clone(),
                status: conn.status_string(),
                connection_attempts: conn.connection_attempts,
                last_connected_seconds,
                tools_count: conn.tools.len(),
                error_message,
            }
        })
        .collect();

    // Check if server is enabled in configuration
    let is_enabled = pool.rule_config.get(&name).copied().unwrap_or(false);

    Ok(Json(ServerDetailsResponse {
        name: name.clone(),
        status,
        server_type: server_config.kind.clone(),
        is_enabled,
        instances: instance_details,
    }))
}

/// Check the health of a server
pub async fn check_server_health(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerHealthResponse>, ApiError> {
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

    // Create a summary status
    let status = if instances.is_empty() {
        "No instances".to_string()
    } else {
        let instance_count = instances.len();
        let ready_count = instances
            .iter()
            .filter(|(_, conn)| matches!(conn.status, ConnectionStatus::Ready))
            .count();

        format!("{}/{} instances ready", ready_count, instance_count)
    };

    // Determine if the server is healthy (at least one instance is ready)
    let healthy = instances
        .iter()
        .any(|(_, conn)| matches!(conn.status, ConnectionStatus::Ready));

    // Create message based on health status
    let message = if healthy {
        "At least one instance is ready and healthy".to_string()
    } else if instances.is_empty() {
        "No instances available".to_string()
    } else {
        "No ready instances available".to_string()
    };

    // Create instance status list
    let instance_statuses = instances
        .iter()
        .map(|(id, conn)| InstanceStatus {
            id: id.clone(),
            status: conn.status_string(),
        })
        .collect();

    // Get current time as ISO 8601 string
    let checked_at = chrono::Utc::now().to_rfc3339();

    Ok(Json(ServerHealthResponse {
        name: name.clone(),
        healthy,
        message,
        status,
        checked_at,
        instances: instance_statuses,
    }))
}
