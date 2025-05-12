// MCPMate Proxy API handlers for MCP server instance operations
// Contains handler functions for listing server instances

use super::common::*;

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
