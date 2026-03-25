// HTTP handlers for client management API (template-driven)

use super::config::{analyze_config_content, get_config_last_modified};
use super::import::build_import_payload_from_value;
use crate::api::models::client::{
    ClientBackupActionData, ClientBackupActionResp, ClientCapabilityConfigData, ClientCapabilityConfigReq,
    ClientCapabilityConfigResp, ClientCheckData, ClientCheckReq, ClientCheckResp, ClientConfigData,
    ClientConfigImportData, ClientConfigImportReq, ClientConfigImportResp, ClientConfigMode, ClientConfigReq,
    ClientConfigResp, ClientConfigRestoreReq, ClientConfigSelected, ClientConfigUpdateData, ClientConfigUpdateReq,
    ClientConfigUpdateResp, ClientImportSummary, ClientImportedServer, ClientInfo, ClientTemplateMetadata,
    ClientTemplateStorageMetadata,
};
use crate::api::routes::AppState;
use crate::clients::models::{ClientTemplate, ContainerType, MergeStrategy, StorageKind, TemplateFormat};
use crate::clients::{
    ClientConfigService, ClientDescriptor, ClientRenderOptions, ConfigError, ConfigMode, TemplateExecutionResult,
};
use crate::common::ClientCategory;
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use json5;
use serde_json::Value;
use serde_yaml;
use std::sync::Arc;
use toml;

/// Handler for GET /api/client
/// Detects and returns all clients, with optional template reload
pub async fn list(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ClientCheckReq>,
) -> Result<Json<ClientCheckResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    if request.refresh {
        if let Err(err) = service.reload_templates().await {
            tracing::warn!("Failed to reload client templates: {}", err);
        }
    }

    let descriptors = service.list_clients(request.refresh).await.map_err(|err| {
        tracing::error!("Failed to list clients: {}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut client_infos = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        match descriptor_to_client_info(service.as_ref(), descriptor).await {
            Ok(info) => client_infos.push(info),
            Err(status) => return Err(status),
        }
    }

    let response = ClientCheckData {
        total: client_infos.len(),
        client: client_infos,
        last_updated: Utc::now().to_rfc3339(),
    };

    Ok(Json(ClientCheckResp::success(response)))
}

