// MCPMate Proxy API handlers for MCP server CRUD operations
// Contains handler functions for creating, updating, and importing servers

use std::collections::HashMap;

use super::{common::*, instance::list_instances};
use crate::{
    api::{handlers::ApiError, models::server::ServerMetaInfo},
    common::types::{ConfigSuitType, ServerType},
    conf::{
        database::Database,
        models::{ConfigSuit, ServerMeta},
        operations,
    },
};

// Private helper functions

/// Get database reference from AppState
///
/// This function extracts the database reference from the AppState and returns
/// a Result with either the database reference or an ApiError if the database
/// is not available.
fn get_database_from_state(state: &Arc<AppState>) -> Result<Arc<Database>, ApiError> {
    match state.http_proxy.as_ref().and_then(|p| p.database.clone()) {
        Some(db) => Ok(db),
        None => Err(ApiError::InternalError(
            "Database not available".to_string(),
        )),
    }
}

/// Get or create default config suit
///
/// This function retrieves the default config suit from the database, or creates
/// it if it doesn't exist. Returns the suit ID on success, or an ApiError on failure.
async fn get_or_create_default_config_suit(db: &Database) -> Result<String, ApiError> {
    // Get the default config suit
    let default_suit = operations::get_config_suit_by_name(&db.pool, "default")
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get default config suit: {e}")))?;

    // Return the suit ID if it exists, or create a new one
    if let Some(suit) = default_suit {
        Ok(suit.id.unwrap())
    } else {
        // Create default config suit if it doesn't exist
        let new_suit = ConfigSuit::new("default".to_string(), ConfigSuitType::Shared);
        operations::upsert_config_suit(&db.pool, &new_suit)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to create default config suit: {e}"))
            })
    }
}

/// Add server to config suit
///
/// This function adds a server to a config suit with the specified enabled status.
/// Returns Ok(()) on success, or an ApiError on failure.
async fn add_server_to_suit(
    db: &Database,
    suit_id: &str,
    server_id: &str,
    enabled: bool,
) -> Result<(), ApiError> {
    operations::add_server_to_config_suit(&db.pool, suit_id, server_id, enabled)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!(
                "Failed to update server status in config suit: {e}"
            ))
        })?;

    Ok(())
}

/// Create server metadata
///
/// This function creates basic metadata for a server and inserts it into the database.
/// Returns Ok(()) on success, or an ApiError on failure.
async fn create_server_metadata(
    db: &Database,
    server_id: &str,
    description: &str,
) -> Result<(), ApiError> {
    // Create basic server metadata
    let meta = ServerMeta {
        id: None,
        server_id: server_id.to_string(),
        description: Some(description.to_string()),
        author: None,
        website: None,
        repository: None,
        category: None,
        recommended_scenario: None,
        rating: None,
        created_at: None,
        updated_at: None,
    };

    // Insert server metadata
    operations::upsert_server_meta(&db.pool, &meta)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create server metadata: {e}")))?;

    Ok(())
}

/// Create a new MCP server
pub async fn create_server(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateServerRequest>,
) -> Result<Json<ServerResponse>, ApiError> {
    // Get database reference
    let db = get_database_from_state(&state)?;

    // Check if a server with the same name already exists
    let existing_server = crate::conf::operations::get_server(&db.pool, &payload.name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to check server: {e}")))?;

    if existing_server.is_some() {
        return Err(ApiError::Conflict(format!(
            "Server with name '{}' already exists. Please choose a different name for your server.",
            payload.name
        )));
    }

    // Validate server type
    match payload.kind.as_str() {
        "stdio" =>
            if payload.command.is_none() {
                return Err(ApiError::BadRequest(
                    "Command is required for stdio servers".to_string(),
                ));
            },
        "sse" | "streamable_http" =>
            if payload.url.is_none() {
                return Err(ApiError::BadRequest(format!(
                    "URL is required for {} servers",
                    payload.kind
                )));
            },
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Invalid server type: {}. Must be one of: stdio, sse, streamable_http",
                payload.kind
            )));
        }
    }

    // Create server model
    let server = match payload.kind.as_str() {
        "stdio" => Server::new_stdio(payload.name.clone(), payload.command.clone()),
        "sse" => Server::new_sse(payload.name.clone(), payload.url.clone()),
        "streamable_http" => Server::new_streamable_http(payload.name.clone(), payload.url.clone()),
        _ => unreachable!(), // We already validated the server type
    };

    // Insert server into database
    let server_id = crate::conf::operations::upsert_server(&db.pool, &server)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create server: {e}")))?;

    // Insert server arguments if provided
    if let Some(args) = &payload.args {
        crate::conf::operations::upsert_server_args(&db.pool, &server_id, args)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to create server arguments: {e}"))
            })?;
    }

    // Insert server environment variables if provided
    if let Some(env) = &payload.env {
        crate::conf::operations::upsert_server_env(&db.pool, &server_id, env)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!(
                    "Failed to create server environment variables: {e}"
                ))
            })?;
    }

    // Create server metadata
    create_server_metadata(&db, &server_id, "Created via API").await?;

    // Add server to default config suit if enabled flag is true or not provided (default to true)
    let enabled = payload.enabled.unwrap_or(true);
    if enabled {
        // Get or create the default config suit
        let suit_id = get_or_create_default_config_suit(&db).await?;

        // Enable the server in the config suit
        add_server_to_suit(&db, &suit_id, &server_id, true).await?;

        tracing::info!("Enabled server '{}' in default config suit", payload.name);
    }

    // Return success response
    Ok(Json(ServerResponse {
        name: payload.name.clone(),
        enabled,
        globally_enabled: true, // New servers are globally enabled by default
        enabled_in_suits: enabled, // Same as enabled for new servers
        server_type: payload.kind.parse().unwrap_or(ServerType::Stdio),
        command: payload.command.clone(),
        url: payload.url.clone(),
        args: payload.args.clone(),
        env: payload.env.clone(),
        meta: None, // No metadata yet
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        instances: Vec::new(), // No instances yet
    }))
}

