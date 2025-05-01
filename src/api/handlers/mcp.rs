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
            ServerDetailsResponse, ServerHealthResponse, ServerListResponse, ServerResponse,
            ServerStatusResponse,
        },
        routes::AppState,
    },
    proxy::types::ConnectionStatus,
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

    let statuses = pool.get_all_server_statuses();

    let servers = statuses
        .into_iter()
        .map(|(name, status)| ServerStatusResponse { name, status })
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

    match pool.get_server_status(&name) {
        Ok(status) => Ok(Json(ServerResponse { name, status })),
        Err(_) => Err(ApiError::NotFound(format!("Server '{}' not found", name))),
    }
}

/// Enable a server
pub async fn enable_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    let mut pool = state.connection_pool.lock().await;

    match pool.enable_server(&name).await {
        Ok(_) => {
            let status = pool.get_server_status(&name).map_err(|_| {
                ApiError::InternalError(format!("Failed to get status for server '{}'", name))
            })?;

            Ok(Json(ServerResponse { name, status }))
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
            let status = pool.get_server_status(&name).map_err(|_| {
                ApiError::InternalError(format!("Failed to get status for server '{}'", name))
            })?;

            Ok(Json(ServerResponse { name, status }))
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
            let status = pool.get_server_status(&name).map_err(|_| {
                ApiError::InternalError(format!("Failed to get status for server '{}'", name))
            })?;

            Ok(Json(ServerResponse { name, status }))
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

    match pool.trigger_connect(&name).await {
        Ok(_) => {
            let status = pool.get_server_status(&name).map_err(|_| {
                ApiError::InternalError(format!("Failed to get status for server '{}'", name))
            })?;

            Ok(Json(ServerResponse { name, status }))
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

    match pool.disconnect(&name).await {
        Ok(_) => {
            let status = pool.get_server_status(&name).map_err(|_| {
                ApiError::InternalError(format!("Failed to get status for server '{}'", name))
            })?;

            Ok(Json(ServerResponse { name, status }))
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

    match pool.reconnect(&name).await {
        Ok(_) => {
            let status = pool.get_server_status(&name).map_err(|_| {
                ApiError::InternalError(format!("Failed to get status for server '{}'", name))
            })?;

            Ok(Json(ServerResponse { name, status }))
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

    // Get the connection for this server
    let conn = pool
        .connections
        .get(&name)
        .ok_or_else(|| ApiError::NotFound(format!("Server '{}' not found", name)))?;

    // Get server configuration
    let server_config = pool.config.mcp_servers.get(&name).ok_or_else(|| {
        ApiError::NotFound(format!("Server configuration for '{}' not found", name))
    })?;

    // Get error message if status is Failed
    let error_message = match &conn.status {
        ConnectionStatus::Failed(msg) => Some(msg.clone()),
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

    // Check if server is enabled in configuration
    let is_enabled = pool.rule_config.get(&name).copied().unwrap_or(false);

    Ok(Json(ServerDetailsResponse {
        name: name.clone(),
        status: conn.status_string(),
        connection_attempts: conn.connection_attempts,
        last_connected_seconds,
        tools_count: conn.tools.len(),
        error_message,
        server_type: server_config.kind.clone(),
        is_enabled,
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

    // Get the connection for this server
    let conn = pool
        .connections
        .get(&name)
        .ok_or_else(|| ApiError::NotFound(format!("Server '{}' not found", name)))?;

    // Determine if the server is healthy
    let (healthy, message) = match conn.status {
        ConnectionStatus::Connected => (true, "Server is connected and healthy".to_string()),
        ConnectionStatus::Connecting => (false, "Server is currently connecting".to_string()),
        ConnectionStatus::Disconnected => (false, "Server is disconnected".to_string()),
        ConnectionStatus::Failed(ref msg) => (false, format!("Server connection failed: {}", msg)),
        ConnectionStatus::Disabled => (false, "Server is disabled".to_string()),
        ConnectionStatus::Paused => (false, "Server is paused".to_string()),
        ConnectionStatus::Reconnecting => (false, "Server is reconnecting".to_string()),
    };

    // Get current time as ISO 8601 string
    let checked_at = chrono::Utc::now().to_rfc3339();

    Ok(Json(ServerHealthResponse {
        name: name.clone(),
        healthy,
        message,
        status: conn.status_string(),
        checked_at,
    }))
}
