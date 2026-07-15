// MCPMate Proxy API handlers for MCP server CRUD operations
// Contains handler functions for creating, updating, and importing servers

use super::{common, shared::*};
use crate::api::models::server::{
    ServerCreateReq, ServerDeleteReq, ServerDetailsData, ServerDetailsResp, ServerMetaPayload,
    ServerNamespaceRemediationReq, ServerOperationData, ServerOperationResp, ServerUpdateReq, ServersImportData,
    ServersImportReq, ServersImportResp, SkippedServerData,
};
use crate::{
    api::handlers::{
        ApiError,
        common::{internal_error, map_anyhow_error, map_database_error},
    },
    common::server::ServerType,
    config::server::capabilities::sync_via_connection_pool,
    config::server::{ImportOptions, ImportOutcome, SkippedServer, import::server_meta_from_payload, import_batch},
    config::server::{
        get_server_headers, headers::has_non_empty_authorization_header, merge_env_for_update, merge_headers_for_update,
    },
    config::server::{replace_server_headers, upsert_server_headers},
    config::{
        database::Database,
        profile,
        server::{self},
    },
    core::secrets::{mcp_config_from_server, sync_server_secret_usages},
};
use axum::{Json, extract::State};
use serde_json::{Map, Value};
use std::sync::Arc;
use std::{
    collections::{BTreeSet, HashMap},
    str::FromStr,
};

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

pub async fn remediate_server_namespace(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ServerNamespaceRemediationReq>,
) -> Result<Json<ServerOperationResp>, ApiError> {
    let db = common::get_database_from_state(&state)?;
    crate::config::server::validate_server_namespace(&payload.namespace)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;

    let issue = common::load_namespace_issue(&db.pool, &payload.id)
        .await
        .map_err(map_anyhow_error)?;
    if issue.is_none() {
        return Err(ApiError::Conflict(
            "Namespace remediation is available only for servers with an unresolved namespace issue".to_string(),
        ));
    }
    if let Some(owner) = crate::config::server::get_server(&db.pool, &payload.namespace)
        .await
        .map_err(map_anyhow_error)?
        .filter(|server| server.id.as_deref() != Some(payload.id.as_str()))
    {
        return Err(ApiError::Conflict(format!(
            "Namespace '{}' is already used by server '{}'",
            payload.namespace,
            owner.id.unwrap_or(owner.name)
        )));
    }

    let secret_store = state.secret_store.read().await.clone();
    let (server, server_config) =
        crate::core::foundation::loader::load_server_config_strict(db.as_ref(), &payload.id, secret_store)
            .await
            .map_err(map_anyhow_error)?;
    let mut snapshot =
        crate::config::server::capabilities::discover_from_config(&payload.id, &server_config, server.server_type)
            .await
            .map_err(map_anyhow_error)?;

    if let Err(error) = crate::config::server::namespace_repair::remediate_namespace_with_snapshot(
        &db.pool,
        &payload.id,
        &payload.namespace,
        &mut snapshot,
    )
    .await
    {
        if error
            .downcast_ref::<crate::core::capability::naming::ExternalIdentifierCollision>()
            .is_some()
        {
            crate::config::server::namespace_repair::record_capability_collision_from_error(&db.pool, &error)
                .await
                .map_err(map_anyhow_error)?;
            return Err(ApiError::Conflict(error.to_string()));
        }
        return Err(map_anyhow_error(error));
    }

    let mut pending_steps = Vec::new();
    if let Err(error) = crate::config::server::capabilities::apply_discovered_snapshot(
        &db.pool,
        &state.redb_cache,
        &payload.id,
        &payload.namespace,
        &snapshot,
        true,
    )
    .await
    {
        if let Err(record_error) =
            crate::config::server::namespace_repair::record_capability_collision_from_error(&db.pool, &error).await
        {
            tracing::warn!(
                server_id = %payload.id,
                error = %record_error,
                "Failed to record post-commit capability collision"
            );
        }
        pending_steps.push(format!("capability cache refresh: {error}"));
    }

    let pool_sync_result = {
        let mut pool = state.connection_pool.lock().await;
        pool.sync_servers_from_active_profile().await
    };
    if let Err(error) = pool_sync_result {
        pending_steps.push(format!("connection pool refresh: {error}"));
    }

    if !pending_steps.is_empty() {
        tracing::warn!(
            server_id = %payload.id,
            pending_steps = %pending_steps.join("; "),
            "Namespace remediation committed, but runtime convergence is pending"
        );
    }

    let (result, status) = if pending_steps.is_empty() {
        ("Namespace remediated", "remediated")
    } else {
        (
            "Namespace remediated; runtime convergence pending",
            "remediated_pending",
        )
    };

    Ok(Json(ServerOperationResp::success(ServerOperationData {
        id: payload.id,
        name: payload.namespace,
        result: result.to_string(),
        status: status.to_string(),
        allowed_operations: Vec::new(),
    })))
}

