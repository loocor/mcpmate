// MCPMate Proxy API handlers for basic MCP server operations
// Contains handler functions for listing and getting servers

use super::common::*;

/// List all MCP servers
pub async fn list_servers(
    State(state): State<Arc<AppState>>
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
            // All servers are enabled by default
            let enabled = true;

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
        return Err(ApiError::NotFound(format!("Server '{name}' not found")));
    }

    // Get all instances for this server
    let instances = pool.connections.get(&name).unwrap();

    // All servers are enabled by default
    let enabled = true;

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