/// Update an existing MCP server
pub async fn update_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(payload): Json<UpdateServerRequest>,
) -> Result<Json<ServerResponse>, ApiError> {
    // Get database reference
    let db = get_database_from_state(&state)?;

    // Check if the server exists
    let existing_server = crate::conf::operations::get_server(&db.pool, &name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to check server: {e}")))?;

    let existing_server = match existing_server {
        Some(server) => server,
        None => {
            return Err(ApiError::NotFound(format!("Server '{name}' not found")));
        }
    };

    // Note: The server name cannot be changed through the update_server endpoint
    // The name is part of the URL path and is used to identify the server
    // If a name change is needed, the client should create a new server and delete the old one

    // Get the server ID
    let server_id = match &existing_server.id {
        Some(id) => id.clone(),
        None => {
            return Err(ApiError::InternalError("Server ID not found".to_string()));
        }
    };

    // Validate server type if provided
    if let Some(kind) = &payload.kind {
        match kind.as_str() {
            "stdio" =>
                if payload.command.is_none() && existing_server.command.is_none() {
                    return Err(ApiError::BadRequest(
                        "Command is required for stdio servers".to_string(),
                    ));
                },
            "sse" | "streamable_http" =>
                if payload.url.is_none() && existing_server.url.is_none() {
                    return Err(ApiError::BadRequest(format!(
                        "URL is required for {kind} servers"
                    )));
                },
            _ => {
                return Err(ApiError::BadRequest(format!(
                    "Invalid server type: {kind}. Must be one of: stdio, sse, streamable_http"
                )));
            }
        }
    }

    // Create updated server model
    let mut updated_server = existing_server.clone();
    if let Some(kind) = payload.kind {
        // Update server type based on kind string
        updated_server.server_type = kind.parse().unwrap_or(updated_server.server_type);

        // Update transport type based on server type
        updated_server.transport_type = match updated_server.server_type {
            ServerType::Stdio => Some(crate::common::types::TransportType::Stdio),
            ServerType::Sse => Some(crate::common::types::TransportType::Sse),
            ServerType::StreamableHttp => Some(crate::common::types::TransportType::StreamableHttp),
        };
    }
    if let Some(command) = payload.command {
        updated_server.command = Some(command);
    }
    if let Some(url) = payload.url {
        updated_server.url = Some(url);
    }

    // Update server in database
    crate::conf::operations::upsert_server(&db.pool, &updated_server)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update server: {e}")))?;

    // Update server arguments if provided
    if let Some(args) = &payload.args {
        crate::conf::operations::upsert_server_args(&db.pool, &server_id, args)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to update server arguments: {e}"))
            })?;
    }

    // Update server environment variables if provided
    if let Some(env) = &payload.env {
        crate::conf::operations::upsert_server_env(&db.pool, &server_id, env)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!(
                    "Failed to update server environment variables: {e}"
                ))
            })?;
    }

    // Update server enabled status if provided
    if let Some(enabled) = payload.enabled {
        // Get or create the default config suit
        let suit_id = get_or_create_default_config_suit(&db).await?;

        // Update the server in the config suit
        add_server_to_suit(&db, &suit_id, &server_id, enabled).await?;

        tracing::info!(
            "Updated server '{}' enabled status to {} in default config suit",
            name,
            enabled
        );
    }

    // Get current instances for the server
    let instances_response = match list_instances(State(state.clone()), Path(name.clone())).await {
        Ok(response) => response.0.instances,
        Err(_) => Vec::new(), // No instances or error
    };

    // Get server ID
    let server_id = updated_server.id.clone().unwrap_or_default();

    // Get server arguments if available
    let args = if !server_id.is_empty() {
        match crate::conf::operations::get_server_args(&db.pool, &server_id).await {
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
        match crate::conf::operations::get_server_env(&db.pool, &server_id).await {
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
        match crate::conf::operations::get_server_meta(&db.pool, &server_id).await {
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
    let created_at = updated_server.created_at.map(|dt| dt.to_rfc3339());
    let updated_at = updated_server.updated_at.map(|dt| dt.to_rfc3339());

    // Get server global enabled status
    let globally_enabled =
        match crate::conf::operations::server::get_server_global_status(&db.pool, &server_id).await
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

    // Get server enabled status in config suits
    let enabled_in_suits =
        match crate::conf::operations::is_server_enabled_in_any_suit(&db.pool, &server_id).await {
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

    // For backward compatibility, enabled is the value from the payload or true if not provided
    let enabled = payload.enabled.unwrap_or(true);

    // Return success response
    Ok(Json(ServerResponse {
        name,
        enabled,
        globally_enabled,
        enabled_in_suits,
        server_type: updated_server.server_type, // Use the existing or updated server type
        command: updated_server.command.clone(),
        url: updated_server.url.clone(),
        args,
        env,
        meta,
        created_at,
        updated_at,
        instances: instances_response,
    }))
}

/// Import servers from JSON configuration
pub async fn import_servers(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ImportServersRequest>,
) -> Result<Json<ImportServersResponse>, ApiError> {
    // Get database reference
    let db = get_database_from_state(&state)?;

    let mut imported_servers = Vec::new();
    let mut failed_servers = Vec::new();
    let mut error_details = HashMap::new();

    // Process each server in the payload
    for (name, config) in payload.mcp_servers {
        // Check if a server with the same name already exists
        let existing_server = match crate::conf::operations::get_server(&db.pool, &name).await {
            Ok(server) => server,
            Err(e) => {
                failed_servers.push(name.clone());
                let error_msg = format!("Failed to check if server exists: {e}");
                error_details.insert(name.clone(), error_msg.clone());
                tracing::error!("Failed to check if server '{}' exists: {}", name, e);
                continue;
            }
        };

        // If the server already exists, add it to the failed list and continue
        if existing_server.is_some() {
            failed_servers.push(name.clone());
            let error_msg = format!(
                "Server with name '{name}' already exists. Please choose a different name."
            );
            error_details.insert(name.clone(), error_msg.clone());
            tracing::error!(
                "Server with name '{}' already exists. Skipping import.",
                name
            );
            continue;
        }

        // Create server model
        let server = match config.kind.as_str() {
            "stdio" => Server::new_stdio(name.clone(), config.command.clone()),
            "sse" => Server::new_sse(name.clone(), config.url.clone()),
            "streamable_http" => Server::new_streamable_http(name.clone(), config.url.clone()),
            _ => {
                failed_servers.push(name.clone());
                let error_msg = format!(
                    "Invalid server type: '{}'. Must be one of: stdio, sse, streamable_http",
                    config.kind
                );
                error_details.insert(name.clone(), error_msg.clone());
                tracing::error!("Invalid server type for '{}': {}", name, config.kind);
                continue;
            }
        };

        // Insert server into database
        let server_id = match crate::conf::operations::upsert_server(&db.pool, &server).await {
            Ok(id) => id,
            Err(e) => {
                failed_servers.push(name.clone());
                let error_msg = format!("Failed to create server: {e}");
                error_details.insert(name.clone(), error_msg.clone());
                tracing::error!("Failed to create server '{}': {}", name, e);
                continue;
            }
        };

        // Insert server arguments if provided
        if let Some(args) = &config.args {
            if let Err(e) =
                crate::conf::operations::upsert_server_args(&db.pool, &server_id, args).await
            {
                tracing::error!("Failed to create arguments for server '{}': {}", name, e);
                // Continue anyway, this is not a critical error
            }
        }

        // Insert server environment variables if provided
        if let Some(env) = &config.env {
            if let Err(e) =
                crate::conf::operations::upsert_server_env(&db.pool, &server_id, env).await
            {
                tracing::error!(
                    "Failed to create environment variables for server '{}': {}",
                    name,
                    e
                );
                // Continue anyway, this is not a critical error
            }
        }

        // Create server metadata
        if let Err(e) = create_server_metadata(&db, &server_id, "Imported via API").await {
            tracing::error!("Failed to create metadata for server '{}': {}", name, e);
            // Continue anyway, this is not a critical error
        }

        // Add server to default config suit (enabled by default)
        // Try to get or create the default config suit
        let suit_id = match get_or_create_default_config_suit(&db).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to get or create default config suit: {}", e);
                // Continue anyway, this is not a critical error
                continue;
            }
        };

        // Enable the server in the config suit
        if let Err(e) = add_server_to_suit(&db, &suit_id, &server_id, true).await {
            tracing::error!("Failed to enable server '{}' in config suit: {}", name, e);
            // Continue anyway, this is not a critical error
        }

        tracing::info!("Successfully imported server '{}'", name);
        imported_servers.push(name);
    }

    // Return success response
    Ok(Json(ImportServersResponse {
        imported_count: imported_servers.len(),
        imported_servers,
        failed_servers,
        error_details: if error_details.is_empty() {
            None
        } else {
            Some(error_details)
        },
    }))
}

/// Delete an existing MCP server
pub async fn delete_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<OperationResponse>, ApiError> {
    // Get database reference
    let db = get_database_from_state(&state)?;

    // Check if the server exists
    let existing_server = crate::conf::operations::get_server(&db.pool, &name)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to check server: {e}")))?;

    let existing_server = match existing_server {
        Some(server) => server,
        None => {
            return Err(ApiError::NotFound(format!("Server '{name}' not found")));
        }
    };

    // Get the server ID
    let server_id = match &existing_server.id {
        Some(id) => id.clone(),
        None => {
            return Err(ApiError::InternalError("Server ID not found".to_string()));
        }
    };

    // First, disconnect all instances of this server if it's in the connection pool
    let pool_result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        state.connection_pool.lock(),
    )
    .await;

    if let Ok(mut pool) = pool_result {
        // Check if the server exists in the connection pool
        if let Some(instances) = pool.connections.get(&name) {
            // Get all instance IDs first to avoid borrowing issues
            let instance_ids: Vec<String> = instances.keys().cloned().collect();

            // Disconnect each instance
            for instance_id in instance_ids {
                if let Err(e) = pool.disconnect(&name, &instance_id).await {
                    tracing::warn!(
                        "Failed to disconnect instance '{}' of server '{}': {}",
                        instance_id,
                        name,
                        e
                    );
                    // Continue anyway, we still want to delete the server
                }
            }
        }
    } else {
        tracing::warn!(
            "Timed out waiting for connection pool lock, proceeding with server deletion anyway"
        );
    }

    // Start a transaction for deleting all related records
    let mut tx = db
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to begin transaction: {e}")))?;

    // 1. Delete server from all config suits
    // First, get all config suits that contain this server
    let config_suits = crate::conf::operations::get_all_config_suits(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get config suits: {e}")))?;

    for suit in config_suits {
        if let Some(suit_id) = &suit.id {
            // Delete server from this config suit
            sqlx::query(
                r#"
                DELETE FROM config_suit_server
                WHERE config_suit_id = ? AND server_id = ?
                "#,
            )
            .bind(suit_id)
            .bind(&server_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to delete server from config suit: {e}"))
            })?;

            // Delete all tools associated with this server from this config suit
            sqlx::query(
                r#"
                DELETE FROM config_suit_tool
                WHERE config_suit_id = ? AND server_id = ?
                "#,
            )
            .bind(suit_id)
            .bind(&server_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!(
                    "Failed to delete server tools from config suit: {e}"
                ))
            })?;
        }
    }

    // 2. Delete server metadata
    sqlx::query(
        r#"
        DELETE FROM server_meta
        WHERE server_id = ?
        "#,
    )
    .bind(&server_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to delete server metadata: {e}")))?;

    // 3. Delete server environment variables
    sqlx::query(
        r#"
        DELETE FROM server_env
        WHERE server_id = ?
        "#,
    )
    .bind(&server_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        ApiError::InternalError(format!(
            "Failed to delete server environment variables: {e}"
        ))
    })?;

    // 4. Delete server arguments
    sqlx::query(
        r#"
        DELETE FROM server_args
        WHERE server_id = ?
        "#,
    )
    .bind(&server_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to delete server arguments: {e}")))?;

    // 5. Finally, delete the server itself
    sqlx::query(
        r#"
        DELETE FROM server_config
        WHERE id = ?
        "#,
    )
    .bind(&server_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to delete server: {e}")))?;

    // Commit the transaction
    tx.commit()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to commit transaction: {e}")))?;

    tracing::info!("Successfully deleted server '{}'", name);

    // Return success response
    Ok(Json(OperationResponse {
        id: server_id,
        name,
        result: "Successfully deleted server".to_string(),
        status: "Deleted".to_string(),
        allowed_operations: Vec::new(), // No operations allowed on deleted server
    }))
}