async fn clear_oauth_auth_source_for_manual_authorization(
    state: &Arc<AppState>,
    db: &Database,
    server_id: &str,
    headers: &HashMap<String, String>,
) -> Result<(), ApiError> {
    if !has_non_empty_authorization_header(headers) {
        return Ok(());
    }

    delete_oauth_secrets_for_server_best_effort(state, db, server_id).await?;
    crate::config::server::delete_server_oauth_config(&db.pool, server_id)
        .await
        .map_err(map_anyhow_error)?;
    crate::config::server::delete_server_oauth_token(&db.pool, server_id)
        .await
        .map_err(map_anyhow_error)?;
    Ok(())
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

async fn upsert_meta_payload(
    db: &Database,
    server_id: &str,
    payload: &ServerMetaPayload,
) -> Result<(), ApiError> {
    let mut meta = server_meta_from_payload(server_id, payload).map_err(|err| ApiError::BadRequest(err.to_string()))?;
    meta.server_version = None;
    meta.protocol_version = None;

    server::upsert_server_meta(&db.pool, &meta)
        .await
        .map_err(map_anyhow_error)
        .map(|_| ())
}

async fn sync_secret_usages_for_server(
    state: &Arc<AppState>,
    db: &Database,
    server_id: &str,
    server: &Server,
) -> Result<(), ApiError> {
    let Some(secret_store) = state.secret_store.read().await.clone() else {
        return Ok(());
    };

    let config = mcp_config_from_server(&db.pool, server_id, server)
        .await
        .map_err(map_anyhow_error)?;

    sync_server_secret_usages(secret_store.as_ref(), server_id, &config)
        .await
        .map_err(map_anyhow_error)
}

async fn delete_oauth_secrets_for_server(
    state: &Arc<AppState>,
    db: &Database,
    server_id: &str,
) -> anyhow::Result<()> {
    let manager =
        crate::core::oauth::OAuthManager::new_optional_store(db.pool.clone(), state.secret_store.read().await.clone());

    manager.delete_all_oauth_secrets(server_id).await
}

async fn delete_oauth_secret_rows_for_server(
    db: &Database,
    server_id: &str,
) -> anyhow::Result<()> {
    use crate::core::oauth::manager::{OAuthSecretSlot, oauth_secret_alias};

    for slot in [
        OAuthSecretSlot::ClientSecret,
        OAuthSecretSlot::AccessToken,
        OAuthSecretSlot::RefreshToken,
    ] {
        let alias = oauth_secret_alias(server_id, slot);
        sqlx::query("DELETE FROM secure_store_secrets WHERE alias = ?")
            .bind(alias)
            .execute(&db.pool)
            .await?;
    }
    Ok(())
}

fn is_oauth_secret_cleanup_unavailable(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<crate::core::oauth::manager::OAuthSecretCleanupUnavailable>()
        .is_some()
}

async fn delete_oauth_secrets_for_server_best_effort(
    state: &Arc<AppState>,
    db: &Database,
    server_id: &str,
) -> Result<(), ApiError> {
    match delete_oauth_secrets_for_server(state, db, server_id).await {
        Ok(()) => Ok(()),
        Err(error) if is_oauth_secret_cleanup_unavailable(&error) => {
            tracing::warn!(
                server_id,
                error = %error,
                "Deleting OAuth secret metadata without decrypting because Secure Store is unavailable during server deletion"
            );
            delete_oauth_secret_rows_for_server(db, server_id)
                .await
                .map_err(map_anyhow_error)?;
            Ok(())
        }
        Err(error) => Err(map_anyhow_error(error)),
    }
}

/// Create a new MCP server configuration
///
/// This endpoint creates a new MCP server configuration. Server types must strictly use the following standard formats:
/// - `"stdio"`: Standard input/output server, launched via command line
/// - `"sse"`: Legacy SSE HTTP server (persisted; protocol uses Streamable HTTP via rmcp)
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
    let started_at = std::time::Instant::now();
    let db = common::get_database_from_state(&state)?;
    let is_pending_import = payload.pending_import.unwrap_or(false);

    crate::config::server::validate_server_namespace(&payload.name)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;

    let existing_server = crate::config::server::get_server(&db.pool, &payload.name)
        .await
        .map_err(map_anyhow_error)?;
    let reusable_pending_server = existing_server
        .as_ref()
        .filter(|server| is_pending_import && server.pending_import);
    let reusable_pending_server_id = reusable_pending_server.and_then(|server| server.id.clone());
    if existing_server.is_some() && reusable_pending_server.is_none() {
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
                - Use \"sse\" or \"streamable_http\" (lowercase; not \"http\", \"streamable-http\", or \"streamableHttp\")\n\n\
                Please check your input and use the correct standard format.",
            payload.server_type
        ))
    })?;

    // Validate server configuration
    validate_server_config(&payload.server_type, &payload.command, &payload.url)?;

    if let Some(server_id) = reusable_pending_server_id.as_deref() {
        delete_oauth_secrets_for_server_best_effort(&state, &db, server_id).await?;
        crate::config::server::delete_server_oauth_config(&db.pool, server_id)
            .await
            .map_err(map_anyhow_error)?;
        crate::config::server::delete_server_oauth_token(&db.pool, server_id)
            .await
            .map_err(map_anyhow_error)?;
    }

    // Create server model using validated ServerType
    let mut server = create_server_from_config(
        payload.name.clone(),
        server_type,
        payload.command.clone(),
        payload.url.clone(),
    );
    if let Some(existing) = reusable_pending_server {
        server.id = existing.id.clone();
    }
    server.source = payload.source.clone();
    server.pending_import = is_pending_import;
    server.unify_direct_exposure_eligible = payload.unify_direct_exposure_eligible.unwrap_or(false);
    if server.pending_import {
        server.enabled = crate::common::status::EnabledStatus::Disabled;
    }

    // Insert server into database
    let server_id = crate::config::server::upsert_server(&db.pool, &server)
        .await
        .map_err(map_anyhow_error)?;
    crate::core::capability::resolver::upsert(&server_id, &payload.name).await;

    // Persist default headers if provided
    if reusable_pending_server.is_some() {
        let empty_headers = std::collections::HashMap::new();
        let headers = payload.headers.as_ref().unwrap_or(&empty_headers);
        replace_server_headers(&db.pool, &server_id, headers)
            .await
            .map_err(map_anyhow_error)?;
        clear_oauth_auth_source_for_manual_authorization(&state, &db, &server_id, headers).await?;
    } else if let Some(headers) = &payload.headers {
        if !headers.is_empty() {
            upsert_server_headers(&db.pool, &server_id, headers)
                .await
                .map_err(map_anyhow_error)?;
            clear_oauth_auth_source_for_manual_authorization(&state, &db, &server_id, headers).await?;
        }
    }

    // Insert server arguments if provided
    if reusable_pending_server.is_some() || payload.args.is_some() {
        let empty_args: Vec<String> = Vec::new();
        let args = payload.args.as_ref().unwrap_or(&empty_args);
        crate::config::server::upsert_server_args(&db.pool, &server_id, args)
            .await
            .map_err(map_anyhow_error)?;
    }

    // Insert server environment variables if provided
    if reusable_pending_server.is_some() || payload.env.is_some() {
        let empty_env = std::collections::HashMap::new();
        let env = payload.env.as_ref().unwrap_or(&empty_env);
        crate::config::server::upsert_server_env(&db.pool, &server_id, env)
            .await
            .map_err(map_anyhow_error)?;
    }

    sync_secret_usages_for_server(&state, &db, &server_id, &server).await?;

    // Apply optional metadata payload
    if let Some(meta_payload) = payload.meta.as_ref() {
        upsert_meta_payload(&db, &server_id, meta_payload).await?;
    }

    // Associate server with specified profiles if provided
    let initial_enabled = payload.enabled.unwrap_or(true);

    if !server.pending_import
        && let Some(profile_ids) = payload.profile_ids.as_ref()
    {
        let mut unique_profiles = BTreeSet::new();
        for profile_id in profile_ids {
            if !unique_profiles.insert(profile_id) {
                continue;
            }

            let profile = crate::config::profile::get_profile(&db.pool, profile_id)
                .await
                .map_err(map_anyhow_error)?
                .ok_or_else(|| ApiError::NotFound(format!("Profile with ID '{}' not found", profile_id)))?;

            if !profile.is_active {
                return Err(ApiError::BadRequest(format!("Profile '{}' is not active", profile_id)));
            }

            add_server_to_profile(&db, profile_id, &server_id, initial_enabled).await?;
            tracing::info!(
                server_id = %server_id,
                profile_id = %profile_id,
                "Associated server '{}' with profile '{}' (enabled={})",
                payload.name,
                profile_id,
                initial_enabled
            );
        }
    }

    if !server.pending_import {
        let mut pool = state.connection_pool.lock().await;
        pool.sync_servers_from_active_profile()
            .await
            .map_err(map_anyhow_error)?;
    }

    // Initial capability discovery + dual write (SQLite shadow + REDB)
    if !server.pending_import {
        if let Err(error) = sync_via_connection_pool(
            &state.connection_pool,
            &state.redb_cache,
            &db.pool,
            &server_id,
            &payload.name,
            crate::config::server::capabilities::default_pool_lock_timeout_secs(),
        )
        .await
        {
            tracing::warn!(server_id = %server_id, error = %error, "Initial capability sync failed after server creation");
        }
    }

    let server_row = crate::config::server::get_server_by_id(&db.pool, &server_id)
        .await
        .map_err(map_anyhow_error)?
        .ok_or_else(|| internal_error("Server record missing after creation"))?;

    let server_name = server_row.name.clone();
    let source = server_row.source.clone();
    let command = server_row.command.clone();
    let url = server_row.url.clone();
    let server_type = server_row.server_type;
    let created_at = server_row.created_at.map(|dt| dt.to_rfc3339());
    let updated_at = server_row.updated_at.map(|dt| dt.to_rfc3339());

    let details = common::get_complete_server_details(&db.pool, &server_id, &server_name, &state).await;
    let effective_enabled = details.globally_enabled && details.enabled_in_profile;
    let oauth_summary = common::load_server_oauth_response_summary(
        &db.pool,
        &state,
        &server_id,
        server_row.server_type.is_http_transport(),
    )
    .await?;

    let audit_server_id = server_id.clone();
    let response = Json(ServerDetailsResp::success(ServerDetailsData {
        id: Some(server_id.clone()),
        name: server_name,
        source,
        enabled: effective_enabled,
        globally_enabled: details.globally_enabled,
        enabled_in_profile: details.enabled_in_profile,
        unify_direct_exposure_eligible: server_row.unify_direct_exposure_eligible,
        server_type,
        command,
        url,
        args: details.args,
        env: details.env,
        headers: None,
        meta: details.meta,
        server_info: details.server_info,
        capability: details.capability,
        protocol_version: details.protocol_version,
        created_at,
        updated_at,
        instances: details.instances,
        auth_mode: None,
        oauth_status: oauth_summary.oauth_status,
        oauth_custody_state: oauth_summary.oauth_custody_state,
        oauth_requires_reconnect: oauth_summary.oauth_requires_reconnect,
        oauth_issue: oauth_summary.oauth_issue,
        namespace_issue: common::load_namespace_issue(&db.pool, &server_id)
            .await
            .map_err(map_anyhow_error)?,
    }));

    let mut data = Map::new();
    data.insert("server_name".to_string(), Value::String(payload.name));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ServerCreate,
            crate::audit::AuditStatus::Success,
            "POST",
            "/api/mcp/servers/create",
            Some(started_at.elapsed().as_millis() as u64),
            Some(audit_server_id),
            None,
            Some(data),
            None,
        ),
    )
    .await;

    Ok(response)
}

