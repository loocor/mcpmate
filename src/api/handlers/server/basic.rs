// MCPMate Proxy API handlers for basic MCP server operations
// Contains handler functions for listing and getting servers

use super::common::*;
use crate::api::models::server::ServerMetaInfo;

/// List all MCP servers
pub async fn list_servers(
    State(state): State<Arc<AppState>>
) -> Result<Json<ServerListResponse>, ApiError> {
    // Get database reference
    let db = match state.http_proxy.as_ref().and_then(|p| p.database.clone()) {
        Some(db) => db,
        None => {
            return Err(ApiError::InternalError(
                "Database not available".to_string(),
            ));
        }
    };

    // Get all servers from the database
    let all_servers = crate::conf::server::get_all_servers(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers: {e}")))?;

    // Get instance information from connection pool if available
    let instances_map = if let Ok(pool) = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        state.connection_pool.lock(),
    )
    .await
    {
        pool.get_all_server_instances()
    } else {
        // If we can't get the connection pool, just use an empty map
        std::collections::HashMap::new()
    };

    // Create server responses
    let mut servers = Vec::new();
    for server in all_servers {
        let name = server.name.clone();

        // Get server ID (clone before using unwrap_or_default to avoid move)
        let server_id = match &server.id {
            Some(id) => id.clone(),
            None => String::new(),
        };

        // Get server enabled status from config suits (enabled_in_suits)
        let enabled_in_suits = match crate::conf::server::is_server_enabled_in_any_suit(
            &db.pool, &server_id,
        )
        .await
        {
            Ok(enabled) => enabled,
            Err(e) => {
                tracing::warn!(
                    "Failed to check if server '{}' is enabled in suits: {}",
                    name,
                    e
                );
                false // Default to false if there's an error
            }
        };

        // Get server global enabled status
        let globally_enabled =
            match crate::conf::server::get_server_global_status(&db.pool, &server_id)
                .await
            {
                Ok(Some(enabled)) => enabled,
                Ok(None) => {
                    tracing::warn!(
                        "Server '{}' global status not found, assuming enabled",
                        name
                    );
                    true // Default to true for backward compatibility
                }
                Err(e) => {
                    tracing::warn!("Failed to get server '{}' global status: {}", name, e);
                    true // Default to true for backward compatibility
                }
            };

        // For backward compatibility, enabled is true if the server is both globally enabled and enabled in suits
        let enabled = globally_enabled && enabled_in_suits;

        // Get instances for this server if available
        let instances = if let Some(instances) = instances_map.get(&name) {
            instances
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

                    // Format started time
                    let started_at = Some(
                        chrono::DateTime::<chrono::Utc>::from(
                            std::time::SystemTime::now() - conn.time_since_creation(),
                        )
                        .to_rfc3339(),
                    );

                    ServerInstanceSummary {
                        id: id.clone(),
                        status: conn.status_string(),
                        started_at,
                        connected_at,
                    }
                })
                .collect()
        } else {
            // No instances for this server
            Vec::new()
        };

        // Get server arguments if available
        let args = if !server_id.is_empty() {
            match crate::conf::server::get_server_args(&db.pool, &server_id).await {
                Ok(server_args) => {
                    if server_args.is_empty() {
                        None
                    } else {
                        // Sort arguments by index and collect values
                        let mut sorted_args: Vec<_> = server_args.into_iter().collect();
                        sorted_args.sort_by_key(|arg| arg.arg_index);
                        Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect())
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get arguments for server '{}': {}", name, e);
                    None
                }
            }
        } else {
            None
        };

        // Get server environment variables if available
        let env = if !server_id.is_empty() {
            match crate::conf::server::get_server_env(&db.pool, &server_id).await {
                Ok(env_map) =>
                    if env_map.is_empty() {
                        None
                    } else {
                        Some(env_map)
                    },
                Err(e) => {
                    tracing::warn!(
                        "Failed to get environment variables for server '{}': {}",
                        name,
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        // Get server metadata if available
        let meta = if !server_id.is_empty() {
            match crate::conf::server::get_server_meta(&db.pool, &server_id).await {
                Ok(Some(server_meta)) => Some(ServerMetaInfo {
                    description: server_meta.description,
                    author: server_meta.author,
                    website: server_meta.website,
                    repository: server_meta.repository,
                    category: server_meta.category,
                    recommended_scenario: server_meta.recommended_scenario,
                    rating: server_meta.rating,
                }),
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!("Failed to get metadata for server '{}': {}", name, e);
                    None
                }
            }
        } else {
            None
        };

        // Format timestamps
        let created_at = server.created_at.map(|dt| dt.to_rfc3339());
        let updated_at = server.updated_at.map(|dt| dt.to_rfc3339());

        // Create server response
        servers.push(ServerResponse {
            name,
            enabled,
            globally_enabled,
            enabled_in_suits,
            server_type: server.server_type,
            command: server.command.clone(),
            url: server.url.clone(),
            args,
            env,
            meta,
            created_at,
            updated_at,
            instances,
        });
    }

    Ok(Json(ServerListResponse { servers }))
}

/// Get a specific MCP server
pub async fn get_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    // Get database reference
    let db = match state.http_proxy.as_ref().and_then(|p| p.database.clone()) {
        Some(db) => db,
        None => {
            return Err(ApiError::InternalError(
                "Database not available".to_string(),
            ));
        }
    };

    // Get the server from the database
    let server = crate::conf::server::get_server(&db.pool, &name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?;

    // Check if the server exists
    let server = match server {
        Some(server) => server,
        None => {
            return Err(ApiError::NotFound(format!("Server '{name}' not found")));
        }
    };

    // Get server ID (clone before using unwrap_or_default to avoid move)
    let server_id = match &server.id {
        Some(id) => id.clone(),
        None => String::new(),
    };

    // Get server enabled status from config suits (enabled_in_suits)
    let enabled_in_suits =
        match crate::conf::server::is_server_enabled_in_any_suit(&db.pool, &server_id).await {
            Ok(enabled) => enabled,
            Err(e) => {
                tracing::warn!(
                    "Failed to check if server '{}' is enabled in suits: {}",
                    name,
                    e
                );
                false // Default to false if there's an error
            }
        };

    // Get server global enabled status
    let globally_enabled =
        match crate::conf::server::get_server_global_status(&db.pool, &server_id).await
        {
            Ok(Some(enabled)) => enabled,
            Ok(None) => {
                tracing::warn!(
                    "Server '{}' global status not found, assuming enabled",
                    name
                );
                true // Default to true for backward compatibility
            }
            Err(e) => {
                tracing::warn!("Failed to get server '{}' global status: {}", name, e);
                true // Default to true for backward compatibility
            }
        };

    // For backward compatibility, enabled is true if the server is both globally enabled and enabled in suits
    let enabled = globally_enabled && enabled_in_suits;

    // Get instance information from connection pool if available
    let instances = if let Ok(pool) = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        state.connection_pool.lock(),
    )
    .await
    {
        // Check if the server exists in the connection pool
        if let Some(instances) = pool.connections.get(&name) {
            // Create instance summaries
            instances
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
                .collect()
        } else {
            // No instances for this server
            Vec::new()
        }
    } else {
        // If we can't get the connection pool, just use an empty vector
        Vec::new()
    };

    // Get server ID (clone before using unwrap_or_default to avoid move)
    let server_id = match &server.id {
        Some(id) => id.clone(),
        None => String::new(),
    };

    // Get server arguments if available
    let args = if !server_id.is_empty() {
        match crate::conf::server::get_server_args(&db.pool, &server_id).await {
            Ok(server_args) => {
                if server_args.is_empty() {
                    None
                } else {
                    // Sort arguments by index and collect values
                    let mut sorted_args: Vec<_> = server_args.into_iter().collect();
                    sorted_args.sort_by_key(|arg| arg.arg_index);
                    Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect())
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get arguments for server '{}': {}", name, e);
                None
            }
        }
    } else {
        None
    };

    // Get server environment variables if available
    let env = if !server_id.is_empty() {
        match crate::conf::server::get_server_env(&db.pool, &server_id).await {
            Ok(env_map) =>
                if env_map.is_empty() {
                    None
                } else {
                    Some(env_map)
                },
            Err(e) => {
                tracing::warn!(
                    "Failed to get environment variables for server '{}': {}",
                    name,
                    e
                );
                None
            }
        }
    } else {
        None
    };

    // Get server metadata if available
    let meta = if !server_id.is_empty() {
        match crate::conf::server::get_server_meta(&db.pool, &server_id).await {
            Ok(Some(server_meta)) => Some(ServerMetaInfo {
                description: server_meta.description,
                author: server_meta.author,
                website: server_meta.website,
                repository: server_meta.repository,
                category: server_meta.category,
                recommended_scenario: server_meta.recommended_scenario,
                rating: server_meta.rating,
            }),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("Failed to get metadata for server '{}': {}", name, e);
                None
            }
        }
    } else {
        None
    };

    // Format timestamps
    let created_at = server.created_at.map(|dt| dt.to_rfc3339());
    let updated_at = server.updated_at.map(|dt| dt.to_rfc3339());

    Ok(Json(ServerResponse {
        name,
        enabled,
        globally_enabled,
        enabled_in_suits,
        server_type: server.server_type,
        command: server.command.clone(),
        url: server.url.clone(),
        args,
        env,
        meta,
        created_at,
        updated_at,
        instances,
    }))
}
