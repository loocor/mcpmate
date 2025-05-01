// MCP Proxy API handlers for MCP server management
// Contains handler functions for MCP server endpoints

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::api::{
    models::mcp::{ServerListResponse, ServerResponse, ServerStatusResponse},
    routes::AppState,
};

use super::ApiError;

/// List all MCP servers
pub async fn list_servers(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ServerListResponse>, ApiError> {
    let pool = state.connection_pool.lock().await;
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
    let pool = state.connection_pool.lock().await;

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

    match pool.connect(&name).await {
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
