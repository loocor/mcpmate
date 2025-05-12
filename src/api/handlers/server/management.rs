// MCPMate Proxy API handlers for MCP server management operations
// Contains handler functions for enabling and disabling servers

use super::common::*;
use super::instance::list_instances;

/// Enable a server by reconnecting existing instances or creating a new one if needed
pub async fn enable_server(
    state: State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<OperationResponse>, ApiError> {
    // call list_instances to check if there are any instance records
    let instances_response = list_instances(state.clone(), Path(server_name.clone())).await?;
    let instances = instances_response.0.instances;

    // Update config_suit to enable server
    if let Some(db) = &state.http_proxy.as_ref().and_then(|p| p.database.clone()) {
        // Get the server ID
        let server = crate::conf::operations::get_server(&db.pool, &server_name)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to get server: {}", e)))?;

        if let Some(server) = server {
            if let Some(server_id) = server.id {
                // Get or create the default config suit
                let default_suit =
                    crate::conf::operations::get_config_suit_by_name(&db.pool, "default")
                        .await
                        .map_err(|e| {
                            ApiError::InternalError(format!(
                                "Failed to get default config suit: {}",
                                e
                            ))
                        })?;

                let suit_id = if let Some(suit) = default_suit {
                    suit.id.unwrap()
                } else {
                    // Create default config suit if it doesn't exist
                    let new_suit = crate::conf::models::ConfigSuit::new(
                        "default".to_string(),
                        crate::conf::models::ConfigSuitType::Shared,
                    );
                    crate::conf::operations::upsert_config_suit(&db.pool, &new_suit)
                        .await
                        .map_err(|e| {
                            ApiError::InternalError(format!(
                                "Failed to create default config suit: {}",
                                e
                            ))
                        })?
                };

                // Enable the server in the config suit
                crate::conf::operations::add_server_to_config_suit(
                    &db.pool, &suit_id, &server_id, true,
                )
                .await
                .map_err(|e| {
                    ApiError::InternalError(format!(
                        "Failed to enable server in config suit: {}",
                        e
                    ))
                })?;

                tracing::info!("Enabled server '{}' in default config suit", server_name);
            }
        }
    }

    if instances.is_empty() {
        // no instance records, return error
        return Err(ApiError::NotFound(format!(
            "No instances found for server '{}'",
            server_name
        )));
    }

    // find if there is a ready instance
    let ready_instance = instances.iter().find(|instance| instance.status == "Ready");

    if let Some(instance) = ready_instance {
        // already has a ready instance, return success
        return Ok(Json(OperationResponse {
            id: instance.id.clone(),
            name: server_name,
            result: format!("Server already enabled with instance '{}'", instance.id),
            status: instance.status.clone(),
            allowed_operations: vec!["disable".to_string()],
        }));
    }

    // no ready instance, try to reconnect the first instance
    let first_instance = &instances[0];

    // call reset_reconnect to reconnect
    match crate::api::handlers::instance::reset_reconnect(
        state.clone(),
        Path((server_name.clone(), first_instance.id.clone())),
    )
    .await
    {
        Ok(response) => {
            // successfully reconnected
            Ok(Json(OperationResponse {
                id: first_instance.id.clone(),
                name: server_name,
                result: format!(
                    "Successfully enabled server by reconnecting instance '{}'",
                    first_instance.id
                ),
                status: response.0.status,
                allowed_operations: vec!["disable".to_string()],
            }))
        }
        Err(e) => {
            // failed to reconnect
            Err(ApiError::BadRequest(format!(
                "Failed to enable server: {}",
                e
            )))
        }
    }
}

/// Disable a server by force disconnecting all instances
pub async fn disable_server(
    state: State<Arc<AppState>>,
    Path(server_name): Path<String>,
) -> Result<Json<OperationResponse>, ApiError> {
    // call list_instances to check if there are any instance records
    let instances_response = list_instances(state.clone(), Path(server_name.clone())).await?;
    let instances = instances_response.0.instances;

    // Update config_suit to disable server
    if let Some(db) = &state.http_proxy.as_ref().and_then(|p| p.database.clone()) {
        // Get the server ID
        let server = crate::conf::operations::get_server(&db.pool, &server_name)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to get server: {}", e)))?;

        if let Some(server) = server {
            if let Some(server_id) = server.id {
                // Get or create the default config suit
                let default_suit =
                    crate::conf::operations::get_config_suit_by_name(&db.pool, "default")
                        .await
                        .map_err(|e| {
                            ApiError::InternalError(format!(
                                "Failed to get default config suit: {}",
                                e
                            ))
                        })?;

                let suit_id = if let Some(suit) = default_suit {
                    suit.id.unwrap()
                } else {
                    // Create default config suit if it doesn't exist
                    let new_suit = crate::conf::models::ConfigSuit::new(
                        "default".to_string(),
                        crate::conf::models::ConfigSuitType::Shared,
                    );
                    crate::conf::operations::upsert_config_suit(&db.pool, &new_suit)
                        .await
                        .map_err(|e| {
                            ApiError::InternalError(format!(
                                "Failed to create default config suit: {}",
                                e
                            ))
                        })?
                };

                // Disable the server in the config suit
                crate::conf::operations::add_server_to_config_suit(
                    &db.pool, &suit_id, &server_id, false,
                )
                .await
                .map_err(|e| {
                    ApiError::InternalError(format!(
                        "Failed to disable server in config suit: {}",
                        e
                    ))
                })?;

                tracing::info!("Disabled server '{}' in default config suit", server_name);
            }
        }
    }

    if instances.is_empty() {
        // no instance records, return success (already disabled)
        return Ok(Json(OperationResponse {
            id: "".to_string(),
            name: server_name.clone(),
            result: format!(
                "Server '{}' already disabled (no instances found)",
                server_name
            ),
            status: "Disabled".to_string(),
            allowed_operations: vec!["enable".to_string()],
        }));
    }

    // track the number of instances successfully disconnected
    let mut success_count = 0;
    let total_count = instances.len();

    // force disconnect each instance
    for instance in &instances {
        match crate::api::handlers::instance::force_disconnect(
            state.clone(),
            Path((server_name.clone(), instance.id.clone())),
        )
        .await
        {
            Ok(_) => {
                success_count += 1;
                tracing::info!(
                    "Successfully disconnected server '{}' instance '{}'",
                    server_name,
                    instance.id
                );
            }
            Err(e) => {
                tracing::error!(
                    "Failed to disconnect server '{}' instance '{}': {}",
                    server_name,
                    instance.id,
                    e
                );
            }
        }
    }

    // call list_instances again to check if all instances are disconnected
    let updated_instances_response =
        list_instances(state.clone(), Path(server_name.clone())).await?;
    let updated_instances = updated_instances_response.0.instances;

    // check if all instances are disconnected
    let all_disconnected = updated_instances
        .iter()
        .all(|instance| instance.status != "Ready");

    let status = if all_disconnected {
        "Disabled"
    } else {
        "Partially Disabled"
    };

    Ok(Json(OperationResponse {
        id: "all".to_string(),
        name: server_name,
        result: format!(
            "Successfully disabled server ({} of {} instances disconnected)",
            success_count, total_count
        ),
        status: status.to_string(),
        allowed_operations: vec!["enable".to_string()],
    }))
}
