// MCPMate Proxy API handlers for MCP server CRUD operations
// Contains handler functions for creating, updating, and importing servers

use std::collections::HashMap;

use super::{common::*, instance::list_instances};
use crate::{
    api::{handlers::ApiError, models::server::ServerMetaInfo},
    common::{config::ConfigSuitType, server::ServerType},
    config::{
        database::Database,
        models::{ConfigSuit, ServerMeta},
        server::{self},
        suit,
    },
};

// Private helper functions

/// Get database reference from AppState
fn get_database_from_state(state: &Arc<AppState>) -> Result<Arc<Database>, ApiError> {
    state
        .http_proxy
        .as_ref()
        .and_then(|p| p.database.clone())
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))
}

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
                return Err(ApiError::BadRequest(format!(
                    "URL is required for {kind} servers"
                )));
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
        suit::upsert_config_suit(&db.pool, &new_suit)
            .await
            .map_err(db_error)
    }
}

/// Server details for response building
#[derive(Default)]
struct ServerDetails {
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    meta: Option<ServerMetaInfo>,
    globally_enabled: bool,
    enabled_in_suits: bool,
}

/// Get complete server details
async fn get_server_details(
    db: &Database,
    server_id: &str,
    _name: &str,
) -> ServerDetails {
    let mut details = ServerDetails::default();

    // Get server arguments
    if let Ok(server_args) = crate::config::server::get_server_args(&db.pool, server_id).await {
        if !server_args.is_empty() {
            let mut sorted_args: Vec<_> = server_args.into_iter().collect();
            sorted_args.sort_by_key(|arg| arg.arg_index);
            details.args = Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect());
        }
    }

    // Get server environment variables
    if let Ok(env_map) = crate::config::server::get_server_env(&db.pool, server_id).await {
        if !env_map.is_empty() {
            details.env = Some(env_map);
        }
    }

    // Get server metadata
    if let Ok(Some(server_meta)) = crate::config::server::get_server_meta(&db.pool, server_id).await
    {
        details.meta = Some(ServerMetaInfo {
            description: server_meta.description,
            author: server_meta.author,
            website: server_meta.website,
            repository: server_meta.repository,
            category: server_meta.category,
            recommended_scenario: server_meta.recommended_scenario,
            rating: server_meta.rating,
        });
    }

    // Get server global enabled status
    details.globally_enabled = crate::config::server::get_server_global_status(&db.pool, server_id)
        .await
        .unwrap_or(Some(true))
        .unwrap_or(true);

    // Get server enabled status in config suits
    details.enabled_in_suits =
        crate::config::server::is_server_enabled_in_any_suit(&db.pool, server_id)
            .await
            .unwrap_or(false);

    details
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
    let db = get_database_from_state(&state)?;

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
    let db = get_database_from_state(&state)?;

    // Get existing server
    let existing_server = get_existing_server_or_error(&db, &name).await?;
    let server_id = existing_server
        .id
        .clone()
        .ok_or_else(|| internal_error("Server ID not found"))?;

    // Validate server type if provided
    if let Some(kind) = &payload.kind {
        let command = payload
            .command
            .as_ref()
            .or(existing_server.command.as_ref());
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
            ServerType::StreamableHttp => {
                Some(crate::common::server::TransportType::StreamableHttp)
            }
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
        Err(_) => Vec::new(),
    };

    // Get server details
    let details = get_server_details(&db, &server_id, &name).await;

    // Return success response
    Ok(Json(ServerResponse {
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
        instances: instances_response,
    }))
}

/// Helper function to import a single server
async fn import_single_server(
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
    validate_server_config(&config.kind, &config.command, &config.url)
        .map_err(|e| e.to_string())?;
    let server = create_server_from_config(
        name.clone(),
        &config.kind,
        config.command.clone(),
        config.url.clone(),
    );

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
        let _ = add_server_to_suit(db, &suit_id, &server_id, true).await;
    }

    Ok(())
}

/// Import servers from JSON configuration
pub async fn import_servers(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ImportServersRequest>,
) -> Result<Json<ImportServersResponse>, ApiError> {
    let db = get_database_from_state(&state)?;

    let mut imported_servers = Vec::new();
    let mut failed_servers = Vec::new();
    let mut error_details = HashMap::new();

    // Process each server in the payload
    for (name, config) in payload.mcp_servers {
        match import_single_server(&db, name.clone(), config).await {
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
    let pool_result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        state.connection_pool.lock(),
    )
    .await;

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
        tracing::warn!(
            "Timed out waiting for connection pool lock, proceeding with server deletion anyway"
        );
    }
}

/// Delete server-related records from database
async fn delete_server_records(
    db: &Database,
    server_id: &str,
) -> Result<(), ApiError> {
    let mut tx = db.pool.begin().await.map_err(db_error)?;

    // Delete from all config suits
    let config_suits = crate::config::suit::get_all_config_suits(&db.pool)
        .await
        .map_err(db_error)?;
    for suit in config_suits {
        if let Some(suit_id) = &suit.id {
            // Delete server from config suit
            sqlx::query(
                "DELETE FROM config_suit_server WHERE config_suit_id = ? AND server_id = ?",
            )
            .bind(suit_id)
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;

            // Delete server tools from config suit
            sqlx::query("DELETE FROM config_suit_tool WHERE config_suit_id = ? AND server_id = ?")
                .bind(suit_id)
                .bind(server_id)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
    }

    // Delete server metadata, env, args, and config
    sqlx::query("DELETE FROM server_meta WHERE server_id = ?")
        .bind(server_id)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    sqlx::query("DELETE FROM server_env WHERE server_id = ?")
        .bind(server_id)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    sqlx::query("DELETE FROM server_args WHERE server_id = ?")
        .bind(server_id)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    sqlx::query("DELETE FROM server_config WHERE id = ?")
        .bind(server_id)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;

    tx.commit().await.map_err(db_error)?;
    Ok(())
}

/// Delete an existing MCP server
pub async fn delete_server(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<OperationResponse>, ApiError> {
    let db = get_database_from_state(&state)?;

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
