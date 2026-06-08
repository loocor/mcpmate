use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    api::{
        handlers::ApiError,
        models::secrets::{
            PassphraseRotateReq, ProviderModePayload, ProviderSwitchData, ProviderSwitchReq, ProviderSwitchResp,
            SecretCreateReq, SecretDeleteData, SecretDeleteReq, SecretDeleteResp, SecretDetailsReq, SecretKindPayload,
            SecretListData, SecretListResp, SecretMetadataData, SecretMetadataResp, SecretOriginData,
            SecretStoreIssueData, SecretStoreProviderData, SecretStoreStatusData, SecretStoreStatusResp,
            SecretStoreUnlockReq, SecretUpdateReq, SecretUsageData, SecretUsageListData, SecretUsageListResp,
            SecretUsageLocationData, SecretUsageReq,
        },
        routes::AppState,
    },
    config::server::{get_server_args, get_server_by_id, get_server_env, get_server_headers},
    core::{
        models::MCPServerConfig,
        secrets::{is_usage_active_in_config},
    },
    core::secrets::store::{
        SecretCreateInput, SecretKindInput, SecretMetadataView, SecretOriginInput, SecretStoreReadiness,
        SecretUpdateInput, SecretUsageLocationInput, SecretUsageView,
    },
};
use mcpmate_secrets::SecretRootKeyProvider;
use sqlx::SqlitePool;
use std::collections::HashMap;

pub async fn get_secret_store_status(
    State(state): State<Arc<AppState>>
) -> Result<Json<SecretStoreStatusResp>, ApiError> {
    let readiness = state.secret_store_readiness.try_read()
        .map(|guard| secret_store_status_data(&guard))
        .unwrap_or_else(|_| SecretStoreStatusData {
            status: "unavailable".to_string(),
            provider: None,
            issue: Some(SecretStoreIssueData {
                reason_code: "read_lock_failed".to_string(),
                message: "Could not read store status".to_string(),
            }),
        });
    Ok(Json(SecretStoreStatusResp::success(readiness)))
}

pub async fn list_secrets(State(state): State<Arc<AppState>>) -> Result<Json<SecretListResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let db = crate::api::handlers::server::common::get_database_from_state(&state)?;
    let mut server_config_cache: HashMap<String, Option<MCPServerConfig>> = HashMap::new();

    let secrets = store
        .list_secret_metadata()
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;

    // Pre-load all usages once and group by alias for active-count computation.
    let all_usages = store
        .list_all_usages()
        .await
        .map_err(|err| ApiError::InternalError(err.to_string()))?;
    let mut usages_by_alias: HashMap<String, Vec<SecretUsageView>> = HashMap::new();
    for usage in all_usages {
        usages_by_alias.entry(usage.alias.clone()).or_default().push(usage);
    }

    let mut enriched = Vec::with_capacity(secrets.len());
    for metadata in secrets {
        let mut active_count: u64 = 0;
        if let Some(usages) = usages_by_alias.get(&metadata.alias) {
            for usage in usages {
                let status =
                    resolve_secret_usage_status(&db.pool, usage, &mut server_config_cache).await?;
                if status == "active" {
                    active_count += 1;
                }
            }
        }
        let mut data = secret_metadata_data(metadata);
        data.used_by_count = active_count;
        enriched.push(data);
    }
    Ok(Json(SecretListResp::success(SecretListData { secrets: enriched })))
}

pub async fn create_secret(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SecretCreateReq>,
) -> Result<Json<SecretMetadataResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let metadata = store
        .create_secret(SecretCreateInput {
            alias: payload.alias,
            kind: secret_kind_input(payload.kind),
            value: payload.value,
            label: payload.label,
            origin: payload.origin.map(secret_origin_input),
        })
        .await
        .map_err(map_secret_store_error)?;
    Ok(Json(SecretMetadataResp::success(secret_metadata_data(metadata))))
}