/// Update an existing MCP server configuration
///
/// This endpoint updates an existing MCP server configuration. If updating the server type, it must strictly use standard formats:
/// - `"stdio"`: Standard input/output server
/// - `"sse"`: Legacy SSE HTTP server (persisted; protocol uses Streamable HTTP via rmcp)
/// - `"streamable_http"`: Streamable HTTP server
///
/// **Important**: The system will reject any non-standard server type formats.
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
    let started_at = std::time::Instant::now();
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
                    - Use \"sse\" or \"streamable_http\" (lowercase; not \"http\", \"streamable-http\", or \"streamableHttp\")\n\n\
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
    }

    if let Some(command) = payload.command {
        updated_server.command = Some(command);
    }

    if let Some(url) = payload.url {
        updated_server.url = Some(url);
    }

    if let Some(s) = payload.source {
        updated_server.source = Some(s);
    }

    if let Some(enabled) = payload.enabled {
        updated_server.enabled = enabled.into();
    }

    if let Some(unify_direct_exposure_eligible) = payload.unify_direct_exposure_eligible {
        updated_server.unify_direct_exposure_eligible = unify_direct_exposure_eligible;
    }

    if let Some(pending_import) = payload.pending_import {
        updated_server.pending_import = pending_import;
        if pending_import {
            updated_server.enabled = crate::common::status::EnabledStatus::Disabled;
        }
    }

    // Update server in database
    crate::config::server::upsert_server(&db.pool, &updated_server)
        .await
        .map_err(map_anyhow_error)?;

    // Replace default headers if provided
    if let Some(headers) = &payload.headers {
        let existing_headers = get_server_headers(&db.pool, &server_id)
            .await
            .map_err(map_anyhow_error)?;
        let merged_headers = merge_headers_for_update(headers, &existing_headers);
        replace_server_headers(&db.pool, &server_id, &merged_headers)
            .await
            .map_err(map_anyhow_error)?;
        clear_oauth_auth_source_for_manual_authorization(&state, &db, &server_id, &merged_headers).await?;
    }

    // Update server arguments if provided
    if let Some(args) = &payload.args {
        crate::config::server::upsert_server_args(&db.pool, &server_id, args)
            .await
            .map_err(map_anyhow_error)?;
    }

    // Update server environment variables if provided
    if let Some(env) = &payload.env {
        let existing_env = crate::config::server::get_server_env(&db.pool, &server_id)
            .await
            .map_err(map_anyhow_error)?;
        let merged_env = merge_env_for_update(env, &existing_env);
        crate::config::server::upsert_server_env(&db.pool, &server_id, &merged_env)
            .await
            .map_err(map_anyhow_error)?;
    }

    sync_secret_usages_for_server(&state, &db, &server_id, &updated_server).await?;

    if let Some(meta_payload) = payload.meta.as_ref() {
        upsert_meta_payload(&db, &server_id, meta_payload).await?;
    }

    // Update server enabled status if provided
    if let Some(profile_ids) = payload.profile_ids.as_ref() {
        let enabled_flag = payload.enabled.unwrap_or(true);
        let mut unique_profiles = BTreeSet::new();
        for profile_id in profile_ids {
            if !unique_profiles.insert(profile_id) {
                continue;
            }

            let profile = crate::config::profile::get_profile(&db.pool, profile_id)
                .await
                .map_err(map_anyhow_error)?
                .ok_or_else(|| ApiError::NotFound(format!("Profile with ID '{}' not found", profile_id)))?;

            if !profile.is_active {
                return Err(ApiError::BadRequest(format!("Profile '{}' is not active", profile_id)));
            }

            add_server_to_profile_with_sync(&state, &db, profile_id, &server_id, enabled_flag).await?;
            tracing::info!(
                server_id = %server_id,
                profile_id = %profile_id,
                "Updated server '{}' association in profile '{}' (enabled={})",
                existing_server.name,
                profile_id,
                enabled_flag
            );
        }
    }

    if existing_server.pending_import && !updated_server.pending_import {
        if let Err(error) = sync_via_connection_pool(
            &state.connection_pool,
            &state.redb_cache,
            &db.pool,
            &server_id,
            &existing_server.name,
            crate::config::server::capabilities::default_pool_lock_timeout_secs(),
        )
        .await
        {
            tracing::warn!(server_id = %server_id, error = %error, "Capability sync failed after completing server import");
        }
    }

    let direct_constraint_changed = existing_server.enabled.as_bool() != updated_server.enabled.as_bool()
        || existing_server.unify_direct_exposure_eligible != updated_server.unify_direct_exposure_eligible;

    if direct_constraint_changed {
        common::reconcile_client_direct_exposure_after_server_constraint_change(&state, &server_id).await?;
    }

    // Get server details via shared helper
    let details = common::get_complete_server_details(&db.pool, &server_id, &updated_server.name, &state).await;
    let oauth_summary = common::load_server_oauth_response_summary(
        &db.pool,
        &state,
        &server_id,
        updated_server.server_type.is_http_transport(),
    )
    .await?;

    // Return success response
    let audit_server_id = server_id.clone();
    let audit_server_name = updated_server.name.clone();
    let response = Json(ServerDetailsResp::success(ServerDetailsData {
        id: Some(server_id),
        name: updated_server.name.clone(),
        source: updated_server.source.clone(),
        enabled: details.globally_enabled && details.enabled_in_profile,
        globally_enabled: details.globally_enabled,
        enabled_in_profile: details.enabled_in_profile,
        unify_direct_exposure_eligible: updated_server.unify_direct_exposure_eligible,
        server_type: updated_server.server_type,
        command: updated_server.command.clone(),
        url: updated_server.url.clone(),
        args: details.args,
        env: details.env,
        headers: None,
        meta: details.meta,
        server_info: details.server_info,
        capability: details.capability,
        protocol_version: details.protocol_version,
        created_at: updated_server.created_at.map(|dt| dt.to_rfc3339()),
        updated_at: updated_server.updated_at.map(|dt| dt.to_rfc3339()),
        instances: details.instances,
        auth_mode: None,
        oauth_status: oauth_summary.oauth_status,
        oauth_custody_state: oauth_summary.oauth_custody_state,
        oauth_requires_reconnect: oauth_summary.oauth_requires_reconnect,
        oauth_issue: oauth_summary.oauth_issue,
        namespace_issue: None,
    }));

    let mut data = Map::new();
    data.insert("server_name".to_string(), Value::String(audit_server_name));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ServerUpdate,
            crate::audit::AuditStatus::Success,
            "POST",
            "/api/mcp/servers/update",
            Some(started_at.elapsed().as_millis() as u64),
            Some(audit_server_id),
            None,
            Some(data),
            None,
        ),
    )
    .await;

    Ok(response)
}

