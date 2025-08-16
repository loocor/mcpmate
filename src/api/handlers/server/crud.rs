// MCPMate Proxy API handlers for MCP server CRUD operations
// Contains handler functions for creating, updating, and importing servers

use std::collections::HashMap;

use super::{common, shared::*};
use crate::{
    api::handlers::ApiError,
    common::{config::ConfigSuitType, server::ServerType},
    config::{
        database::Database,
        models::{ConfigSuit, ServerMeta},
        server::{self},
        suit,
    },
};

// Private helper functions

/// Convert database error to ApiError
fn db_error(e: impl std::fmt::Display) -> ApiError {
    ApiError::InternalError(format!("Database error: {e}"))
}

/// Create internal error
fn internal_error(msg: &str) -> ApiError {
    ApiError::InternalError(msg.to_string())
}

/// Validate server configuration
fn validate_server_config(
    kind: &str,
    command: &Option<String>,
    url: &Option<String>,
) -> Result<(), ApiError> {
    match kind {
        "stdio" => {
            if command.is_none() {
                return Err(ApiError::BadRequest(
                    "Command is required for stdio servers".to_string(),
                ));
            }
        }
        "sse" | "streamable_http" => {
            if url.is_none() {
                return Err(ApiError::BadRequest(format!("URL is required for {kind} servers")));
            }
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "Invalid server type: {kind}. Must be one of: stdio, sse, streamable_http"
            )));
        }
    }
    Ok(())
}

/// Get existing server or return error
async fn get_existing_server_or_error(
    db: &Database,
    name: &str,
) -> Result<crate::config::models::Server, ApiError> {
    let server = crate::config::server::get_server(&db.pool, name)
        .await
        .map_err(db_error)?;

    server.ok_or_else(|| ApiError::NotFound(format!("Server '{name}' not found")))
}

/// Create server model from configuration
fn create_server_from_config(
    name: String,
    kind: &str,
    command: Option<String>,
    url: Option<String>,
) -> Server {
    match kind {
        "stdio" => Server::new_stdio(name, command),
        "sse" => Server::new_sse(name, url),
        "streamable_http" => Server::new_streamable_http(name, url),
        _ => unreachable!(), // Already validated
    }
}

/// Get or create default config suit
async fn get_or_create_default_config_suit(db: &Database) -> Result<String, ApiError> {
    let default_suit = suit::get_config_suit_by_name(&db.pool, "default")
        .await
        .map_err(db_error)?;

    if let Some(suit) = default_suit {
        Ok(suit.id.unwrap())
    } else {
        let new_suit = ConfigSuit::new("default".to_string(), ConfigSuitType::Shared);
        suit::upsert_config_suit(&db.pool, &new_suit).await.map_err(db_error)
    }
}

/// Add server to config suit
async fn add_server_to_suit(
    db: &Database,
    suit_id: &str,
    server_id: &str,
    enabled: bool,
) -> Result<(), ApiError> {
    suit::add_server_to_config_suit(&db.pool, suit_id, server_id, enabled)
        .await
        .map_err(db_error)
        .map(|_| ())
}

/// Add server to config suit with capabilities sync
async fn add_server_to_suit_with_sync(
    _state: &Arc<AppState>,
    db: &Database,
    suit_id: &str,
    server_id: &str,
    enabled: bool,
) -> Result<(), ApiError> {
    // Add server to suit
    suit::add_server_to_config_suit(&db.pool, suit_id, server_id, enabled)
        .await
        .map_err(db_error)?;

    // Sync server capabilities to the configuration suit (async, non-blocking)
    if false {
        let pool_clone = db.pool.clone();
        let suit_id_clone = suit_id.to_string();
        let server_id_clone = server_id.to_string();
        let _noop = ();

        // Use the same semaphore to limit concurrent operations
        static CAPABILITY_SYNC_SEMAPHORE: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
        let semaphore = CAPABILITY_SYNC_SEMAPHORE.get_or_init(|| tokio::sync::Semaphore::new(2));

        tokio::spawn(async move {
            // Acquire semaphore permit
            let _permit = match semaphore.try_acquire() {
                Ok(permit) => permit,
                Err(_) => {
                    tracing::warn!(
                        "Too many concurrent capability sync operations. Skipping sync for server {} to suit {}",
                        server_id_clone,
                        suit_id_clone
                    );
                    return;
                }
            };

            if let Err(e) =
                crate::config::suit::sync_server_capabilities_to_suit(&pool_clone, &suit_id_clone, &server_id_clone)
                    .await
            {
                tracing::warn!(
                    "Failed to sync capabilities for server {} to suit {}: {}",
                    server_id_clone,
                    suit_id_clone,
                    e
                );
            } else {
                tracing::info!(
                    "Successfully synced capabilities for server {} to suit {}",
                    server_id_clone,
                    suit_id_clone
                );
            }
        });
    }

    Ok(())
}

/// Create server metadata
async fn create_server_metadata(
    db: &Database,
    server_id: &str,
    description: &str,
) -> Result<(), ApiError> {
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

    server::upsert_server_meta(&db.pool, &meta)
        .await
        .map_err(db_error)
        .map(|_| ())
}