pub async fn update_secret(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SecretUpdateReq>,
) -> Result<Json<SecretMetadataResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let metadata = store
        .update_secret(SecretUpdateInput {
            alias: payload.alias,
            kind: payload.kind.map(secret_kind_input),
            value: payload.value,
            label: payload.label,
            origin: payload.origin.map(secret_origin_input),
        })
        .await
        .map_err(map_secret_store_error)?;
    Ok(Json(SecretMetadataResp::success(secret_metadata_data(metadata))))
}

pub async fn get_secret_details(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SecretDetailsReq>,
) -> Result<Json<SecretMetadataResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let metadata = store
        .get_secret_metadata(&query.alias)
        .await
        .map_err(map_secret_store_error)?;
    Ok(Json(SecretMetadataResp::success(secret_metadata_data(metadata))))
}

pub async fn delete_secret(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SecretDeleteReq>,
) -> Result<Json<SecretDeleteResp>, ApiError> {
    let store = get_secret_store(&state)?;

    // When not force-deleting, check if any usages are still active
    // (i.e. the server still exists and its config still references the secret).
    // Stale usages (server removed or config changed) should not block deletion.
    if !payload.force {
        let db = crate::api::handlers::server::common::get_database_from_state(&state)?;
        let usages = store
            .list_usages(&payload.alias)
            .await
            .map_err(map_secret_store_error)?;
        let mut server_config_cache: HashMap<String, Option<MCPServerConfig>> = HashMap::new();
        for usage in &usages {
            let status =
                resolve_secret_usage_status(&db.pool, usage, &mut server_config_cache).await?;
            if status == "active" {
                return Err(ApiError::Conflict(format!(
                    "Secret '{}' is actively used by server '{}' and cannot be deleted",
                    payload.alias, usage.server_id
                )));
            }
        }
    }
    store
        .delete_secret(&payload.alias, true)
        .await
        .map_err(map_secret_store_error)?;
    Ok(Json(SecretDeleteResp::success(SecretDeleteData {
        alias: payload.alias,
        deleted: true,
    })))
}

pub async fn list_secret_usages(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SecretUsageReq>,
) -> Result<Json<SecretUsageListResp>, ApiError> {
    let store = get_secret_store(&state)?;
    let db = crate::api::handlers::server::common::get_database_from_state(&state)?;
    let usages = store
        .list_usages(&query.alias)
        .await
        .map_err(map_secret_store_error)?;
    let mut server_config_cache: HashMap<String, Option<MCPServerConfig>> = HashMap::new();
    let mut enriched = Vec::with_capacity(usages.len());
    for usage in usages {
        let status = resolve_secret_usage_status(&db.pool, &usage, &mut server_config_cache).await?;
        enriched.push(secret_usage_data(usage, status));
    }
    Ok(Json(SecretUsageListResp::success(SecretUsageListData { usages: enriched })))
}

fn get_secret_store(state: &Arc<AppState>) -> Result<Arc<crate::core::secrets::store::LocalSecretStore>, ApiError> {
    // Try read-lock first (non-blocking). If poisoned or empty, return error.
    state.secret_store.try_read()
        .ok()
        .and_then(|guard| guard.clone())
        .ok_or_else(|| {
            ApiError::ServiceUnavailable(
                "Secret store is unavailable. Unlock or configure the operating-system secure storage provider."
                    .to_string(),
            )
        })
}

fn map_secret_store_error(error: anyhow::Error) -> ApiError {
    let message = error.to_string();
    if message.contains("was not found") {
        ApiError::NotFound(message)
    } else if message.contains("in use") {
        ApiError::Conflict(message)
    } else {
        ApiError::BadRequest(message)
    }
}