/// Import servers from JSON configuration (now uses unified core)
///
/// **Endpoint:** `POST /mcp/servers/import`
pub async fn import_servers(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ServersImportReq>,
) -> Result<Json<ServersImportResp>, ApiError> {
    let started_at = std::time::Instant::now();
    let db = common::get_database_from_state(&state)?;

    // Use safer dedup strategy by default: name + fingerprint, skip on conflict
    if let Some(profile_id) = payload.target_profile_id.as_ref() {
        let profile = crate::config::profile::get_profile(&db.pool, profile_id)
            .await
            .map_err(map_anyhow_error)?
            .ok_or_else(|| ApiError::NotFound(format!("Profile with ID '{}' not found", profile_id)))?;

        if !profile.is_active {
            return Err(ApiError::BadRequest(format!("Profile '{}' is not active", profile_id)));
        }
    }

    // Validate: require either client_identifier or mcp_servers
    if payload.client_identifier.is_none() && payload.mcp_servers.is_empty() {
        return Err(ApiError::BadRequest(
            "Either 'client_identifier' or 'mcp_servers' must be provided".to_string(),
        ));
    }

    let mcp_servers = if let Some(ref client_id) = payload.client_identifier {
        let service = state
            .client_service
            .as_ref()
            .ok_or_else(|| ApiError::InternalError("Client service unavailable".into()))?;

        service
            .fetch_state(client_id)
            .await
            .map_err(|err| ApiError::InternalError(err.to_string()))?
            .ok_or_else(|| ApiError::NotFound(format!("Client '{}' not found", client_id)))?;

        let plan = crate::config::server::plan_import_from_client_inspection(
            service,
            client_id,
            None,
            None,
            payload.selected_server_names.as_slice(),
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

        plan.items
    } else {
        payload.mcp_servers
    };

    let outcome = import_batch(
        &db.pool,
        &state.connection_pool,
        &state.redb_cache,
        mcp_servers,
        ImportOptions::dashboard_import(payload.dry_run, payload.target_profile_id.clone()),
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let ImportOutcome {
        imported,
        skipped,
        failed,
        scheduled: _,
    } = outcome;

    let imported_servers: Vec<String> = imported.into_iter().map(|s| s.name).collect();
    let skipped_servers: Vec<SkippedServerData> = skipped.into_iter().map(skipped_server_to_api).collect();
    let failed_servers: Vec<String> = failed.keys().cloned().collect();
    let error_details = if failed.is_empty() { None } else { Some(failed) };

    let import_data = ServersImportData {
        imported_count: imported_servers.len(),
        imported_servers,
        skipped_count: skipped_servers.len(),
        skipped_servers,
        failed_count: failed_servers.len(),
        failed_servers,
        error_details,
    };

    let mut data = Map::new();
    data.insert(
        "imported_count".to_string(),
        Value::from(import_data.imported_count as u64),
    );
    data.insert(
        "skipped_count".to_string(),
        Value::from(import_data.skipped_count as u64),
    );
    data.insert("failed_count".to_string(), Value::from(import_data.failed_count as u64));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ServerImport,
            if import_data.failed_count > 0 {
                crate::audit::AuditStatus::Failed
            } else {
                crate::audit::AuditStatus::Success
            },
            "POST",
            "/api/mcp/servers/import",
            Some(started_at.elapsed().as_millis() as u64),
            None,
            payload.target_profile_id,
            Some(data),
            None,
        ),
    )
    .await;

    Ok(Json(ServersImportResp::success(import_data)))
}

