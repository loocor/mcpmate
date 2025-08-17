// MCPMate Proxy API handlers for MCP server management operations
// Contains handler functions for enabling and disabling servers
//
// Server Status Synchronization Policy:
// 1. API operations have priority over config suit settings
// 2. When a server is disabled via API, it is disabled in all config suits
// 3. When a server is enabled via API, it is only enabled in the default config suit
// 4. Changes to server status in config suits trigger connection/disconnection operations
// 5. This creates a one-way synchronization where API operations take priority

use super::{common, shared::*};

// Helper functions for server management operations

/// Sync server connections by invalidating suit service cache
async fn sync_server_connections(state: &Arc<AppState>) -> Result<(), ApiError> {
    if let Some(merge_service) = &state.suit_merge_service {
        // Invalidate cache to force re-merging of configurations
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated suit service cache to sync server connections");
    }

    Ok(())
}

/// Sync client configurations using the client manager
async fn sync_client_configurations(
    state: &Arc<AppState>,
    config_suit_id: Option<String>,
) -> Result<(), ApiError> {
    // Use the helper function from suits::helpers
    crate::api::handlers::suits::helpers::sync_client_configurations(state, config_suit_id).await
}

/// Create operation response
fn create_operation_response(
    id: String,
    name: String,
    result: String,
    status: String,
    is_enabled: bool,
) -> Result<Json<OperationResponse>, ApiError> {
    let allowed_operations = if is_enabled {
        vec!["disable".to_string()]
    } else {
        vec!["enable".to_string()]
    };

    Ok(Json(OperationResponse {
        id,
        name,
        result,
        status,
        allowed_operations,
    }))
}

/// Enable a server by setting its global availability to enabled
pub async fn enable_server(
    state: State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<OperationResponse>, ApiError> {
    // Get database reference
    let db = common::get_database_from_state(&state)?;

    // Get the server information by ID
    let server_row = crate::config::server::get_server_by_id(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{id}' not found")))?;
    let server_id = server_row.id.clone().unwrap_or_default();
    let server_name = server_row.name.clone();

    // Update the server's global enabled status
    match crate::config::server::update_server_global_status(&db.pool, &server_id, true).await {
        Ok(true) => {
            tracing::info!("Set server '{}' global availability to enabled", server_name);
        }
        Ok(false) => {
            return Err(ApiError::NotFound(format!(
                "Server '{server_name}' not found when updating global status"
            )));
        }
        Err(e) => {
            return Err(ApiError::InternalError(format!(
                "Failed to update server '{server_name}' global status: {e}"
            )));
        }
    }

    // Sync server connections
    sync_server_connections(&state).await?;

    // Check if sync parameter is true
    let should_sync = query.get("sync").map(|v| v == "true").unwrap_or(false);
    if should_sync {
        // Spawn async task to sync client configurations
        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Err(e) = sync_client_configurations(&state_clone, None).await {
                tracing::warn!("Failed to sync client configurations: {}", e);
            }
        });
    }

    // Get connection pool
    let mut pool = common::get_connection_pool_with_timeout(&state).await?;

    // Load the server configuration from the database
    // This will update the connection pool's configuration with the latest server information
    if let Some(http_proxy) = &state.http_proxy {
        if let Some(db) = &http_proxy.database {
            // Load server configuration from database
            match crate::core::foundation::loader::load_server_config(db).await {
                Ok(config) => {
                    // Create a new Config with the loaded configuration
                    let new_config = std::sync::Arc::new(config);

                    // Update the connection pool's configuration
                    if let Err(e) = pool.set_config(new_config) {
                        tracing::error!("Failed to update connection pool configuration: {}", e);
                        return Err(ApiError::InternalError(format!(
                            "Failed to update pool configuration: {}",
                            e
                        )));
                    }

                    tracing::info!("Updated connection pool configuration with latest server information");
                }
                Err(e) => {
                    tracing::warn!("Failed to load server configuration: {}", e);
                }
            }
        }
    }

    // Check if the server exists in the connection pool
    if !pool.connections.contains_key(&server_name) {
        // Server not in connection pool, add it
        let connection = crate::core::connection::UpstreamConnection::new(server_name.clone());
        let instance_id = connection.id.clone();

        // Add to connection pool
        pool.connections
            .entry(server_name.clone())
            .or_insert_with(std::collections::HashMap::new)
            .insert(instance_id.clone(), connection);

        // Try to connect
        if let Err(e) = pool.connect(&server_name, &instance_id).await {
            tracing::warn!("Failed to connect to server '{}': {}", server_name, e);

            // Return success anyway, as the server is now enabled in the config suit
            return create_operation_response(
                instance_id,
                server_name,
                format!("Server enabled in configuration but connection failed: {e}"),
                "Enabled (Connection Failed)".to_string(),
                true,
            );
        }

        // Successfully connected
        return create_operation_response(
            instance_id,
            server_name,
            "Successfully enabled server with new connection".to_string(),
            "Enabled".to_string(),
            true,
        );
    }

    // Server exists in connection pool, check instances
    let instances = pool.connections.get(&server_name).unwrap();

    // If there are no instances, create one
    if instances.is_empty() {
        let connection = crate::core::connection::UpstreamConnection::new(server_name.clone());
        let instance_id = connection.id.clone();

        // Add to connection pool
        pool.connections
            .get_mut(&server_name)
            .unwrap()
            .insert(instance_id.clone(), connection);

        // Try to connect
        if let Err(e) = pool.connect(&server_name, &instance_id).await {
            tracing::warn!("Failed to connect to server '{}': {}", server_name, e);

            // Return success anyway, as the server is now enabled in the config suit
            return create_operation_response(
                instance_id,
                server_name,
                format!("Server enabled in configuration but connection failed: {e}"),
                "Enabled (Connection Failed)".to_string(),
                true,
            );
        }

        // Successfully connected
        return create_operation_response(
            instance_id,
            server_name,
            "Successfully enabled server with new connection".to_string(),
            "Enabled".to_string(),
            true,
        );
    }

    // Check if there's already a ready instance
    let ready_instance = instances
        .iter()
        .find(|(_, conn)| conn.is_connected())
        .map(|(id, _)| id.clone());

    if let Some(instance_id) = ready_instance {
        // Already has a ready instance, return success
        return create_operation_response(
            instance_id,
            server_name,
            "Server already enabled with active connection".to_string(),
            "Enabled".to_string(),
            true,
        );
    }

    // No ready instance, try to reconnect the first instance
    let first_instance_id = instances.keys().next().unwrap().clone();

    // Try to connect
    if let Err(e) = pool.connect(&server_name, &first_instance_id).await {
        tracing::warn!("Failed to connect to server '{}': {}", server_name, e);

        // Return success anyway, as the server is now enabled in the config suit
        return create_operation_response(
            first_instance_id,
            server_name,
            format!("Server enabled in configuration but connection failed: {e}"),
            "Enabled (Connection Failed)".to_string(),
            true,
        );
    }

    // Successfully connected
    create_operation_response(
        first_instance_id,
        server_name,
        "Successfully enabled server by reconnecting instance".to_string(),
        "Enabled".to_string(),
        true,
    )
}