fn secret_kind_input(kind: SecretKindPayload) -> SecretKindInput {
    match kind {
        SecretKindPayload::Generic => SecretKindInput::Generic,
        SecretKindPayload::Token => SecretKindInput::Token,
        SecretKindPayload::ApiKey => SecretKindInput::ApiKey,
        SecretKindPayload::Password => SecretKindInput::Password,
        SecretKindPayload::OAuthAccessToken => SecretKindInput::OAuthAccessToken,
        SecretKindPayload::OAuthRefreshToken => SecretKindInput::OAuthRefreshToken,
        SecretKindPayload::UrlCredential => SecretKindInput::UrlCredential,
        SecretKindPayload::HeaderValue => SecretKindInput::HeaderValue,
    }
}

fn secret_metadata_data(metadata: SecretMetadataView) -> SecretMetadataData {
    SecretMetadataData {
        alias: metadata.alias,
        placeholder: metadata.placeholder,
        kind: metadata.kind,
        label: metadata.label,
        origin: metadata.origin.map(secret_origin_data),
        provider_id: metadata.provider_id,
        provider_kind: metadata.provider_kind,
        version: metadata.version,
        used_by_count: metadata.used_by_count,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
    }
}

fn secret_origin_input(origin: SecretOriginData) -> SecretOriginInput {
    SecretOriginInput {
        server_id: origin.server_id,
        server_name: origin.server_name,
        server_kind: origin.server_kind,
        source: origin.source,
        field_group: origin.field_group,
        field_key: origin.field_key,
        field_index: origin.field_index,
        field_path: origin.field_path,
    }
}

fn secret_origin_data(origin: SecretOriginInput) -> SecretOriginData {
    SecretOriginData {
        server_id: origin.server_id,
        server_name: origin.server_name,
        server_kind: origin.server_kind,
        source: origin.source,
        field_group: origin.field_group,
        field_key: origin.field_key,
        field_index: origin.field_index,
        field_path: origin.field_path,
    }
}

fn secret_usage_data(usage: SecretUsageView, status: String) -> SecretUsageData {
    SecretUsageData {
        alias: usage.alias,
        server_id: usage.server_id,
        location: secret_usage_location_data(usage.location),
        status,
    }
}

async fn resolve_secret_usage_status(
    pool: &SqlitePool,
    usage: &SecretUsageView,
    cache: &mut HashMap<String, Option<MCPServerConfig>>,
) -> Result<String, ApiError> {
    let config = if let Some(cached) = cache.get(&usage.server_id) {
        cached.clone()
    } else {
        let loaded = load_mcp_server_config(pool, &usage.server_id)
            .await
            .map_err(crate::api::handlers::common::errors::map_anyhow_error)?;
        cache.insert(usage.server_id.clone(), loaded.clone());
        loaded
    };

    let Some(config) = config else {
        return Ok("stale".to_string());
    };

    let active = is_usage_active_in_config(&usage.alias, &usage.server_id, &usage.location, &config)
        .map_err(crate::api::handlers::common::errors::map_anyhow_error)?;
    Ok(if active { "active" } else { "stale" }.to_string())
}

async fn load_mcp_server_config(
    pool: &SqlitePool,
    server_id: &str,
) -> anyhow::Result<Option<MCPServerConfig>> {
    let Some(server) = get_server_by_id(pool, server_id).await? else {
        return Ok(None);
    };

    let args = get_server_args(pool, server_id)
        .await?
        .into_iter()
        .map(|arg| arg.arg_value)
        .collect::<Vec<_>>();
    let env = get_server_env(pool, server_id).await?;
    let headers = get_server_headers(pool, server_id).await?;

    Ok(Some(MCPServerConfig {
        kind: server.server_type,
        command: server.command.clone(),
        args: if args.is_empty() { None } else { Some(args) },
        url: server.url.clone(),
        env: if env.is_empty() { None } else { Some(env) },
        headers: if headers.is_empty() { None } else { Some(headers) },
    }))
}

fn secret_store_provider_data(snapshot: &crate::core::secrets::store::SecretStoreProviderSnapshot) -> SecretStoreProviderData {
    SecretStoreProviderData {
        provider_id: snapshot.provider_id.clone(),
        provider_kind: snapshot.provider_kind.clone(),
        provider_mode: snapshot.provider_mode.clone(),
        security_level: snapshot.security_level.clone(),
    }
}