/// Handler for GET /api/client/config/details
/// Returns current configuration content
pub async fn config_details(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ClientConfigReq>,
) -> Result<Json<ClientConfigResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let template = service.get_client_template(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to load client template {}: {}", request.identifier, err);
        StatusCode::NOT_FOUND
    })?;

    let config_path = service.config_path(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to resolve config path for {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut warnings: Vec<String> = Vec::new();
    let content = match service.read_current_config(&request.identifier).await {
        Ok(content) => content,
        Err(err) => {
            let message = format!("Unable to read current configuration: {}", err);
            tracing::warn!(
                client = %request.identifier,
                error = %err,
                "Gracefully degrading after configuration read failure"
            );
            warnings.push(message);
            None
        }
    };

    let config_exists = content.is_some();
    let parsed_content = content
        .as_deref()
        .map(|raw| parse_config_value(raw, &template))
        .unwrap_or(Value::Null);

    let (has_mcp_config, mcp_servers_count) = content
        .as_deref()
        .map(|raw| analyze_config_content(raw, &request.identifier, &template))
        .unwrap_or((false, 0));

    let last_modified = config_path.as_deref().and_then(get_config_last_modified);

    let config_type = convert_container_type(template.config_mapping.container_type);

    let managed = match service.is_client_managed(&request.identifier).await {
        Ok(state) => state,
        Err(err) => {
            tracing::warn!(
                client = %request.identifier,
                error = %err,
                "Falling back to disabled managed state after lookup failure"
            );
            warnings.push(format!("Failed to load managed state: {}", err));
            false
        }
    };

    let (imported_servers, import_summary) = (None, None);
    let description = metadata_string(&template, "description");
    let homepage_url = metadata_string(&template, "homepage_url");
    let docs_url = metadata_string(&template, "docs_url");
    let support_url = metadata_string(&template, "support_url");
    let logo_url = extract_logo_url(&template);

    let capability_config = service
        .get_capability_config(&request.identifier)
        .await
        .map_err(|err| {
            tracing::error!(
                client = %request.identifier,
                error = %err,
                "Failed to load capability config"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or_default();

    let data = ClientConfigData {
        config_path: config_path.unwrap_or_default(),
        config_exists,
        content: parsed_content,
        has_mcp_config,
        mcp_servers_count,
        last_modified,
        config_type,
        imported_servers,
        import_summary,
        template: build_template_metadata(&template),
        supported_transports: extract_supported_transports(&template),
        managed,
        description,
        homepage_url,
        docs_url,
        support_url,
        logo_url,
        capability_source: capability_config.capability_source,
        selected_profile_ids: capability_config.selected_profile_ids,
        custom_profile_id: capability_config.custom_profile_id,
        warnings,
    };

    Ok(Json(ClientConfigResp::success(data)))
}

/// Handler for POST /api/client/config/apply
/// Generates and optionally applies configuration
#[tracing::instrument(
    skip(app_state, request),
    level = "debug",
    fields(
        client = %request.identifier,
        mode = ?request.mode,
        preview = %request.preview,
        selected = ?request.selected_config
    )
)]
pub async fn config_apply(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientConfigUpdateReq>,
) -> Result<Json<ClientConfigUpdateResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let template = service.get_client_template(&request.identifier).await.map_err(|err| {
        tracing::error!(
            client = %request.identifier,
            error = %err,
            "Failed to load client template"
        );
        StatusCode::NOT_FOUND
    })?;
    let options = build_render_options(&request);
    let outcome = service.apply_with_deferred(options).await.map_err(|err| {
        let status = match err {
            ConfigError::ClientDisabled { .. } => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        tracing::error!(
            client = %request.identifier,
            mode = ?request.mode,
            preview = %request.preview,
            selected = ?request.selected_config,
            status = %status.as_u16(),
            error = %err,
            "config_apply failed"
        );
        status
    })?;
    let synthetic = TemplateExecutionResult::DryRun {
        diff: crate::clients::renderer::ConfigDiff {
            format: outcome.preview.format,
            before: outcome.preview.before.clone(),
            after: outcome.preview.after.clone(),
            summary: outcome.preview.summary.clone(),
        },
        content: outcome.preview.after.clone().unwrap_or_default(),
    };
    let preview = build_update_preview(&template, &synthetic);
    let mut warnings = outcome.warnings.clone();
    warnings.extend(outcome.preview.summary.clone().into_iter());
    let diff_format = Some(outcome.preview.format.as_str().to_string());
    let diff_before = outcome.preview.before.clone();
    let diff_after = outcome.preview.after.clone();
    let applied = outcome.applied && !request.preview;
    let backup_path = outcome.backup_path.clone();

    let data = ClientConfigUpdateData {
        success: true,
        preview,
        applied,
        backup_path,
        warnings,
        diff_format,
        diff_before,
        diff_after,
        scheduled: Some(outcome.scheduled),
        scheduled_reason: outcome.scheduled_reason,
    };

    tracing::info!(
        client = %request.identifier,
        applied = outcome.applied,
        preview = %request.preview,
        scheduled = outcome.scheduled,
        "config_apply succeeded"
    );
    Ok(Json(ClientConfigUpdateResp::success(data)))
}

/// Handler for POST /api/client/config/restore
/// Restores configuration from a named backup snapshot
pub async fn config_restore(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientConfigRestoreReq>,
) -> Result<Json<ClientBackupActionResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let result = service
        .restore_backup(&request.identifier, &request.backup)
        .await
        .map_err(|err| match err {
            ConfigError::TemplateIndexError(_) | ConfigError::FileOperationError(_) => {
                tracing::warn!(
                    "Backup {} for client {} not found or unreadable",
                    request.backup,
                    request.identifier
                );
                StatusCode::NOT_FOUND
            }
            other => {
                tracing::error!(
                    "Failed to restore backup {} for {}: {}",
                    request.backup,
                    request.identifier,
                    other
                );
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    let message = match result {
        Some(path) => format!("Restored configuration; previous version saved to {}", path),
        None => "Restored configuration without creating a new backup".to_string(),
    };

    let data = ClientBackupActionData {
        identifier: request.identifier,
        backup: request.backup,
        message,
    };

    Ok(Json(ClientBackupActionResp::success(data)))
}

/// Handler for POST /api/client/config/import
/// Preview or import servers from the client's existing configuration
#[tracing::instrument(skip(app_state, request), level = "debug", fields(client = %request.identifier, preview = %request.preview, profile = ?request.profile_id))]
pub async fn config_import(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientConfigImportReq>,
) -> Result<Json<ClientConfigImportResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let template = service.get_client_template(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to load client template {}: {}", request.identifier, err);
        StatusCode::NOT_FOUND
    })?;

    let raw = service.read_current_config(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to read config for {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let json_value = raw
        .as_deref()
        .map(|raw| parse_config_value(raw, &template))
        .unwrap_or(serde_json::Value::Null);

    let db = app_state.database.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Build standard import payload from parsed config
    let items = build_import_payload_from_value(&json_value);
    let opts = crate::config::server::ImportOptions {
        by_name: true,
        by_fingerprint: true,
        conflict_policy: crate::config::server::ConflictPolicy::Skip,
        preview: request.preview,
        target_profile: request.profile_id.clone(),
    };
    let outcome =
        crate::config::server::import_batch(&db.pool, &app_state.connection_pool, &app_state.redb_cache, items, opts)
            .await
            .map_err(|err| {
                tracing::error!("Failed to import via unified core: {}", err);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    // Report the profile used for association (actual association is handled in import core)
    let mut profile_used: Option<String> = None;
    if !request.preview && !outcome.imported.is_empty() {
        let profile_id = if let Some(pid) = &request.profile_id {
            pid.clone()
        } else {
            // Ensure the system default anchor profile exists so we can report its identifier
            match crate::config::profile::ensure_default_anchor_profile_id(&db.pool).await {
                Ok(id) => id,
                Err(err) => {
                    tracing::error!("Failed to ensure default anchor profile during client import: {}", err);
                    String::new()
                }
            }
        };
        if !profile_id.is_empty() {
            profile_used = Some(profile_id);
        }
    }

    let crate::config::server::ImportOutcome {
        imported,
        skipped,
        failed,
        scheduled,
    } = outcome;

    let imported_servers: Vec<ClientImportedServer> = imported
        .into_iter()
        .map(|s| ClientImportedServer {
            name: s.name,
            command: s.command.unwrap_or_default(),
            args: s.args,
            env: s.env,
            server_type: s.server_type,
            url: s.url,
        })
        .collect();

    let summary = ClientImportSummary {
        attempted: true,
        imported_count: imported_servers.len() as u32,
        skipped_count: skipped.len() as u32,
        failed_count: failed.len() as u32,
        errors: if failed.is_empty() { None } else { Some(failed) },
    };

    let data = ClientConfigImportData {
        summary,
        imported_servers,
        profile_id: profile_used,
        scheduled: Some(scheduled),
        scheduled_reason: None,
    };

    Ok(Json(ClientConfigImportResp::success(data)))
}

pub(crate) fn get_client_service(state: &AppState) -> Result<Arc<ClientConfigService>, StatusCode> {
    state
        .client_service
        .as_ref()
        .cloned()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

/// PATCH/POST /api/client/update - partial update client settings
pub async fn update_settings(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<crate::api::models::client::ClientSettingsUpdateReq>,
) -> Result<Json<crate::api::models::client::ClientSettingsUpdateResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    tracing::info!(
        client = %request.identifier,
        config_mode = ?request.config_mode,
        transport = ?request.transport,
        client_version = ?request.client_version,
        "update_settings: received request"
    );

    service
        .set_client_settings(
            &request.identifier,
            request.config_mode.clone(),
            request.transport.clone(),
            request.client_version.clone(),
        )
        .await
        .map_err(|err| {
            tracing::error!(
                client = %request.identifier,
                error = %err,
                "Failed to update client settings"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let (mode, transport, version) = service
        .get_client_settings(&request.identifier)
        .await
        .map_err(|err| {
            tracing::error!(
                client = %request.identifier,
                error = %err,
                "Failed to fetch updated client settings"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or(("hosted".into(), "auto".into(), None));

    let data = crate::api::models::client::ClientSettingsUpdateData {
        identifier: request.identifier,
        config_mode: mode,
        transport,
        client_version: version,
    };

    Ok(Json(crate::api::models::client::ClientSettingsUpdateResp::success(
        data,
    )))
}

pub async fn update_capability_config(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientCapabilityConfigReq>,
) -> Result<Json<ClientCapabilityConfigResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let config = service
        .set_capability_config(
            &request.identifier,
            request.capability_source,
            request.selected_profile_ids,
        )
        .await
        .map_err(|err| {
            tracing::error!(
                client = %request.identifier,
                error = %err,
                "Failed to update capability config"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ClientCapabilityConfigResp::success(
        ClientCapabilityConfigData {
            identifier: request.identifier,
            capability_source: config.capability_source,
            selected_profile_ids: config.selected_profile_ids,
            custom_profile_id: config.custom_profile_id,
        },
    )))
}

pub async fn get_capability_config(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ClientConfigReq>,
) -> Result<Json<ClientCapabilityConfigResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let config = service
        .get_capability_config(&request.identifier)
        .await
        .map_err(|err| {
            tracing::error!(
                client = %request.identifier,
                error = %err,
                "Failed to load capability config"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ClientCapabilityConfigResp::success(
        ClientCapabilityConfigData {
            identifier: request.identifier,
            capability_source: config.capability_source,
            selected_profile_ids: config.selected_profile_ids,
            custom_profile_id: config.custom_profile_id,
        },
    )))
}

async fn descriptor_to_client_info(
    service: &ClientConfigService,
    descriptor: ClientDescriptor,
) -> Result<ClientInfo, StatusCode> {
    let template = descriptor.template.clone();
    let display_name = template_display_name(&template);
    let logo_url = extract_logo_url(&template);
    let category = extract_category(&template);
    let supported_transports = extract_supported_transports(&template);
    let description = metadata_string(&template, "description");
    let homepage_url = metadata_string(&template, "homepage_url");
    let docs_url = metadata_string(&template, "docs_url");
    let support_url = metadata_string(&template, "support_url");
    let config_type = convert_container_type(template.config_mapping.container_type);
    let capability_config = service
        .get_capability_config(&template.identifier)
        .await
        .map_err(|err| {
            tracing::error!(
                client = %template.identifier,
                error = %err,
                "Failed to load client capability config"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or_default();

    let content = if descriptor.config_exists {
        match service.read_current_config(&template.identifier).await {
            Ok(content) => content,
            Err(err) => {
                tracing::warn!(
                    client = %template.identifier,
                    error = %err,
                    "Continuing list operation despite configuration read failure"
                );
                None
            }
        }
    } else {
        None
    };

    let (has_mcp_config, mcp_servers_count) = content
        .as_deref()
        .map(|raw| analyze_config_content(raw, &template.identifier, &template))
        .unwrap_or((false, 0));

    let last_modified = descriptor.config_path.as_deref().and_then(get_config_last_modified);

    Ok(ClientInfo {
        identifier: template.identifier.clone(),
        display_name,
        logo_url,
        category,
        enabled: descriptor.managed,
        managed: descriptor.managed,
        detected: descriptor.detection.is_some(),
        install_path: None,
        config_path: descriptor.config_path.unwrap_or_default(),
        config_exists: descriptor.config_exists,
        has_mcp_config,
        supported_transports,
        description,
        homepage_url,
        docs_url,
        support_url,
        config_mode: service
            .get_client_settings(&template.identifier)
            .await
            .ok()
            .and_then(|o| o.map(|(mode, _, _)| mode)),
        transport: service
            .get_client_settings(&template.identifier)
            .await
            .ok()
            .map(|o| o.map(|(_, tr, _)| tr).unwrap_or_else(|| "auto".to_string())),
        client_version: service
            .get_client_settings(&template.identifier)
            .await
            .ok()
            .and_then(|o| o.and_then(|(_, _, ver)| ver)),
        capability_source: capability_config.capability_source,
        selected_profile_ids: capability_config.selected_profile_ids,
        custom_profile_id: capability_config.custom_profile_id,
        config_type,
        last_detected: descriptor.detected_at.map(|dt| dt.to_rfc3339()),
        last_modified,
        mcp_servers_count: Some(mcp_servers_count),
        template: build_template_metadata(&template),
    })
}

// moved to POST /api/client/config/import

fn template_display_name(template: &ClientTemplate) -> String {
    template
        .display_name
        .clone()
        .unwrap_or_else(|| template.identifier.clone())
}

fn extract_logo_url(template: &ClientTemplate) -> Option<String> {
    metadata_string(template, "logo_url")
}

fn extract_category(template: &ClientTemplate) -> ClientCategory {
    metadata_string(template, "category")
        .as_deref()
        .and_then(ClientCategory::parse)
        .unwrap_or_default()
}

fn extract_supported_transports(template: &ClientTemplate) -> Vec<String> {
    let keymap = crate::clients::keymap::registry();
    keymap.advertise_supported(&template.config_mapping.format_rules)
}

fn build_template_metadata(template: &ClientTemplate) -> ClientTemplateMetadata {
    ClientTemplateMetadata {
        format: template.format.as_str().to_string(),
        protocol_revision: template.protocol_revision.clone(),
        storage: ClientTemplateStorageMetadata {
            kind: storage_kind_to_str(template.storage.kind).to_string(),
            path_strategy: template.storage.path_strategy.clone(),
        },
        container_type: convert_container_type(template.config_mapping.container_type)
            .unwrap_or(crate::api::models::client::ClientConfigType::Standard),
        merge_strategy: merge_strategy_to_str(template.config_mapping.merge_strategy).to_string(),
        keep_original_config: template.config_mapping.keep_original_config,
        managed_source: template.config_mapping.managed_source.clone().or_else(|| {
            template
                .config_mapping
                .managed_endpoint
                .as_ref()
                .and_then(|e| e.source.clone())
        }),
        description: metadata_string(template, "description"),
        homepage_url: metadata_string(template, "homepage_url"),
        docs_url: metadata_string(template, "docs_url"),
        support_url: metadata_string(template, "support_url"),
    }
}

fn storage_kind_to_str(kind: StorageKind) -> &'static str {
    match kind {
        StorageKind::File => "file",
        StorageKind::Kv => "kv",
        StorageKind::Custom => "custom",
    }
}

fn merge_strategy_to_str(strategy: MergeStrategy) -> &'static str {
    match strategy {
        MergeStrategy::Replace => "replace",
        MergeStrategy::DeepMerge => "deep_merge",
    }
}

fn convert_container_type(container: ContainerType) -> Option<crate::api::models::client::ClientConfigType> {
    use crate::api::models::client::ClientConfigType;
    match container {
        ContainerType::ObjectMap => Some(ClientConfigType::Standard),
        ContainerType::Array => Some(ClientConfigType::Array),
    }
}

fn parse_config_value(
    content: &str,
    template: &ClientTemplate,
) -> Value {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Value::Null;
    }

    match template.format {
        TemplateFormat::Json => serde_json::from_str(trimmed).unwrap_or(Value::Null),
        TemplateFormat::Json5 => json5::from_str(trimmed).unwrap_or(Value::Null),
        TemplateFormat::Toml => toml::from_str::<toml::Value>(trimmed)
            .ok()
            .and_then(|value| serde_json::to_value(value).ok())
            .unwrap_or(Value::Null),
        TemplateFormat::Yaml => serde_yaml::from_str(trimmed).unwrap_or(Value::Null),
    }
}

fn metadata_string(
    template: &ClientTemplate,
    key: &str,
) -> Option<String> {
    template
        .metadata
        .get(key)
        .and_then(|value| value.as_str().map(|s| s.to_string()))
}

fn build_render_options(request: &ClientConfigUpdateReq) -> ClientRenderOptions {
    let mode = map_mode(request.mode.clone());
    let profile_id = match &request.selected_config {
        ClientConfigSelected::Profile { profile_id } => Some(profile_id.clone()),
        ClientConfigSelected::Default => None,
        ClientConfigSelected::Servers { .. } => None,
    };
    let server_ids = match (&request.selected_config, mode.clone()) {
        (ClientConfigSelected::Servers { server_ids }, ConfigMode::Native) => Some(server_ids.clone()),
        _ => None,
    };

    ClientRenderOptions {
        client_id: request.identifier.clone(),
        mode,
        profile_id,
        server_ids,
        dry_run: request.preview,
    }
}

fn map_mode(mode: ClientConfigMode) -> ConfigMode {
    match mode {
        ClientConfigMode::Hosted => ConfigMode::Managed,
        ClientConfigMode::Transparent => ConfigMode::Native,
    }
}

fn build_update_preview(
    template: &ClientTemplate,
    execution: &TemplateExecutionResult,
) -> Value {
    match execution {
        TemplateExecutionResult::Applied { content, .. } => parse_config_value(content, template),
        TemplateExecutionResult::DryRun { content, .. } => parse_config_value(content, template),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::routes::AppState;
    use crate::clients::{CapabilitySource, source::{FileTemplateSource, TemplateRoot}};
    use crate::common::profile::ProfileType;
    use crate::config::{
        client::init::initialize_client_table,
        database::Database,
        models::Profile,
        profile::{self, init::initialize_profile_tables},
        server::init::initialize_server_tables,
    };
    use crate::core::{
        cache::{RedbCacheManager, manager::CacheConfig},
        models::Config,
        pool::UpstreamConnectionPool,
        profile::ConfigApplicationStateManager,
    };
    use crate::inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager};
    use crate::system::metrics::MetricsCollector;
    use axum::extract::{Query, State};
    use axum::Json;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::{path::PathBuf, sync::Arc, time::Duration};
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    struct TestContext {
        _temp_dir: TempDir,
        app_state: Arc<AppState>,
        client_service: Arc<ClientConfigService>,
        db_pool: sqlx::SqlitePool,
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

        initialize_server_tables(&db_pool).await.expect("init server tables");
        initialize_profile_tables(&db_pool).await.expect("init profile tables");
        initialize_client_table(&db_pool).await.expect("init client table");

        let database = Arc::new(Database {
            pool: db_pool.clone(),
            path: PathBuf::from(":memory:"),
        });

        let template_root = TemplateRoot::new(temp_dir.path().join("client-templates"));
        let template_source = Arc::new(FileTemplateSource::bootstrap(template_root).await.expect("template source"));
        let client_service = Arc::new(
            ClientConfigService::with_source(Arc::new(db_pool.clone()), template_source)
                .await
                .expect("client service"),
        );

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
            database: Some(database),
            config_application_state: Arc::new(ConfigApplicationStateManager::new()),
            redb_cache,
            unified_query: None,
            client_service: Some(client_service.clone()),
            inspector_calls: Arc::new(InspectorCallRegistry::new()),
            inspector_sessions: Arc::new(InspectorSessionManager::new()),
        });

        TestContext {
            _temp_dir: temp_dir,
            app_state,
            client_service,
            db_pool,
        }
    }

    async fn insert_shared_profile(pool: &sqlx::SqlitePool, name: &str) -> String {
        let profile = Profile::new(name.to_string(), ProfileType::Shared);
        profile::upsert_profile(pool, &profile).await.expect("upsert profile")
    }

    #[tokio::test]
    async fn update_capability_config_returns_updated_payload() {
        let context = create_test_context().await;
        let profile_id = insert_shared_profile(&context.db_pool, "profile-a").await;

        let Json(response) = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-a".to_string(),
                capability_source: CapabilitySource::Profiles,
                selected_profile_ids: vec![profile_id.clone()],
            }),
        )
        .await
        .expect("update capability config");

        assert!(response.success);
        let data = response.data.expect("response data");
        assert_eq!(data.identifier, "client-a");
        assert_eq!(data.capability_source, CapabilitySource::Profiles);
        assert_eq!(data.selected_profile_ids, vec![profile_id.clone()]);
        assert!(data.custom_profile_id.is_none());

        let stored = context
            .client_service
            .get_capability_config("client-a")
            .await
            .expect("stored config")
            .expect("stored data");
        assert_eq!(stored.capability_source, CapabilitySource::Profiles);
        assert_eq!(stored.selected_profile_ids, vec![profile_id]);
    }

    #[tokio::test]
    async fn get_capability_config_returns_stored_custom_profile() {
        let context = create_test_context().await;
        let config = context
            .client_service
            .set_capability_config("client-a", CapabilitySource::Custom, Vec::new())
            .await
            .expect("seed custom capability config");

        let Json(response) = get_capability_config(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "client-a".to_string(),
            }),
        )
        .await
        .expect("get capability config");

        assert!(response.success);
        let data = response.data.expect("response data");
        assert_eq!(data.identifier, "client-a");
        assert_eq!(data.capability_source, CapabilitySource::Custom);
        assert!(data.selected_profile_ids.is_empty());
        assert_eq!(data.custom_profile_id, config.custom_profile_id);
    }
}