fn skipped_server_to_api(source: SkippedServer) -> SkippedServerData {
    SkippedServerData::from(source)
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
    state: &Arc<AppState>,
    db: &Database,
    server_id: &str,
) -> Result<(), ApiError> {
    delete_oauth_secrets_for_server_best_effort(state, db, server_id).await?;

    let mut tx = db.pool.begin().await.map_err(map_database_error)?;

    // Purge secret usages for this server first (no CASCADE FK to server_config).
    sqlx::query("DELETE FROM secure_store_usages WHERE server_id = ?")
        .bind(server_id)
        .execute(&mut *tx)
        .await
        .map_err(map_database_error)?;

    // Use CASCADE DELETE for server_config and its FK-linked tables.
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
    // - server_oauth_config (has FK to server_config.id)
    // - server_oauth_tokens (has FK to server_config.id)
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
    let started_at = std::time::Instant::now();
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
    delete_server_records(&state, &db, &server_id).await?;

    // Remove capability cache (REDB) for this server
    if let Err(e) = state.redb_cache.remove_server_data(&server_id).await {
        tracing::warn!("Failed to remove REDB cache for server '{}': {}", server_id, e);
    }

    // Remove resolver mapping to keep id<->name cache consistent
    crate::core::capability::resolver::remove_by_id(&server_id).await;

    tracing::info!("Successfully deleted server '{}'", existing_server.name);

    // Return success response
    let response = Json(ServerOperationData {
        id: server_id,
        name: existing_server.name,
        result: "Successfully deleted server".to_string(),
        status: "Deleted".to_string(),
        allowed_operations: Vec::new(),
    });

    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            crate::audit::AuditAction::ServerDelete,
            crate::audit::AuditStatus::Success,
            "DELETE",
            "/api/mcp/servers/delete",
            Some(started_at.elapsed().as_millis() as u64),
            Some(response.0.id.clone()),
            None,
            None,
            None,
        ),
    )
    .await;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::models::{ServerOAuthConfig, ServerOAuthToken},
        core::{
            cache::{RedbCacheManager, manager::CacheConfig},
            models::Config,
            pool::UpstreamConnectionPool,
            profile::ConfigApplicationStateManager,
            secrets::store::LocalSecretStore,
        },
        inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager},
        system::metrics::MetricsCollector,
    };
    use sqlx::sqlite::SqlitePoolOptions;
    use std::{path::PathBuf, sync::Arc, time::Duration};
    use tempfile::TempDir;
    use tokio::sync::{Mutex, RwLock};

    struct TestContext {
        _temp_dir: TempDir,
        app_state: Arc<AppState>,
        database: Arc<Database>,
    }

    #[tokio::test]
    async fn delete_server_records_continues_when_oauth_secret_cleanup_needs_secure_store() {
        let context = create_test_context().await;
        let server_id = "serv_delete_without_store";

        let mut server =
            Server::new_streamable_http("oauth server".to_string(), Some("https://example.com/mcp".to_string()));
        server.id = Some(server_id.to_string());
        server::upsert_server(&context.database.pool, &server)
            .await
            .expect("insert server");

        server::upsert_server_oauth_config(
            &context.database.pool,
            &ServerOAuthConfig {
                id: None,
                server_id: server_id.to_string(),
                authorization_endpoint: "https://issuer.example.com/authorize".to_string(),
                token_endpoint: "https://issuer.example.com/token".to_string(),
                client_id: "client-1".to_string(),
                client_secret: Some(format!("[[secret:oauth/{server_id}/client-secret]]")),
                scopes: Some("read write".to_string()),
                redirect_uri: "http://localhost:5173/oauth/callback".to_string(),
                created_at: None,
                updated_at: None,
            },
        )
        .await
        .expect("insert oauth config");

        server::upsert_server_oauth_token(
            &context.database.pool,
            &ServerOAuthToken {
                id: None,
                server_id: server_id.to_string(),
                access_token: format!("[[secret:oauth/{server_id}/access-token]]"),
                refresh_token: Some(format!("[[secret:oauth/{server_id}/refresh-token]]")),
                token_type: "bearer".to_string(),
                expires_at: None,
                scope: Some("read write".to_string()),
                created_at: None,
                updated_at: None,
            },
        )
        .await
        .expect("insert oauth token");
        insert_test_oauth_secret_rows(&context.database.pool, server_id).await;

        delete_server_records(&context.app_state, context.database.as_ref(), server_id)
            .await
            .expect("delete server records");

        assert!(
            server::get_server_by_id(&context.database.pool, server_id)
                .await
                .expect("load server")
                .is_none()
        );
        assert!(
            server::get_server_oauth_config(&context.database.pool, server_id)
                .await
                .expect("load oauth config")
                .is_none()
        );
        assert!(
            server::get_server_oauth_token(&context.database.pool, server_id)
                .await
                .expect("load oauth token")
                .is_none()
        );
        let remaining_secret_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM secure_store_secrets WHERE alias LIKE ?")
                .bind(format!("oauth/{server_id}/%"))
                .fetch_one(&context.database.pool)
                .await
                .expect("count oauth secret rows");
        assert_eq!(remaining_secret_count, 0);
    }

    #[tokio::test]
    async fn update_server_with_manual_authorization_header_clears_oauth_auth_source() {
        let context = create_test_context().await;
        let server_id = "serv_manual_authorization";

        let mut server = Server::new_streamable_http(
            "manual auth server".to_string(),
            Some("https://example.com/mcp".to_string()),
        );
        server.id = Some(server_id.to_string());
        server::upsert_server(&context.database.pool, &server)
            .await
            .expect("insert server");

        insert_plain_oauth_auth_source(&context.database.pool, server_id).await;

        let _response = update_server(
            State(context.app_state.clone()),
            Json(
                serde_json::from_value(serde_json::json!({
                    "id": server_id,
                    "headers": {
                        "Authorization": "Bearer manual-token",
                        "X-Trace": "trace-1",
                    }
                }))
                .expect("decode update request"),
            ),
        )
        .await
        .expect("update server");

        assert!(
            server::get_server_oauth_config(&context.database.pool, server_id)
                .await
                .expect("load oauth config")
                .is_none()
        );
        assert!(
            server::get_server_oauth_token(&context.database.pool, server_id)
                .await
                .expect("load oauth token")
                .is_none()
        );

        let headers = server::get_server_headers(&context.database.pool, server_id)
            .await
            .expect("load headers");
        assert_eq!(
            headers.get("authorization").map(String::as_str),
            Some("Bearer manual-token")
        );
        assert_eq!(headers.get("x-trace").map(String::as_str), Some("trace-1"));
    }

    #[tokio::test]
    async fn create_server_rejects_non_canonical_namespace_with_suggestion() {
        let context = create_test_context().await;

        let error = create_server(
            State(context.app_state.clone()),
            Json(
                serde_json::from_value(serde_json::json!({
                    "name": "Sequential Thinking-v2",
                    "server_type": "streamable_http",
                    "url": "https://example.com/mcp"
                }))
                .expect("decode create request"),
            ),
        )
        .await
        .expect_err("non-canonical namespace must fail");

        match error {
            ApiError::BadRequest(message) => {
                assert!(message.contains("Suggested namespace: 'sequential_thinking_v2'"));
            }
            other => panic!("expected bad request, got {other:?}"),
        }
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM server_config")
            .fetch_one(&context.database.pool)
            .await
            .expect("count servers");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn oauth_pending_import_rejects_namespace_before_creating_record() {
        let context = create_test_context().await;

        let error = create_server(
            State(context.app_state.clone()),
            Json(
                serde_json::from_value(serde_json::json!({
                    "name": "OAuth Pending Server",
                    "server_type": "streamable_http",
                    "url": "https://example.com/mcp",
                    "pending_import": true
                }))
                .expect("decode pending import request"),
            ),
        )
        .await
        .expect_err("pending OAuth record requires a confirmed namespace");

        assert!(matches!(error, ApiError::BadRequest(_)));
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM server_config")
            .fetch_one(&context.database.pool)
            .await
            .expect("count servers");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn create_server_rejects_namespace_conflicts_without_renaming() {
        let context = create_test_context().await;
        let mut existing = Server::new_streamable_http(
            "stable_namespace".to_string(),
            Some("https://example.com/existing".to_string()),
        );
        existing.id = Some("server-existing".to_string());
        server::upsert_server(&context.database.pool, &existing)
            .await
            .expect("insert existing server");

        let error = create_server(
            State(context.app_state.clone()),
            Json(
                serde_json::from_value(serde_json::json!({
                    "name": "stable_namespace",
                    "server_type": "streamable_http",
                    "url": "https://example.com/new"
                }))
                .expect("decode create request"),
            ),
        )
        .await
        .expect_err("duplicate namespace must fail");

        assert!(matches!(error, ApiError::Conflict(_)));
        let names = sqlx::query_scalar::<_, String>("SELECT name FROM server_config ORDER BY name")
            .fetch_all(&context.database.pool)
            .await
            .expect("load namespaces");
        assert_eq!(names, vec!["stable_namespace"]);
    }

    #[tokio::test]
    async fn namespace_remediation_is_available_only_for_invalid_legacy_servers() {
        let context = create_test_context().await;
        let fixture = context._temp_dir.path().join("namespace_remediation_fixture.py");
        std::fs::write(
            &fixture,
            format!(
                r#"
import json
import sys

for line in sys.stdin:
    request = json.loads(line)
    request_id = request.get("id")
    if request_id is None:
        continue
    method = request.get("method")
    if method == "initialize":
        result = {{
            "protocolVersion": "{}",
            "capabilities": {{"tools": {{}}}},
            "serverInfo": {{"name": "namespace-remediation-fixture", "version": "1.0.0"}}
        }}
    elif method == "tools/list":
        result = {{"tools": []}}
    else:
        continue
    sys.stdout.write(json.dumps({{"jsonrpc": "2.0", "id": request_id, "result": result}}) + "\n")
    sys.stdout.flush()
"#,
                crate::common::constants::protocol::CURRENT_VERSION
            ),
        )
        .expect("write namespace remediation fixture");
        let mut legacy = Server::new_stdio("Sequential Thinking".to_string(), Some("python3".to_string()));
        legacy.id = Some("server-legacy".to_string());
        server::upsert_server(&context.database.pool, &legacy)
            .await
            .expect("insert legacy server");
        server::upsert_server_args(
            &context.database.pool,
            "server-legacy",
            &[fixture.to_string_lossy().to_string()],
        )
        .await
        .expect("store fixture argument");

        let response = remediate_server_namespace(
            State(context.app_state.clone()),
            Json(ServerNamespaceRemediationReq {
                id: "server-legacy".to_string(),
                namespace: "sequential_reasoning".to_string(),
            }),
        )
        .await
        .expect("remediate legacy namespace");
        assert_eq!(response.0.data.expect("operation data").name, "sequential_reasoning");

        let error = remediate_server_namespace(
            State(context.app_state.clone()),
            Json(ServerNamespaceRemediationReq {
                id: "server-legacy".to_string(),
                namespace: "another_namespace".to_string(),
            }),
        )
        .await
        .expect_err("ordinary rename must remain unavailable");
        assert!(matches!(error, ApiError::Conflict(_)));
    }

    #[tokio::test]
    async fn namespace_remediation_connection_failure_does_not_commit_changes() {
        let context = create_test_context().await;
        let mut legacy =
            Server::new_streamable_http("Legacy Server".to_string(), Some("http://127.0.0.1:9/mcp".to_string()));
        legacy.id = Some("server-unreachable".to_string());
        server::upsert_server(&context.database.pool, &legacy)
            .await
            .expect("insert unreachable legacy server");

        remediate_server_namespace(
            State(context.app_state.clone()),
            Json(ServerNamespaceRemediationReq {
                id: "server-unreachable".to_string(),
                namespace: "legacy_server".to_string(),
            }),
        )
        .await
        .expect_err("upstream discovery failure must abort remediation");

        let namespace: String = sqlx::query_scalar("SELECT name FROM server_config WHERE id = 'server-unreachable'")
            .fetch_one(&context.database.pool)
            .await
            .expect("load unchanged namespace");
        assert_eq!(namespace, "Legacy Server");
    }

    #[tokio::test]
    async fn create_server_with_manual_authorization_header_persists_header() {
        let context = create_test_context().await;

        let response = create_server(
            State(context.app_state.clone()),
            Json(
                serde_json::from_value(serde_json::json!({
                    "name": "manual_auth_create_server",
                    "server_type": "streamable_http",
                    "url": "https://example.com/mcp",
                    "headers": {
                        "Authorization": "Bearer manual-token",
                        "X-Trace": "trace-1",
                    }
                }))
                .expect("decode create request"),
            ),
        )
        .await
        .expect("create server");
        let created = response.0.data.expect("created server data");
        let server_id = created.id.expect("created server id");

        let headers = server::get_server_headers(&context.database.pool, &server_id)
            .await
            .expect("load headers");
        assert_eq!(
            headers.get("authorization").map(String::as_str),
            Some("Bearer manual-token")
        );
        assert_eq!(headers.get("x-trace").map(String::as_str), Some("trace-1"));
    }

    #[tokio::test]
    async fn manual_authorization_cleanup_ignores_blank_authorization_and_proxy_authorization() {
        let context = create_test_context().await;
        let server_id = "serv_non_manual_authorization";

        let mut server = Server::new_streamable_http(
            "non manual auth server".to_string(),
            Some("https://example.com/mcp".to_string()),
        );
        server.id = Some(server_id.to_string());
        server::upsert_server(&context.database.pool, &server)
            .await
            .expect("insert server");
        insert_plain_oauth_auth_source(&context.database.pool, server_id).await;

        clear_oauth_auth_source_for_manual_authorization(
            &context.app_state,
            &context.database,
            server_id,
            &HashMap::from([
                ("Authorization".to_string(), "   ".to_string()),
                ("Proxy-Authorization".to_string(), "Bearer proxy-token".to_string()),
            ]),
        )
        .await
        .expect("manual auth cleanup");

        assert!(
            server::get_server_oauth_config(&context.database.pool, server_id)
                .await
                .expect("load oauth config")
                .is_some()
        );
        assert!(
            server::get_server_oauth_token(&context.database.pool, server_id)
                .await
                .expect("load oauth token")
                .is_some()
        );
    }

    async fn insert_plain_oauth_auth_source(
        pool: &sqlx::SqlitePool,
        server_id: &str,
    ) {
        server::upsert_server_oauth_config(
            pool,
            &ServerOAuthConfig {
                id: None,
                server_id: server_id.to_string(),
                authorization_endpoint: "https://issuer.example.com/authorize".to_string(),
                token_endpoint: "https://issuer.example.com/token".to_string(),
                client_id: "client-1".to_string(),
                client_secret: None,
                scopes: Some("read write".to_string()),
                redirect_uri: "http://localhost:5173/oauth/callback".to_string(),
                created_at: None,
                updated_at: None,
            },
        )
        .await
        .expect("insert oauth config");
        server::upsert_server_oauth_token(
            pool,
            &ServerOAuthToken {
                id: None,
                server_id: server_id.to_string(),
                access_token: "oauth-access-token".to_string(),
                refresh_token: Some("oauth-refresh-token".to_string()),
                token_type: "bearer".to_string(),
                expires_at: None,
                scope: Some("read write".to_string()),
                created_at: None,
                updated_at: None,
            },
        )
        .await
        .expect("insert oauth token");
    }

    async fn insert_test_oauth_secret_rows(
        pool: &sqlx::SqlitePool,
        server_id: &str,
    ) {
        for (alias, kind) in [
            (format!("oauth/{server_id}/client-secret"), "oauth_client_secret"),
            (format!("oauth/{server_id}/access-token"), "oauth_access_token"),
            (format!("oauth/{server_id}/refresh-token"), "oauth_refresh_token"),
        ] {
            sqlx::query(
                r#"
                INSERT INTO secure_store_secrets (
                    alias, kind, provider_id, provider_kind, version,
                    key_nonce, encrypted_key, nonce, encrypted_value
                )
                VALUES (?, ?, 'test-provider', 'test', 1, 'key-nonce', 'encrypted-key', 'nonce', 'encrypted-value')
                "#,
            )
            .bind(alias)
            .bind(kind)
            .execute(pool)
            .await
            .expect("insert oauth secret row");
        }
    }

    async fn create_test_context() -> TestContext {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&db_pool)
            .await
            .expect("enable foreign keys");

        crate::config::initialization::run_initialization(&db_pool)
            .await
            .expect("initialize database");
        LocalSecretStore::initialize_with_development_root_key(
            db_pool.clone(),
            temp_dir.path().join("secrets").join("local-root.key"),
        )
        .await
        .expect("init secret store tables");

        let database = Arc::new(Database {
            pool: db_pool.clone(),
            path: PathBuf::from(":memory:"),
        });

        let cache_path = temp_dir.path().join("capability.redb");
        let redb_cache = Arc::new(RedbCacheManager::new(cache_path, CacheConfig::default()).expect("cache manager"));

        let app_state = Arc::new(AppState {
            connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
                Arc::new(Config::default()),
                Some(database.clone()),
            ))),
            metrics_collector: Arc::new(MetricsCollector::new(Duration::from_secs(5))),
            http_proxy: None,
            profile_merge_service: None,
            database: Some(database.clone()),
            audit_database: None,
            audit_service: None,
            config_application_state: Arc::new(ConfigApplicationStateManager::new()),
            redb_cache,
            unified_query: None,
            client_service: None,
            inspector_calls: Arc::new(InspectorCallRegistry::new()),
            inspector_sessions: Arc::new(InspectorSessionManager::new()),
            oauth_manager: RwLock::new(Some(Arc::new(crate::core::oauth::OAuthManager::new(db_pool)))),
            secret_store: RwLock::new(None),
            secret_store_readiness: RwLock::new(crate::api::routes::unavailable_secret_store_readiness(
                "test_unavailable",
            )),
        });

        TestContext {
            _temp_dir: temp_dir,
            app_state,
            database,
        }
    }
}