fn secret_store_status_data(readiness: &SecretStoreReadiness) -> SecretStoreStatusData {
    match readiness {
        SecretStoreReadiness::Ready {
            provider_id,
            provider_kind,
            provider_mode,
            security_level,
        } => SecretStoreStatusData {
            status: "ready".to_string(),
            provider: Some(SecretStoreProviderData {
                provider_id: provider_id.clone(),
                provider_kind: provider_kind.clone(),
                provider_mode: provider_mode.clone(),
                security_level: security_level.clone(),
            }),
            issue: None,
        },
        SecretStoreReadiness::Unavailable {
            reason_code,
            message,
            provider,
        } => SecretStoreStatusData {
            status: "unavailable".to_string(),
            provider: provider.as_ref().map(secret_store_provider_data),
            issue: Some(SecretStoreIssueData {
                reason_code: reason_code.clone(),
                message: message.clone(),
            }),
        },
    }
}

fn data_dir_from_state(state: &Arc<AppState>) -> Result<std::path::PathBuf, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::ServiceUnavailable("No database configured".to_string()))?;
    Ok(db.path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf())
}

async fn apply_secret_store_bootstrap(
    state: &Arc<AppState>,
    bootstrap: crate::core::secrets::store::SecretStoreBootstrap,
) -> Result<SecretStoreReadiness, ApiError> {
    let readiness = bootstrap.readiness.clone();
    let store_arc = bootstrap.store.map(Arc::new);

    if let Some(store) = store_arc.clone() {
        state.connection_pool.lock().await.set_secret_resolver(store);
    }

    {
        let mut store_guard = state.secret_store.write().await;
        *store_guard = store_arc;
    }
    {
        let mut readiness_guard = state.secret_store_readiness.write().await;
        *readiness_guard = readiness.clone();
    }

    Ok(readiness)
}

async fn persist_provider_mode(
    state: &Arc<AppState>,
    mode: crate::core::secrets::store::RootKeyProviderMode,
) -> Result<(), ApiError> {
    let pool = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::ServiceUnavailable("No database configured".to_string()))?
        .pool
        .clone();
    mcpmate_secrets::database::upsert_provider_config(
        &pool,
        crate::core::secrets::store::provider_mode_to_persisted(mode),
    )
    .await
    .map_err(|err| ApiError::InternalError(format!("Failed to persist provider mode: {err}")))?;
    Ok(())
}

pub async fn unlock_secret_store(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SecretStoreUnlockReq>,
) -> Result<Json<SecretStoreStatusResp>, ApiError> {
    if payload.passphrase.is_empty() {
        return Err(ApiError::BadRequest("Passphrase cannot be empty".to_string()));
    }

    let needs_unlock = {
        let readiness = state.secret_store_readiness.try_read().map_err(|_| {
            ApiError::ServiceUnavailable("Store lock contention".to_string())
        })?;
        match &*readiness {
            SecretStoreReadiness::Unavailable { reason_code, .. } => {
                reason_code == "passphrase_unlock_required"
            }
            _ => false,
        }
    };
    if !needs_unlock {
        return Err(ApiError::BadRequest(
            "Secret store does not require passphrase unlock".to_string(),
        ));
    }

    let data_dir = data_dir_from_state(&state)?;
    let pool = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::ServiceUnavailable("No database configured".to_string()))?
        .pool
        .clone();

    let bootstrap = crate::core::secrets::store::initialize_secret_store_with_passphrase(
        pool,
        &data_dir,
        &payload.passphrase,
    )
    .await
    .map_err(|err| ApiError::InternalError(format!("Failed to unlock secret store: {err}")))?;

    if bootstrap.store.is_none() {
        let message = match &bootstrap.readiness {
            SecretStoreReadiness::Unavailable { message, .. } => message.clone(),
            _ => "Failed to unlock secret store".to_string(),
        };
        return Err(ApiError::BadRequest(message));
    }

    let readiness = apply_secret_store_bootstrap(&state, bootstrap).await?;
    Ok(Json(SecretStoreStatusResp::success(secret_store_status_data(
        &readiness,
    ))))
}

