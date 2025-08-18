// MCPMate Proxy API handlers for basic MCP server operations
// Contains handler functions for listing and getting servers

use super::{common, shared::*};

/// Get a specific MCP server (ID-only)
pub async fn get_server(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    // Get database reference
    let db = common::get_database_from_state(&state)?;

    // Get the server by ID
    let server = crate::config::server::get_server_by_id(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{id}' not found")))?;
    let server_id = server.id.clone().unwrap_or_default();
    let name = server.name.clone();

    // Get complete server details using unified function
    let details = common::get_complete_server_details(&db.pool, &server_id, &name, &state).await;

    // Use globally_enabled as the primary enabled status for global server API
    let enabled = details.globally_enabled;

    // Format timestamps
    let created_at = server.created_at.map(|dt| dt.to_rfc3339());
    let updated_at = server.updated_at.map(|dt| dt.to_rfc3339());

    Ok(Json(ServerResponse {
        id: server.id.clone(),
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
    }))
}

/// List all MCP servers
pub async fn list_servers(State(state): State<Arc<AppState>>) -> Result<Json<ServerListResponse>, ApiError> {
    // Get database reference
    let db = common::get_database_from_state(&state)?;

    // Get all servers from the database
    let all_servers = crate::config::server::get_all_servers(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers: {e}")))?;

    // Create server responses using unified detail fetching
    let mut servers = Vec::new();
    for server in all_servers {
        let name = server.name.clone();
        let server_id = server.id.clone().unwrap_or_default();

        // Get complete server details using unified function
        let details = common::get_complete_server_details(&db.pool, &server_id, &name, &state).await;

        // Use globally_enabled as the primary enabled status for global server API
        let enabled = details.globally_enabled;

        // Format timestamps
        let created_at = server.created_at.map(|dt| dt.to_rfc3339());
        let updated_at = server.updated_at.map(|dt| dt.to_rfc3339());

        // Create server response
        servers.push(ServerResponse {
            id: server.id.clone(),
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

    Ok(Json(ServerListResponse { servers }))
}

/// List all instances for a specific MCP server (ID-only)
pub async fn list_instances(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ServerInstancesResponse>, ApiError> {
    // Resolve server name by ID for pool access
    let db = common::get_database_from_state(&state)?;
    let server = crate::config::server::get_server_by_id(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{id}' not found")))?;
    let name = server.name;

    // Reuse shared instance summarizer
    let instance_summaries = common::get_server_instances(&state, &name).await;

    Ok(Json(ServerInstancesResponse {
        name,
        instances: instance_summaries,
    }))
}
