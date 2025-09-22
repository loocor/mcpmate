// HTTP handlers for client management API (template-driven)

use super::config::{analyze_config_content, get_config_last_modified};
use super::import::import_servers_from_config;
use crate::api::models::client::{
    ClientBackupActionData, ClientBackupActionResp, ClientCheckData, ClientCheckReq, ClientCheckResp, ClientConfigData,
    ClientConfigMode, ClientConfigReq, ClientConfigResp, ClientConfigRestoreReq, ClientConfigSelected,
    ClientConfigUpdateData, ClientConfigUpdateReq, ClientConfigUpdateResp, ClientImportedServer, ClientInfo,
    ClientTemplateMetadata, ClientTemplateStorageMetadata,
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

const DEFAULT_RUNTIMES: &[&str] = &["npx", "uvx", "docker", "binary"];

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

    let content = service.read_current_config(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to read config for {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

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

    let managed = service.is_client_managed(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to fetch managed state for {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let imported_servers = if request.import && config_exists {
        import_configured_servers(&app_state, &parsed_content).await?
    } else {
        None
    };

    let data = ClientConfigData {
        config_path: config_path.unwrap_or_default(),
        config_exists,
        content: parsed_content,
        has_mcp_config,
        mcp_servers_count,
        last_modified,
        config_type,
        imported_servers,
        template: build_template_metadata(&template),
        supported_transports: extract_supported_transports(&template),
        supported_runtimes: extract_supported_runtimes(&template),
        managed,
    };

    Ok(Json(ClientConfigResp::success(data)))
}

/// Handler for POST /api/client/config/apply
/// Generates and optionally applies configuration
pub async fn config_apply(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientConfigUpdateReq>,
) -> Result<Json<ClientConfigUpdateResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let template = service.get_client_template(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to load client template {}: {}", request.identifier, err);
        StatusCode::NOT_FOUND
    })?;

    let options = build_render_options(&request);
    let result = service.execute_render(options).await.map_err(|err| match err {
        ConfigError::ClientDisabled { identifier } => {
            tracing::warn!("Client {} is disabled; skipping configuration update", identifier);
            StatusCode::FORBIDDEN
        }
        other => {
            tracing::error!("Failed to execute render for {}: {}", request.identifier, other);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let preview = build_update_preview(&template, &result.execution);
    let mut warnings = Vec::new();
    if let Some(summary) = diff_summary(&result.execution) {
        warnings.push(summary);
    }

    let (applied, backup_path) = match &result.execution {
        TemplateExecutionResult::Applied { backup_path, .. } => (true, backup_path.clone()),
        _ => (false, None),
    };

    let data = ClientConfigUpdateData {
        success: true,
        preview,
        applied: !request.preview && applied,
        backup_path,
        warnings,
    };

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

pub(crate) fn get_client_service(state: &AppState) -> Result<Arc<ClientConfigService>, StatusCode> {
    state
        .client_service
        .as_ref()
        .cloned()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
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
    let supported_runtimes = extract_supported_runtimes(&template);
    let config_type = convert_container_type(template.config_mapping.container_type);

    let content = if descriptor.config_exists {
        service.read_current_config(&template.identifier).await.map_err(|err| {
            tracing::error!(
                "Failed to read config for {} while building list: {}",
                template.identifier,
                err
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
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
        supported_runtimes,
        config_mode: None,
        config_type,
        last_detected: descriptor.detected_at.map(|dt| dt.to_rfc3339()),
        last_modified,
        mcp_servers_count: Some(mcp_servers_count),
        template: build_template_metadata(&template),
    })
}

async fn import_configured_servers(
    state: &AppState,
    content: &Value,
) -> Result<Option<Vec<ClientImportedServer>>, StatusCode> {
    let db = state.database.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    import_servers_from_config(content, &db.pool)
        .await
        .map(Some)
        .map_err(|err| {
            tracing::error!("Failed to import servers: {}", err);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

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
    template.config_mapping.format_rules.keys().cloned().collect()
}

fn extract_supported_runtimes(template: &ClientTemplate) -> Vec<String> {
    match template.metadata.get("supported_runtimes") {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Some(Value::Object(map)) => map
            .values()
            .find_map(|value| value.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| DEFAULT_RUNTIMES.iter().map(|s| s.to_string()).collect()),
        _ => DEFAULT_RUNTIMES.iter().map(|s| s.to_string()).collect(),
    }
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
        managed_source: template
            .config_mapping
            .managed_source
            .clone()
            .or_else(|| template.config_mapping.managed_endpoint.as_ref().and_then(|e| e.source.clone())),
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
        ContainerType::Mixed => Some(ClientConfigType::Mixed),
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

fn diff_summary(execution: &TemplateExecutionResult) -> Option<String> {
    match execution {
        TemplateExecutionResult::DryRun { diff, .. } => diff.summary.clone(),
        _ => None,
    }
}