pub async fn rotate_passphrase(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PassphraseRotateReq>,
) -> Result<Json<SecretStoreStatusResp>, ApiError> {
    if payload.new_passphrase.is_empty() {
        return Err(ApiError::BadRequest("New passphrase cannot be empty".to_string()));
    }
    if payload.new_passphrase != payload.confirm {
        return Err(ApiError::BadRequest("New passphrases do not match".to_string()));
    }
    if payload.new_passphrase.len() < 4 {
        return Err(ApiError::BadRequest(
            "New passphrase must be at least 4 characters".to_string(),
        ));
    }

    let store = get_secret_store(&state)?;
    if store.provider_metadata().mode() != crate::core::secrets::store::RootKeyProviderMode::Passphrase {
        return Err(ApiError::BadRequest(
            "Passphrase rotation is only available in passphrase encryption mode".to_string(),
        ));
    }

    let data_dir = data_dir_from_state(&state)?;
    let (passphrase_path, _) = crate::core::secrets::store::secret_store_paths(&data_dir);

    let current_provider = crate::core::secrets::store::PassphraseRootKeyProvider::new(
        passphrase_path.clone(),
        payload.current_passphrase.clone(),
    );
    let root_key = current_provider
        .load_or_create_root_key()
        .map_err(|err| match err {
            mcpmate_secrets::SecretRootKeyError::InvalidMaterial(message) => {
                ApiError::BadRequest(format!("Invalid current passphrase: {message}"))
            }
            other => ApiError::InternalError(format!("Failed to load root key: {other}")),
        })?;

    // Backup the old passphrase file before overwriting.
    let backup_path = passphrase_path.with_extension("json.rotate-bak");
    let has_backup = std::fs::copy(&passphrase_path, &backup_path).is_ok();

    crate::core::secrets::store::PassphraseRootKeyProvider::new(
        passphrase_path.clone(),
        payload.new_passphrase.clone(),
    )
    .set_root_key(&root_key)
    .map_err(|err| ApiError::InternalError(format!("Failed to re-wrap root key: {err}")))?;

    let pool = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::ServiceUnavailable("No database configured".to_string()))?
        .pool
        .clone();
    let bootstrap = match crate::core::secrets::store::initialize_secret_store_with_passphrase(
        pool,
        &data_dir,
        &payload.new_passphrase,
    )
    .await
    {
        Ok(bootstrap) => bootstrap,
        Err(err) => {
            // Rollback: restore old passphrase file from backup.
            if has_backup {
                let _ = std::fs::rename(&backup_path, &passphrase_path);
            }
            return Err(ApiError::InternalError(format!(
                "Failed to reinitialize secret store; passphrase rolled back: {err}"
            )));
        }
    };

    if bootstrap.store.is_none() {
        // Rollback: restore old passphrase file from backup.
        if has_backup {
            let _ = std::fs::rename(&backup_path, &passphrase_path);
        }
        return Err(ApiError::InternalError(
            "Secret store failed to initialize after passphrase rotation; passphrase rolled back".to_string(),
        ));
    }

    // Clean up backup on success.
    if has_backup {
        let _ = std::fs::remove_file(&backup_path);
    }

    let readiness = apply_secret_store_bootstrap(&state, bootstrap).await?;
    Ok(Json(SecretStoreStatusResp::success(secret_store_status_data(
        &readiness,
    ))))
}