/// Create a new MCP server
pub async fn create_server(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateServerRequest>,
) -> Result<Json<ServerResponse>, ApiError> {
    let db = common::get_database_from_state(&state)?;

    // Check if server already exists
    if crate::config::server::get_server(&db.pool, &payload.name)
        .await
        .map_err(db_error)?
        .is_some()
    {
        return Err(ApiError::Conflict(format!(
            "Server with name '{}' already exists. Please choose a different name for your server.",
            payload.name
        )));
    }

    // Validate server configuration
    validate_server_config(&payload.kind, &payload.command, &payload.url)?;

    // Create server model
    let server = create_server_from_config(
        payload.name.clone(),
        &payload.kind,
        payload.command.clone(),
        payload.url.clone(),
    );

    // Insert server into database
    let server_id = crate::config::server::upsert_server(&db.pool, &server)
        .await
        .map_err(db_error)?;

    // Insert server arguments if provided
    if let Some(args) = &payload.args {
        crate::config::server::upsert_server_args(&db.pool, &server_id, args)
            .await
            .map_err(db_error)?;
    }

    // Insert server environment variables if provided
    if let Some(env) = &payload.env {
        crate::config::server::upsert_server_env(&db.pool, &server_id, env)
            .await
            .map_err(db_error)?;
    }

    // Create server metadata
    create_server_metadata(&db, &server_id, "Created via API").await?;

    // Add server to default config suit if enabled
    let enabled = payload.enabled.unwrap_or(true);
    if enabled {
        let suit_id = get_or_create_default_config_suit(&db).await?;
        add_server_to_suit(&db, &suit_id, &server_id, true).await?;
        tracing::info!("Enabled server '{}' in default config suit", payload.name);
    }

    // Return success response
    Ok(Json(ServerResponse {
        id: Some(server_id),
        name: payload.name.clone(),
        enabled,
        globally_enabled: true,
        enabled_in_suits: enabled,
        server_type: payload.kind.parse().unwrap_or(ServerType::Stdio),
        command: payload.command.clone(),
        url: payload.url.clone(),
        args: payload.args.clone(),
        env: payload.env.clone(),
        meta: None,
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
        instances: Vec::new(),
    }))
}

/// Update an existing MCP server
pub async fn update_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(payload): Json<UpdateServerRequest>,
) -> Result<Json<ServerResponse>, ApiError> {
    let db = common::get_database_from_state(&state)?;

    // Get existing server
    let existing_server = get_existing_server_or_error(&db, &name).await?;
    let server_id = existing_server
        .id
        .clone()
        .ok_or_else(|| internal_error("Server ID not found"))?;

    // Validate server type if provided
    if let Some(kind) = &payload.kind {
        let command = payload.command.as_ref().or(existing_server.command.as_ref());
        let url = payload.url.as_ref().or(existing_server.url.as_ref());
        validate_server_config(kind, &command.cloned(), &url.cloned())?;
    }

    // Create updated server model
    let mut updated_server = existing_server.clone();
    if let Some(kind) = payload.kind {
        updated_server.server_type = kind.parse().unwrap_or(updated_server.server_type);
        updated_server.transport_type = match updated_server.server_type {
            ServerType::Stdio => Some(crate::common::server::TransportType::Stdio),
            ServerType::Sse => Some(crate::common::server::TransportType::Sse),
            ServerType::StreamableHttp => Some(crate::common::server::TransportType::StreamableHttp),
        };
    }
    if let Some(command) = payload.command {
        updated_server.command = Some(command);
    }
    if let Some(url) = payload.url {
        updated_server.url = Some(url);
    }

    // Update server in database
    crate::config::server::upsert_server(&db.pool, &updated_server)
        .await
        .map_err(db_error)?;

    // Update server arguments if provided
    if let Some(args) = &payload.args {
        crate::config::server::upsert_server_args(&db.pool, &server_id, args)
            .await
            .map_err(db_error)?;
    }

    // Update server environment variables if provided
    if let Some(env) = &payload.env {
        crate::config::server::upsert_server_env(&db.pool, &server_id, env)
            .await
            .map_err(db_error)?;
    }

    // Update server enabled status if provided
    if let Some(enabled) = payload.enabled {
        let suit_id = get_or_create_default_config_suit(&db).await?;
        add_server_to_suit_with_sync(&state, &db, &suit_id, &server_id, enabled).await?;
        tracing::info!(
            "Updated server '{}' enabled status to {} in default config suit",
            name,
            enabled
        );
    }

    // Get server details via shared helper
    let details = common::get_complete_server_details(&db.pool, &server_id, &name, &state).await;

    // Return success response
    Ok(Json(ServerResponse {
        id: Some(server_id),
        name,
        enabled: payload.enabled.unwrap_or(true),
        globally_enabled: details.globally_enabled,
        enabled_in_suits: details.enabled_in_suits,
        server_type: updated_server.server_type,
        command: updated_server.command.clone(),
        url: updated_server.url.clone(),
        args: details.args,
        env: details.env,
        meta: details.meta,
        created_at: updated_server.created_at.map(|dt| dt.to_rfc3339()),
        updated_at: updated_server.updated_at.map(|dt| dt.to_rfc3339()),
        instances: details.instances,
    }))
}

