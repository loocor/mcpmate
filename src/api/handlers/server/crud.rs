// MCPMate Proxy API handlers for MCP server CRUD operations
// Contains handler functions for creating, updating, and importing servers

use super::{common, shared::*};
use crate::api::models::server::{
    ServerCreateReq, ServerDeleteReq, ServerDetailsData, ServerDetailsResp, ServerUpdateReq, ServersImportData,
    ServersImportReq,
};
use crate::{
    api::handlers::{
        ApiError,
        common::{internal_error, map_anyhow_error, map_database_error},
    },
    common::{profile::ProfileType, server::ServerType},
    config::server::capabilities::sync_via_connection_pool,
    config::server::{ConflictPolicy, ImportOptions, import_batch},
    config::{
        database::Database,
        models::{Profile, ServerMeta},
        profile,
        server::{self},
    },
};
use axum::{Json, extract::State};
use std::str::FromStr;
use std::sync::Arc;

/// Validate server configuration
#[inline]
fn validate_server_config(
    kind: &str,
    command: &Option<String>,
    url: &Option<String>,
) -> Result<(), ApiError> {
    match kind {
        "stdio" if command.is_none() => Err(ApiError::BadRequest("Command is required for stdio servers".to_owned())),
        "sse" | "streamable_http" if url.is_none() => {
            Err(ApiError::BadRequest(format!("URL is required for {kind} servers")))
        }
        "stdio" | "sse" | "streamable_http" => Ok(()),
        _ => Err(ApiError::BadRequest(format!(
            "Invalid server type: {kind}. Must be one of: stdio, sse, streamable_http"
        ))),
    }
}

/// Create server model from configuration using strict ServerType enum
#[inline]
fn create_server_from_config(
    name: String,
    kind: ServerType,
    command: Option<String>,
    url: Option<String>,
) -> Server {
    match kind {
        ServerType::Stdio => Server::new_stdio(name, command),
        ServerType::Sse => Server::new_sse(name, url),
        ServerType::StreamableHttp => Server::new_streamable_http(name, url),
    }
}

/// Get or create default profile
async fn get_or_create_default_profile(db: &Database) -> Result<String, ApiError> {
    let default_profile = profile::get_profile_by_name(&db.pool, "default")
        .await
        .map_err(map_anyhow_error)?;

    match default_profile {
        Some(profile) => profile.id.ok_or_else(|| internal_error("Profile id missing")),
        None => {
            let new_profile = Profile::new("default".to_owned(), ProfileType::Shared);
            profile::upsert_profile(&db.pool, &new_profile)
                .await
                .map_err(map_anyhow_error)
        }
    }
}

/// Add server to profile
#[inline]
async fn add_server_to_profile(
    db: &Database,
    profile_id: &str,
    server_id: &str,
    enabled: bool,
) -> Result<(), ApiError> {
    profile::add_server_to_profile(&db.pool, profile_id, server_id, enabled)
        .await
        .map_err(map_anyhow_error)
        .map(|_| ())
}

