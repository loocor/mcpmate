// MCPMate Proxy API handlers for MCP server CRUD operations
// Contains handler functions for creating, updating, and importing servers

use super::{common::*, instance::list_instances};
use crate::{
    api::handlers::ApiError,
    conf::{
        database::Database,
        models::{ConfigSuit, ConfigSuitType, ServerMeta},
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
            "Server with name '{}' already exists",
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
        name: payload.name,
        enabled,
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
        updated_server.server_type = kind.clone();
        // Update transport type based on server type
        updated_server.transport_type = match kind.as_str() {
            "stdio" => Some("Stdio".to_string()),
            "sse" => Some("Sse".to_string()),
            "streamable_http" => Some("StreamableHttp".to_string()),
            _ => None,
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

    // Return success response
    Ok(Json(ServerResponse {
        name,
        enabled: payload.enabled.unwrap_or(true), // Default to true if not provided
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

    // Process each server in the payload
    for (name, config) in payload.mcp_servers {
        // Create server model
        let server = match config.kind.as_str() {
            "stdio" => Server::new_stdio(name.clone(), config.command.clone()),
            "sse" => Server::new_sse(name.clone(), config.url.clone()),
            "streamable_http" => Server::new_streamable_http(name.clone(), config.url.clone()),
            _ => {
                failed_servers.push(name.clone());
                tracing::error!("Invalid server type for '{}': {}", name, config.kind);
                continue;
            }
        };

        // Insert server into database
        let server_id = match crate::conf::operations::upsert_server(&db.pool, &server).await {
            Ok(id) => id,
            Err(e) => {
                failed_servers.push(name.clone());
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
    }))
}