fn secret_usage_location_data(location: SecretUsageLocationInput) -> SecretUsageLocationData {
    match location {
        SecretUsageLocationInput::StdioCommand => SecretUsageLocationData::StdioCommand,
        SecretUsageLocationInput::StdioArgument { index } => SecretUsageLocationData::StdioArgument { index },
        SecretUsageLocationInput::StdioEnv { name } => SecretUsageLocationData::StdioEnv { name },
        SecretUsageLocationInput::StreamableHttpUrl => SecretUsageLocationData::StreamableHttpUrl,
        SecretUsageLocationInput::StreamableHttpHeader { name } => {
            SecretUsageLocationData::StreamableHttpHeader { name }
        }
        SecretUsageLocationInput::OAuthToken => SecretUsageLocationData::OAuthToken,
    }
}

pub async fn switch_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ProviderSwitchReq>,
) -> Result<Json<ProviderSwitchResp>, ApiError> {
    // Determine data directory from database path.
    let db = state.database.as_ref().ok_or_else(|| {
        ApiError::ServiceUnavailable("No database configured".to_string())
    })?;
    let data_dir = db.path.parent().unwrap_or(std::path::Path::new("."));
    let secrets_dir = data_dir.join("secrets");
    let passphrase_path = secrets_dir.join("passphrase-wrapped-key.json");
    let local_file_path = secrets_dir.join("local-root.key");

    // Check current mode. Fall back to persisted provider mode from the database
    // when the in-memory store is unavailable (e.g. passphrase_unlock_required).
    let current_mode = {
        let store_guard = state.secret_store.try_read().map_err(|_| {
            ApiError::ServiceUnavailable("Store lock contention".to_string())
        })?;
        store_guard
            .as_ref()
            .map(|s| s.provider_metadata().mode())
    };

    let current_mode = match current_mode {
        Some(mode) => mode,
        None => {
            // Store not initialized — read persisted mode from the database.
            let pool = &db.pool;
            mcpmate_secrets::database::get_provider_config(pool)
                .await
                .map_err(|err| {
                    ApiError::InternalError(format!("Failed to read provider config: {err}"))
                })?
                .map(|cfg| {
                    crate::core::secrets::store::parse_persisted_provider_mode(&cfg.provider_mode)
                        .map_err(|err| ApiError::BadRequest(err))
                })
                .transpose()?
                .ok_or_else(|| {
                    ApiError::BadRequest(
                        "Secret store is not configured. Set up encryption before switching providers."
                            .to_string(),
                    )
                })?
        }
    };

    // Determine new mode.
    let new_mode = match payload.mode {
        ProviderModePayload::OperatingSystem => crate::core::secrets::store::RootKeyProviderMode::OperatingSystem,
        ProviderModePayload::Passphrase => crate::core::secrets::store::RootKeyProviderMode::Passphrase,
        ProviderModePayload::LocalFile => crate::core::secrets::store::RootKeyProviderMode::LocalFile,
    };

    // Already on this mode — return current status.
    if current_mode == new_mode {
        let readiness = state.secret_store_readiness.try_read()
            .map(|guard| secret_store_status_data(&guard))
            .unwrap_or_else(|_| SecretStoreStatusData {
                status: "ready".to_string(),
                provider: None,
                issue: None,
            });
        return Ok(Json(ProviderSwitchResp::success(ProviderSwitchData {
            new_status: readiness,
        })));
    }

    // Validate passphrases for the switch direction.
    let new_passphrase = payload.passphrase.filter(|p| !p.is_empty());
    let current_passphrase = payload.current_passphrase.filter(|p| !p.is_empty());

    if new_mode == crate::core::secrets::store::RootKeyProviderMode::Passphrase && new_passphrase.is_none() {
        return Err(ApiError::BadRequest(
            "Passphrase is required for passphrase mode".to_string(),
        ));
    }

    let switching_from_passphrase =
        current_mode == crate::core::secrets::store::RootKeyProviderMode::Passphrase;
    if switching_from_passphrase && current_passphrase.is_none() {
        return Err(ApiError::BadRequest(
            "Current passphrase is required to switch from passphrase mode".to_string(),
        ));
    }

    let load_passphrase = current_passphrase.clone().unwrap_or_default();
    let store_passphrase = new_passphrase.clone().unwrap_or_default();

    // Load root key from the old provider.
    let root_key = {
        let old_provider: Box<dyn crate::core::secrets::store::SecretRootKeyProvider> = match current_mode {
            crate::core::secrets::store::RootKeyProviderMode::OperatingSystem => {
                Box::new(crate::core::secrets::store::OperatingSystemRootKeyProvider::new())
            }
            crate::core::secrets::store::RootKeyProviderMode::Passphrase => {
                Box::new(crate::core::secrets::store::PassphraseRootKeyProvider::new(
                    passphrase_path.clone(),
                    load_passphrase.clone(),
                ))
            }
            crate::core::secrets::store::RootKeyProviderMode::LocalFile => {
                Box::new(crate::core::secrets::store::LocalFileRootKeyProvider::new(
                    local_file_path.clone(),
                ))
            }
            _ => {
                return Err(ApiError::BadRequest(
                    "Current provider mode does not support switching".to_string(),
                ));
            }
        };
        old_provider.load_or_create_root_key().map_err(|err| match err {
            mcpmate_secrets::SecretRootKeyError::InvalidMaterial(message) => {
                ApiError::BadRequest(format!("Invalid current passphrase: {message}"))
            }
            other => ApiError::InternalError(format!("Failed to load root key: {other}")),
        })?
    };

    // Backup old provider key file (for passphrase/local_file modes).
    let backup_path = passphrase_path.with_extension("json.rollback-bak");
    let local_backup_path = local_file_path.with_extension("key.rollback-bak");
    let has_backup = match current_mode {
        crate::core::secrets::store::RootKeyProviderMode::Passphrase if passphrase_path.exists() => {
            std::fs::copy(&passphrase_path, &backup_path).is_ok()
        }
        crate::core::secrets::store::RootKeyProviderMode::LocalFile if local_file_path.exists() => {
            std::fs::copy(&local_file_path, &local_backup_path).is_ok()
        }
        _ => false,
    };

    // Store the root key in the new provider.
    match new_mode {
        crate::core::secrets::store::RootKeyProviderMode::OperatingSystem => {
            crate::core::secrets::store::OperatingSystemRootKeyProvider::new()
                .set_root_key(&root_key)
                .map_err(|err| ApiError::InternalError(format!("Failed to store key in OS keyring: {err}")))?;
        }
        crate::core::secrets::store::RootKeyProviderMode::Passphrase => {
            crate::core::secrets::store::PassphraseRootKeyProvider::new(
                passphrase_path.clone(),
                store_passphrase.clone(),
            )
            .set_root_key(&root_key)
            .map_err(|err| ApiError::InternalError(format!("Failed to store key as passphrase: {err}")))?;
        }
        crate::core::secrets::store::RootKeyProviderMode::LocalFile => {
            crate::core::secrets::store::LocalFileRootKeyProvider::new(local_file_path.clone())
                .set_root_key(&root_key)
                .map_err(|err| ApiError::InternalError(format!("Failed to store key as local file: {err}")))?;
        }
        _ => {
            return Err(ApiError::BadRequest(
                "Target provider mode is not supported".to_string(),
            ));
        }
    }

    // Create a new store with the new provider.
    let new_provider: Arc<dyn crate::core::secrets::store::SecretRootKeyProvider> = match new_mode {
        crate::core::secrets::store::RootKeyProviderMode::OperatingSystem => {
            Arc::new(crate::core::secrets::store::OperatingSystemRootKeyProvider::new())
        }
        crate::core::secrets::store::RootKeyProviderMode::Passphrase => {
            Arc::new(crate::core::secrets::store::PassphraseRootKeyProvider::new(
                passphrase_path.clone(),
                store_passphrase.clone(),
            ))
        }
        crate::core::secrets::store::RootKeyProviderMode::LocalFile => {
            Arc::new(crate::core::secrets::store::LocalFileRootKeyProvider::new(local_file_path.clone()))
        }
        _ => unreachable!(),
    };

    let new_store = match crate::core::secrets::store::LocalSecretStore::initialize_with_root_key_provider(
        db.pool.clone(),
        new_provider,
    )
    .await
    {
        Ok(store) => store,
        Err(err) => {
            // Rollback: restore old provider key.
            let _ = match current_mode {
                crate::core::secrets::store::RootKeyProviderMode::OperatingSystem => {
                    crate::core::secrets::store::OperatingSystemRootKeyProvider::new()
                        .set_root_key(&root_key)
                }
                crate::core::secrets::store::RootKeyProviderMode::Passphrase => {
                    if has_backup {
                        let _ = std::fs::rename(&backup_path, &passphrase_path);
                    }
                    crate::core::secrets::store::PassphraseRootKeyProvider::new(
                        passphrase_path.clone(),
                        load_passphrase.clone(),
                    )
                    .set_root_key(&root_key)
                }
                crate::core::secrets::store::RootKeyProviderMode::LocalFile => {
                    if has_backup {
                        let _ = std::fs::rename(&local_backup_path, &local_file_path);
                    }
                    crate::core::secrets::store::LocalFileRootKeyProvider::new(local_file_path.clone())
                        .set_root_key(&root_key)
                }
                _ => Ok(()),
            };
            // Re-initialize the store with the old provider mode (not always passphrase).
            let _ = match current_mode {
                crate::core::secrets::store::RootKeyProviderMode::Passphrase => {
                    crate::core::secrets::store::initialize_secret_store_with_passphrase(
                        db.pool.clone(),
                        data_dir,
                        &load_passphrase,
                    )
                    .await
                    .map(|_| ())
                }
                crate::core::secrets::store::RootKeyProviderMode::OperatingSystem => {
                    crate::core::secrets::store::LocalSecretStore::initialize_with_root_key_provider(
                        db.pool.clone(),
                        Arc::new(crate::core::secrets::store::OperatingSystemRootKeyProvider::new()),
                    )
                    .await
                    .map(|_| ())
                }
                crate::core::secrets::store::RootKeyProviderMode::LocalFile => {
                    crate::core::secrets::store::LocalSecretStore::initialize_with_root_key_provider(
                        db.pool.clone(),
                        Arc::new(crate::core::secrets::store::LocalFileRootKeyProvider::new(local_file_path.clone())),
                    )
                    .await
                    .map(|_| ())
                }
                _ => Ok(()),
            };
            return Err(ApiError::InternalError(format!(
                "Failed to initialize store with new provider; rolled back to previous state: {err}"
            )));
        }
    };

    // Clean up backup files on success.
    if has_backup {
        let _ = std::fs::remove_file(&backup_path);
        let _ = std::fs::remove_file(&local_backup_path);
    }

    persist_provider_mode(&state, new_mode).await?;

    let bootstrap = crate::core::secrets::store::SecretStoreBootstrap {
        readiness: crate::core::secrets::store::SecretStoreReadiness::ready(new_store.provider_metadata()),
        store: Some(new_store),
    };
    let new_readiness = apply_secret_store_bootstrap(&state, bootstrap).await?;

    // Clean up the old provider's key file AFTER bootstrap succeeds.
    // The root key is now securely stored in the new provider.
    match current_mode {
        crate::core::secrets::store::RootKeyProviderMode::LocalFile => {
            let _ = std::fs::remove_file(&local_file_path);
        }
        crate::core::secrets::store::RootKeyProviderMode::Passphrase => {
            let _ = std::fs::remove_file(&passphrase_path);
        }
        _ => {} // OS keyring has no file to clean.
    }

    let new_status = secret_store_status_data(&new_readiness);
    Ok(Json(ProviderSwitchResp::success(ProviderSwitchData {
        new_status,
    })))
}
