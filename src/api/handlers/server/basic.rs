// MCPMate Proxy API handlers for basic MCP server operations
// Contains handler functions for listing and getting servers

use super::{common, shared::*};
use crate::api::models::server::{
    InstanceListData, InstanceListReq, InstanceListResp, ServerDetailsData, ServerDetailsReq, ServerDetailsResp,
    ServerListData, ServerListReq, ServerListResp,
};
use axum::http::StatusCode;

/// Macro to extract database pool from app state with early return on error
macro_rules! get_db_pool {
    ($app_state:expr) => {
        match &$app_state.database {
            Some(db) => db.pool.clone(),
            None => return Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    };
}

/// Get details for a specific MCP server
///
/// **Endpoint:** `GET /mcp/servers/details?id={server_id}`
pub async fn server_details(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerDetailsReq>,
) -> Result<Json<ServerDetailsResp>, StatusCode> {
    let db_pool = get_db_pool!(app_state);
    let result = server_details_core(&request, &db_pool, &app_state).await?;
    Ok(Json(result))
}

/// List all MCP servers with optional filtering
///
/// **Endpoint:** `GET /mcp/servers/list?enabled={bool}&server_type={type}&limit={limit}&offset={offset}`
pub async fn server_list(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerListReq>,
) -> Result<Json<ServerListResp>, StatusCode> {
    let db_pool = get_db_pool!(app_state);
    let result = server_list_core(&request, &db_pool, &app_state).await?;
    Ok(Json(result))
}

/// List instances for servers
///
/// **Endpoint:** `GET /mcp/servers/instances/list?id={server_id}`
pub async fn instance_list(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<InstanceListReq>,
) -> Result<Json<InstanceListResp>, StatusCode> {
    let db_pool = get_db_pool!(app_state);
    let result = instance_list_core(&request, &db_pool, &app_state).await?;
    Ok(Json(result))
}

// ==================== Core Business Functions ====================

/// Core business logic for server details operation
async fn server_details_core(
    request: &ServerDetailsReq,
    db_pool: &sqlx::SqlitePool,
    state: &Arc<AppState>,
) -> Result<ServerDetailsResp, StatusCode> {
    // Get the server by ID
    let server = crate::config::server::get_server_by_id(db_pool, &request.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get server: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let id_opt = server.id.clone();
    let server_id = id_opt.as_deref().unwrap_or_default();
    let name = server.name.clone();

    // Get complete server details using unified function
    let details = common::get_complete_server_details(db_pool, server_id, &name, state).await;
    let enabled = details.globally_enabled;
    let created_at = server.created_at.map(|dt| dt.to_rfc3339());
    let updated_at = server.updated_at.map(|dt| dt.to_rfc3339());

    let server_details = ServerDetailsData {
        id: id_opt,
        name,
        enabled,
        globally_enabled: details.globally_enabled,
        enabled_in_suits: details.enabled_in_suits,
        server_type: server.server_type,
        command: server.command.clone(),
        url: server.url.clone(),
        args: details.args,
        env: details.env,
        meta: details.meta,
        created_at,
        updated_at,
        instances: details.instances,
    };

    Ok(ServerDetailsResp::success(server_details))
}

/// Core business logic for server list operation
async fn server_list_core(
    request: &ServerListReq,
    db_pool: &sqlx::SqlitePool,
    state: &Arc<AppState>,
) -> Result<ServerListResp, StatusCode> {
    // Get all servers from the database
    let all_servers = crate::config::server::get_all_servers(db_pool).await.map_err(|e| {
        tracing::error!("Failed to get servers: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Apply filtering and pagination
    let mut filtered_servers = Vec::new();
    let mut total_count = 0;

    for server in all_servers {
        let name = server.name.clone();
        let id_opt = server.id.clone();
        let server_id = id_opt.as_deref().unwrap_or_default();

        // Get complete server details using unified function
        let details = common::get_complete_server_details(db_pool, server_id, &name, state).await;

        // Apply enabled filter if specified
        if let Some(enabled_filter) = request.enabled {
            if details.globally_enabled != enabled_filter {
                continue;
            }
        }

        // Apply server_type filter if specified
        if let Some(ref type_filter) = request.server_type {
            if server.server_type.as_str() != type_filter {
                continue;
            }
        }

        total_count += 1;

        // Apply pagination
        let offset = request.offset.unwrap_or(0) as usize;
        let limit = request.limit.unwrap_or(100) as usize;

        if total_count > offset && filtered_servers.len() < limit {
            let enabled = details.globally_enabled;
            let created_at = server.created_at.map(|dt| dt.to_rfc3339());
            let updated_at = server.updated_at.map(|dt| dt.to_rfc3339());

            filtered_servers.push(ServerDetailsData {
                id: id_opt,
                name,
                enabled,
                globally_enabled: details.globally_enabled,
                enabled_in_suits: details.enabled_in_suits,
                server_type: server.server_type,
                command: server.command.clone(),
                url: server.url.clone(),
                args: details.args,
                env: details.env,
                meta: details.meta,
                created_at,
                updated_at,
                instances: details.instances,
            });
        }
    }

    Ok(ServerListResp::success(ServerListData {
        servers: filtered_servers,
    }))
}

/// Core business logic for instance list operation
async fn instance_list_core(
    request: &InstanceListReq,
    db_pool: &sqlx::SqlitePool,
    state: &Arc<AppState>,
) -> Result<InstanceListResp, StatusCode> {
    if let Some(ref server_id) = request.id {
        // List instances for specific server
        let server = crate::config::server::get_server_by_id(db_pool, server_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get server: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

        let name = server.name;
        let instance_summaries = common::get_server_instances(state, &name).await;

        Ok(InstanceListResp::success(InstanceListData {
            name,
            instances: instance_summaries,
        }))
    } else {
        // List all instances for all servers
        let pool = match tokio::time::timeout(std::time::Duration::from_secs(1), state.connection_pool.lock()).await {
            Ok(pool) => pool,
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        };

        let mut all_instances = Vec::new();
        for (_server_name, instances) in &pool.connections {
            for (instance_id, conn) in instances {
                // Convert Instant to DateTime for serialization
                let now = std::time::SystemTime::now();
                let duration_since_created = conn.created_at.elapsed();
                let created_time = now - duration_since_created;
                let started_at = Some(chrono::DateTime::<chrono::Utc>::from(created_time).to_rfc3339());

                let connected_at = if conn.is_connected() {
                    let duration_since_connected = conn.last_connected.elapsed();
                    let connected_time = now - duration_since_connected;
                    Some(chrono::DateTime::<chrono::Utc>::from(connected_time).to_rfc3339())
                } else {
                    None
                };

                all_instances.push(crate::api::models::server::InstanceSummary {
                    id: instance_id.clone(),
                    status: conn.status_string(),
                    started_at,
                    connected_at,
                });
            }
        }

        Ok(InstanceListResp::success(InstanceListData {
            name: "all".to_string(),
            instances: all_instances,
        }))
    }
}