/// Add server to profile with capabilities sync
async fn add_server_to_profile_with_sync(
    _state: &Arc<AppState>,
    db: &Database,
    profile_id: &str,
    server_id: &str,
    enabled: bool,
) -> Result<(), ApiError> {
    // Add server to profile
    profile::add_server_to_profile(&db.pool, profile_id, server_id, enabled)
        .await
        .map_err(map_anyhow_error)?;

    // Sync server capabilities to the profile (async, non-blocking)
    if false {
        let pool_clone = db.pool.clone();
        let profile_id_clone = profile_id.to_string();
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
                        "Too many concurrent capability sync operations. Skipping sync for server {} to profile {}",
                        server_id_clone,
                        profile_id_clone
                    );
                    return;
                }
            };

            if let Err(e) = crate::config::profile::sync_server_capabilities(
                &pool_clone,
                &profile_id_clone,
                &server_id_clone,
                crate::config::profile::ServerCapabilityAction::Add,
            )
            .await
            {
                tracing::warn!(
                    "Failed to sync capabilities for server {} to profile {}: {}",
                    server_id_clone,
                    profile_id_clone,
                    e
                );
            } else {
                tracing::debug!(
                    "Successfully synced capabilities for server {} to profile {}",
                    server_id_clone,
                    profile_id_clone
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
        server_id: server_id.to_owned(),
        description: Some(description.to_owned()),
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
        .map_err(map_anyhow_error)
        .map(|_| ())
}

/// Create a new MCP server configuration
///
/// This endpoint creates a new MCP server configuration. Server types must strictly use the following standard formats:
/// - `"stdio"`: Standard input/output server, launched via command line
/// - `"sse"`: Server-Sent Events server, connected via HTTP SSE
/// - `"streamable_http"`: Streamable HTTP server, connected via HTTP streaming
///
/// **Important**: The system will reject any non-standard formats such as "http", "streamable-http", "streamableHttp", etc.
///
/// **Endpoint**: `POST /mcp/servers/create`
///
/// # Parameters
/// - `payload`: Server creation request containing server name, type, command or URL, etc.
///
/// # Returns
/// - Success: Returns detailed information of the created server
/// - Failure: Returns specific error information and correction suggestions
///
/// # Error Handling
/// - 400 Bad Request: Server type format is incorrect or configuration is invalid
/// - 409 Conflict: Server name already exists
/// - 500 Internal Server Error: Database operation failed
///
/// # Server Type Validation
/// The system will strictly validate server type formats. Any input that does not conform to standards will be rejected with detailed error information.
pub async fn create_server(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ServerCreateReq>,
) -> Result<Json<ServerDetailsResp>, ApiError> {
    let db = common::get_database_from_state(&state)?;

    // Check if server already exists
    if crate::config::server::get_server(&db.pool, &payload.name)
        .await
        .map_err(map_anyhow_error)?
        .is_some()
    {
        return Err(ApiError::Conflict(format!(
            "Server with name '{}' already exists. Please choose a different name for your server.",
            payload.name
        )));
    }

    // Strictly validate server type format
    let server_type = ServerType::from_str(&payload.server_type).map_err(|_| {
        ApiError::BadRequest(format!(
            "Invalid server type '{}'.\n\nCorrect format requirements:\n\
                - Use \"stdio\" (not \"Stdio\" or other variants)\n\
                - Use \"sse\" (not \"SSE\" or other variants)\n\
                - Use \"streamable_http\" (not \"http\", \"streamable-http\", or \"streamableHttp\")\n\n\
                Please check your input and use the correct standard format.",
            payload.server_type
        ))
    })?;

    // Validate server configuration
    validate_server_config(&payload.server_type, &payload.command, &payload.url)?;

    // Create server model using validated ServerType
    let mut server = create_server_from_config(
        payload.name.clone(),
        server_type,
        payload.command.clone(),
        payload.url.clone(),
    );
    server.registry_server_id = payload.registry_server_id.clone();

    // Insert server into database
    let server_id = crate::config::server::upsert_server(&db.pool, &server)
        .await
        .map_err(map_anyhow_error)?;

    // Insert server arguments if provided
    if let Some(args) = &payload.args {
        crate::config::server::upsert_server_args(&db.pool, &server_id, args)
            .await
            .map_err(map_anyhow_error)?;
    }

    // Insert server environment variables if provided
    if let Some(env) = &payload.env {
        crate::config::server::upsert_server_env(&db.pool, &server_id, env)
            .await
            .map_err(map_anyhow_error)?;
    }

    // Create server metadata
    create_server_metadata(&db, &server_id, "Created via API").await?;

    // Add server to default profile if enabled
    let enabled = payload.enabled.unwrap_or(true);
    if enabled {
        let profile_id = get_or_create_default_profile(&db).await?;
        add_server_to_profile(&db, &profile_id, &server_id, true).await?;
        tracing::info!("Enabled server '{}' in default profile", payload.name);
    }

    // Initial capability discovery + dual write (SQLite shadow + REDB)
    let _ = sync_via_connection_pool(
        &state.connection_pool,
        &state.redb_cache,
        &db.pool,
        &server_id,
        &payload.name,
        10,
    )
    .await;

    // Return success response
    let now = chrono::Utc::now();
    Ok(Json(ServerDetailsResp::success(ServerDetailsData {
        id: Some(server_id),
        name: payload.name.clone(),
        registry_server_id: payload.registry_server_id.clone(),
        enabled,
        globally_enabled: true,
        enabled_in_profile: enabled,
        server_type: payload.server_type.parse().unwrap_or(ServerType::Stdio),
        command: payload.command.clone(),
        url: payload.url.clone(),
        args: payload.args.clone(),
        env: payload.env.clone(),
        meta: None,
        created_at: Some(now.to_rfc3339()),
        updated_at: Some(now.to_rfc3339()),
        instances: Vec::new(),
    })))
}

/// Update an existing MCP server configuration
///
/// This endpoint updates an existing MCP server configuration. If updating the server type, it must strictly use standard formats:
/// - `"stdio"`: Standard input/output server
/// - `"sse"`: Server-Sent Events server
/// - `"streamable_http"`: Streamable HTTP server
///
/// **Important**: The system will reject any non-standard server type formats
///
/// **Endpoint**: `POST /mcp/servers/update`
///
/// # Parameters
/// - `payload`: Server update request containing fields to be updated
///
/// # Returns
/// - Success: Returns detailed information of the updated server
/// - Failure: Returns specific error information and correction suggestions
///
/// # Error Handling
/// - 400 Bad Request: Server type format is incorrect or configuration is invalid
/// - 404 Not Found: The specified server does not exist
/// - 500 Internal Server Error: Database operation failed
pub async fn update_server(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ServerUpdateReq>,
) -> Result<Json<ServerDetailsResp>, ApiError> {
    let db = common::get_database_from_state(&state)?;

    let id = payload.id.clone();

    // Get existing server by ID
    let existing_server = crate::config::server::get_server_by_id(&db.pool, &id)
        .await
        .map_err(map_anyhow_error)?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{id}' not found")))?;
    let server_id = existing_server
        .id
        .clone()
        .ok_or_else(|| internal_error("Server ID not found"))?;

    // Strictly validate server type format (if provided)
    let validated_server_type = if let Some(ref kind) = payload.kind {
        let server_type = ServerType::from_str(kind).map_err(|_| {
            ApiError::BadRequest(format!(
                "Invalid server type '{}'.\n\nCorrect format requirements:\n\
                    - Use \"stdio\" (not \"Stdio\" or other variants)\n\
                    - Use \"sse\" (not \"SSE\" or other variants)\n\
                    - Use \"streamable_http\" (not \"http\", \"streamable-http\", or \"streamableHttp\")\n\n\
                    Please check your input and use the correct standard format.",
                kind
            ))
        })?;

        let command = payload.command.as_ref().or(existing_server.command.as_ref());
        let url = payload.url.as_ref().or(existing_server.url.as_ref());
        validate_server_config(kind, &command.cloned(), &url.cloned())?;

        Some(server_type)
    } else {
        None
    };

    // Create updated server model
    let mut updated_server = existing_server.clone();

    if let Some(server_type) = validated_server_type {
        updated_server.server_type = server_type;
        updated_server.transport_type = Some(match server_type {
            ServerType::Stdio => crate::common::server::TransportType::Stdio,
            ServerType::Sse => crate::common::server::TransportType::Sse,
            ServerType::StreamableHttp => crate::common::server::TransportType::StreamableHttp,
        });
    }

    if let Some(command) = payload.command {
        updated_server.command = Some(command);
    }

    if let Some(url) = payload.url {
        updated_server.url = Some(url);
    }

    if let Some(registry_id) = payload.registry_server_id.clone() {
        updated_server.registry_server_id = Some(registry_id);
    }

    // Update server in database
    crate::config::server::upsert_server(&db.pool, &updated_server)
        .await
        .map_err(map_anyhow_error)?;

    // Update server arguments if provided
    if let Some(args) = &payload.args {
        crate::config::server::upsert_server_args(&db.pool, &server_id, args)
            .await
            .map_err(map_anyhow_error)?;
    }

    // Update server environment variables if provided
    if let Some(env) = &payload.env {
        crate::config::server::upsert_server_env(&db.pool, &server_id, env)
            .await
            .map_err(map_anyhow_error)?;
    }

    // Update server enabled status if provided
    if let Some(enabled) = payload.enabled {
        let profile_id = get_or_create_default_profile(&db).await?;
        add_server_to_profile_with_sync(&state, &db, &profile_id, &server_id, enabled).await?;
        tracing::info!(
            "Updated server '{}' enabled status to {} in default profile",
            existing_server.name,
            enabled
        );
    }

    // Get server details via shared helper
    let details = common::get_complete_server_details(&db.pool, &server_id, &existing_server.name, &state).await;

    // Return success response
    Ok(Json(ServerDetailsResp::success(ServerDetailsData {
        id: Some(server_id),
        name: existing_server.name,
        registry_server_id: updated_server.registry_server_id.clone(),
        enabled: details.globally_enabled && details.enabled_in_profile,
        globally_enabled: details.globally_enabled,
        enabled_in_profile: details.enabled_in_profile,
        server_type: updated_server.server_type,
        command: updated_server.command.clone(),
        url: updated_server.url.clone(),
        args: details.args,
        env: details.env,
        meta: details.meta,
        created_at: updated_server.created_at.map(|dt| dt.to_rfc3339()),
        updated_at: updated_server.updated_at.map(|dt| dt.to_rfc3339()),
        instances: details.instances,
    })))
}

/// Import servers from JSON configuration (now uses unified core)
///
/// **Endpoint:** `POST /mcp/servers/import`
pub async fn import_servers(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ServersImportReq>,
) -> Result<Json<ServersImportData>, ApiError> {
    let db = common::get_database_from_state(&state)?;

    // Use safer dedup strategy by default: name + fingerprint, skip on conflict
    let outcome = import_batch(
        &db.pool,
        &state.connection_pool,
        &state.redb_cache,
        payload.mcp_servers,
        ImportOptions {
            by_name: true,
            by_fingerprint: true,
            conflict_policy: ConflictPolicy::Skip,
            preview: false,
            target_profile: None,
        },
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(ServersImportData {
        imported_count: outcome.imported.len(),
        imported_servers: outcome.imported.into_iter().map(|s| s.name).collect(),
        failed_servers: outcome.failed.keys().cloned().collect(),
        error_details: if outcome.failed.is_empty() {
            None
        } else {
            Some(outcome.failed)
        },
    }))
}

/// Disconnect server instances from connection pool
async fn disconnect_server_instances(
    state: &Arc<AppState>,
    name: &str,
) {
    let mut pool =
        match crate::api::handlers::server::common::ConnectionPoolManager::get_pool_for_health_check(state).await {
            Ok(pool) => pool,
            Err(_) => {
                tracing::warn!("Failed to get connection pool, proceeding with server deletion anyway");
                return;
            }
        };

    let Some(instances) = pool.connections.get(name) else {
        return;
    };

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

/// Delete server-related records from database
async fn delete_server_records(
    db: &Database,
    server_id: &str,
) -> Result<(), ApiError> {
    let mut tx = db.pool.begin().await.map_err(map_database_error)?;

    // Option 1: Use CASCADE DELETE (recommended)
    // Since all tables have proper ON DELETE CASCADE constraints,
    // we can simply delete from server_config and let the database handle the rest
    sqlx::query("DELETE FROM server_config WHERE id = ?")
        .bind(server_id)
        .execute(&mut *tx)
        .await
        .map_err(map_database_error)?;

    // The following tables will be automatically cleaned up by CASCADE DELETE:
    // - server_tools (has FK to server_config.id)
    // - server_args (has FK to server_config.id)
    // - server_env (has FK to server_config.id)
    // - server_meta (has FK to server_config.id)
    // - profile_server (has FK to server_config.id)
    // - profile_resource (has FK to server_config.id)
    // - profile_prompt (has FK to server_config.id)
    // - profile_tool (has FK to server_tools.id, which cascades from server_config)

    tx.commit().await.map_err(map_database_error)?;
    Ok(())
}

/// Delete an existing MCP server (updated for payload parameters)
///
/// **Endpoint:** `DELETE /mcp/servers/delete`
pub async fn delete_server(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ServerDeleteReq>,
) -> Result<Json<ServerOperationData>, ApiError> {
    let db = common::get_database_from_state(&state)?;

    let id = request.id;

    // Get existing server by ID
    let existing_server = crate::config::server::get_server_by_id(&db.pool, &id)
        .await
        .map_err(map_anyhow_error)?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{id}' not found")))?;
    let server_id = existing_server
        .id
        .clone()
        .ok_or_else(|| internal_error("Server ID not found"))?;

    // Disconnect server instances
    disconnect_server_instances(&state, &existing_server.name).await;

    // Delete all server-related records
    delete_server_records(&db, &server_id).await?;

    // Remove capability cache (REDB) for this server
    if let Err(e) = state.redb_cache.remove_server_data(&server_id).await {
        tracing::warn!("Failed to remove REDB cache for server '{}': {}", server_id, e);
    }

    // Remove resolver mapping to keep id<->name cache consistent
    crate::core::capability::resolver::remove_by_id(&server_id).await;

    tracing::info!("Successfully deleted server '{}'", existing_server.name);

    // Return success response
    Ok(Json(ServerOperationData {
        id: server_id,
        name: existing_server.name,
        result: "Successfully deleted server".to_string(),
        status: "Deleted".to_string(),
        allowed_operations: Vec::new(),
    }))
}