/// Disable a server by setting its global availability to disabled
pub async fn disable_server(
    state: State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<OperationResponse>, ApiError> {
    // Get database reference
    let db = common::get_database_from_state(&state)?;

    // Get the server information by ID
    let server_row = crate::config::server::get_server_by_id(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get server: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{id}' not found")))?;
    let server_id = server_row.id.clone().unwrap_or_default();
    let server_name = server_row.name.clone();

    // Update the server's global enabled status
    match crate::config::server::update_server_global_status(&db.pool, &server_id, false).await {
        Ok(true) => {
            tracing::info!("Set server '{}' global availability to disabled", server_name);
        }
        Ok(false) => {
            return Err(ApiError::NotFound(format!(
                "Server '{server_name}' not found when updating global status"
            )));
        }
        Err(e) => {
            return Err(ApiError::InternalError(format!(
                "Failed to update server '{server_name}' global status: {e}"
            )));
        }
    }

    // Sync server connections
    sync_server_connections(&state).await?;

    // Check if sync parameter is true
    let should_sync = query.get("sync").map(|v| v == "true").unwrap_or(false);
    if should_sync {
        // Spawn async task to sync client configurations
        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Err(e) = sync_client_configurations(&state_clone, None).await {
                tracing::warn!("Failed to sync client configurations: {}", e);
            }
        });
    }

    // Get connection pool
    let pool_result = tokio::time::timeout(std::time::Duration::from_secs(1), state.connection_pool.lock()).await;

    let mut pool = match pool_result {
        Ok(pool) => pool,
        Err(_) => {
            // If we can't get the connection pool, just return success
            // The server is already disabled in the config suits
            return create_operation_response(
                "all".to_string(),
                server_name,
                "Server disabled in configuration (connection pool unavailable)".to_string(),
                "Disabled".to_string(),
                false,
            );
        }
    };

    // Check if the server exists in the connection pool
    if !pool.connections.contains_key(&server_name) {
        // Server not in connection pool, already disabled
        return create_operation_response(
            "all".to_string(),
            server_name,
            "Server already disabled (not in connection pool)".to_string(),
            "Disabled".to_string(),
            false,
        );
    }

    // Get all instance IDs
    let instance_ids: Vec<String> = pool.connections.get(&server_name).unwrap().keys().cloned().collect();

    if instance_ids.is_empty() {
        // No instances, already disabled
        return create_operation_response(
            "all".to_string(),
            server_name,
            "Server already disabled (no instances)".to_string(),
            "Disabled".to_string(),
            false,
        );
    }

    // Track the number of instances successfully disconnected
    let mut success_count = 0;
    let total_count = instance_ids.len();

    // Disconnect each instance
    for instance_id in &instance_ids {
        if let Err(e) = pool.disconnect(&server_name, instance_id).await {
            tracing::error!(
                "Failed to disconnect server '{}' instance '{}': {}",
                server_name,
                instance_id,
                e
            );
        } else {
            success_count += 1;
            tracing::info!(
                "Successfully disconnected server '{}' instance '{}'",
                server_name,
                instance_id
            );
        }
    }

    // Check if all instances are disconnected
    let all_disconnected = success_count == total_count;

    let status = if all_disconnected {
        "Disabled"
    } else {
        "Partially Disabled"
    };

    create_operation_response(
        "all".to_string(),
        server_name,
        format!("Successfully disabled server ({success_count} of {total_count} instances disconnected)"),
        status.to_string(),
        false,
    )
}