/// Helper function to import a single server
async fn import_single_server(
    state: &Arc<AppState>,
    db: &Database,
    name: String,
    config: crate::api::models::server::ImportServerConfig,
) -> Result<(), String> {
    // Check if server already exists
    if crate::config::server::get_server(&db.pool, &name)
        .await
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err(format!(
            "Server with name '{name}' already exists. Please choose a different name."
        ));
    }

    // Validate and create server
    validate_server_config(&config.kind, &config.command, &config.url).map_err(|e| e.to_string())?;
    let server = create_server_from_config(name.clone(), &config.kind, config.command.clone(), config.url.clone());

    // Insert server into database
    let server_id = crate::config::server::upsert_server(&db.pool, &server)
        .await
        .map_err(|e| e.to_string())?;

    // Insert optional data (non-critical errors)
    if let Some(args) = &config.args {
        let _ = crate::config::server::upsert_server_args(&db.pool, &server_id, args).await;
    }
    if let Some(env) = &config.env {
        let _ = crate::config::server::upsert_server_env(&db.pool, &server_id, env).await;
    }
    let _ = create_server_metadata(db, &server_id, "Imported via API").await;

    // Add to default config suit
    if let Ok(suit_id) = get_or_create_default_config_suit(db).await {
        let _ = add_server_to_suit_with_sync(state, db, &suit_id, &server_id, true).await;
    }

    Ok(())
}

/// Import servers from JSON configuration
pub async fn import_servers(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ImportServersRequest>,
) -> Result<Json<ImportServersResponse>, ApiError> {
    let db = common::get_database_from_state(&state)?;

    let mut imported_servers = Vec::new();
    let mut failed_servers = Vec::new();
    let mut error_details = HashMap::new();

    // Process each server in the payload
    for (name, config) in payload.mcp_servers {
        match import_single_server(&state, &db, name.clone(), config).await {
            Ok(()) => {
                tracing::info!("Successfully imported server '{}'", name);
                imported_servers.push(name);
            }
            Err(error_msg) => {
                tracing::error!("Failed to import server '{}': {}", name, error_msg);
                failed_servers.push(name.clone());
                error_details.insert(name, error_msg);
            }
        }
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

/// Disconnect server instances from connection pool
async fn disconnect_server_instances(
    state: &Arc<AppState>,
    name: &str,
) {
    let pool_result = tokio::time::timeout(std::time::Duration::from_secs(1), state.connection_pool.lock()).await;

    if let Ok(mut pool) = pool_result {
        if let Some(instances) = pool.connections.get(name) {
            let instance_ids: Vec<String> = instances.keys().cloned().collect();
            for instance_id in instance_ids {
                if let Err(e) = pool.disconnect(name, &instance_id).await {
                    tracing::warn!(
                        "Failed to disconnect instance '{}' of server '{}': {}",
                        instance_id,
                        name,
                        e
                    );
                }
            }
        }
    } else {
        tracing::warn!("Timed out waiting for connection pool lock, proceeding with server deletion anyway");
    }
}

/// Delete server-related records from database
async fn delete_server_records(
    db: &Database,
    server_id: &str,
) -> Result<(), ApiError> {
    let mut tx = db.pool.begin().await.map_err(db_error)?;

    // Option 1: Use CASCADE DELETE (recommended)
    // Since all tables have proper ON DELETE CASCADE constraints,
    // we can simply delete from server_config and let the database handle the rest
    sqlx::query("DELETE FROM server_config WHERE id = ?")
        .bind(server_id)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;

    // The following tables will be automatically cleaned up by CASCADE DELETE:
    // - server_tools (has FK to server_config.id)
    // - server_args (has FK to server_config.id)
    // - server_env (has FK to server_config.id)
    // - server_meta (has FK to server_config.id)
    // - config_suit_server (has FK to server_config.id)
    // - config_suit_resource (has FK to server_config.id)
    // - config_suit_prompt (has FK to server_config.id)
    // - config_suit_tool (has FK to server_tools.id, which cascades from server_config)

    tx.commit().await.map_err(db_error)?;
    Ok(())
}

/// Delete an existing MCP server
pub async fn delete_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<OperationResponse>, ApiError> {
    let db = common::get_database_from_state(&state)?;

    // Get existing server
    let existing_server = get_existing_server_or_error(&db, &name).await?;
    let server_id = existing_server
        .id
        .clone()
        .ok_or_else(|| internal_error("Server ID not found"))?;

    // Disconnect server instances
    disconnect_server_instances(&state, &name).await;

    // Delete all server-related records
    delete_server_records(&db, &server_id).await?;

    tracing::info!("Successfully deleted server '{}'", name);

    // Return success response
    Ok(Json(OperationResponse {
        id: server_id,
        name,
        result: "Successfully deleted server".to_string(),
        status: "Deleted".to_string(),
        allowed_operations: Vec::new(),
    }))
}
