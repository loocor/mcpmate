// HTTP handlers for client management API (template-driven)

use super::config::{analyze_config_content, get_config_last_modified};
use super::import::build_import_payload_from_value;
use crate::api::models::client::{
    ClientBackupActionData, ClientBackupActionResp, ClientCapabilityConfigData, ClientCapabilityConfigReq,
    ClientCapabilityConfigResp, ClientCheckData, ClientCheckReq, ClientCheckResp, ClientConfigData,
    ClientConfigImportData, ClientConfigImportReq, ClientConfigImportResp, ClientConfigMode, ClientConfigReq,
    ClientConfigResp, ClientConfigRestoreReq, ClientConfigSelected, ClientConfigUpdateData, ClientConfigUpdateReq,
    ClientConfigUpdateResp, ClientImportSummary, ClientImportedServer, ClientInfo, ClientTemplateMetadata,
    ClientTemplateStorageMetadata, ClientUnifyDirectExposureData,
};
use crate::api::routes::AppState;
use crate::audit::{AuditAction, AuditEvent, AuditStatus};
use crate::clients::models::{
    ClientTemplate, ContainerType, MergeStrategy, StorageKind, TemplateFormat, UnifyDirectExposureConfig,
};
use crate::clients::service::core::{ClientStateRow, RuntimeClientMetadata};
use crate::clients::service::settings::ActiveClientSettingsUpdate;
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
use serde_json::{Value, json};
use serde_yaml;
use std::sync::Arc;
use toml;

type ClientSettingsErrorResponse = (StatusCode, Json<crate::api::models::client::ClientSettingsUpdateResp>);

fn client_settings_error(
    status: StatusCode,
    message: impl Into<String>,
) -> ClientSettingsErrorResponse {
    (
        status,
        Json(crate::api::models::client::ClientSettingsUpdateResp::error_simple(
            "client_settings_invalid",
            &message.into(),
        )),
    )
}

/// Handler for GET /api/client
/// Detects and returns all clients, with optional template reload
pub async fn list(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ClientCheckReq>,
) -> Result<Json<ClientCheckResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

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
    let state = service.fetch_state(&request.identifier).await.ok().flatten();
    let template = service.get_client_template(&request.identifier).await.ok();
    if template.is_none() && state.is_none() {
        tracing::error!("Failed to resolve client config details for {}", request.identifier);
        return Err(StatusCode::NOT_FOUND);
    }

    let config_path = service.config_path(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to resolve config path for {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut warnings: Vec<String> = Vec::new();
    let content = match template.as_ref() {
        None => {
            warnings.push("Runtime template is unavailable for this client record".to_string());
            read_runtime_config(service.as_ref(), &request.identifier)
                .await
                .map_err(|err| {
                    tracing::error!(client = %request.identifier, error = %err, "Failed to read runtime config");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
        }
        Some(_) => match service.read_current_config(&request.identifier).await {
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
        },
    };

    let config_exists = content.is_some();
    let parsed_content = match (content.as_deref(), template.as_ref()) {
        (Some(raw), Some(template)) => parse_config_value(raw, template),
        (Some(raw), None) => parse_runtime_config_value(raw, config_path.as_deref()),
        (None, _) => Value::Null,
    };

    let (has_mcp_config, mcp_servers_count) = match (content.as_deref(), template.as_ref()) {
        (Some(raw), Some(template)) => analyze_config_content(raw, &request.identifier, template),
        _ => (false, 0),
    };

    let last_modified = config_path.as_deref().and_then(get_config_last_modified);

    let config_type = template
        .as_ref()
        .and_then(|template| convert_container_type(template.config_mapping.container_type))
        .or_else(|| infer_config_type_from_path(config_path.as_deref()));

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
    let runtime_metadata = state
        .as_ref()
        .map(ClientStateRow::runtime_client_metadata)
        .unwrap_or_default();
    let description = template
        .as_ref()
        .and_then(|template| metadata_string(template, "description"))
        .or_else(|| runtime_metadata.description.clone());
    let homepage_url = template
        .as_ref()
        .and_then(|template| metadata_string(template, "homepage_url"))
        .or_else(|| runtime_metadata.homepage_url.clone());
    let docs_url = template
        .as_ref()
        .and_then(|template| metadata_string(template, "docs_url"))
        .or_else(|| runtime_metadata.docs_url.clone());
    let support_url = template
        .as_ref()
        .and_then(|template| metadata_string(template, "support_url"))
        .or_else(|| runtime_metadata.support_url.clone());
    let logo_url = template
        .as_ref()
        .and_then(extract_logo_url)
        .or_else(|| runtime_metadata.logo_url.clone());

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
    let record_kind = state
        .as_ref()
        .map(|row| row.record_kind().as_str().to_string())
        .or_else(|| Some("template_known".to_string()));
    let governance_kind = state
        .as_ref()
        .map(|row| row.governance_kind().as_str().to_string())
        .or_else(|| Some("passive".to_string()));
    let connection_mode = state
        .as_ref()
        .map(|row| row.connection_mode().as_str().to_string())
        .or_else(|| {
            if config_path.as_deref().unwrap_or_default().is_empty() {
                Some("manual".to_string())
            } else {
                Some("local_config_detected".to_string())
            }
        });
    let governed_by_default_policy = state
        .as_ref()
        .map(|row| row.governed_by_default_policy())
        .unwrap_or(true);
    let approval_status = state.as_ref().map(|row| row.approval_status().to_string());
    let writable_config = service
        .has_verified_local_config_target(&request.identifier)
        .await
        .unwrap_or_else(|err| {
            tracing::warn!(client = %request.identifier, error = %err, "Failed to verify local config target");
            false
        });

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
        template: build_client_template_metadata(template.as_ref(), state.as_ref(), &runtime_metadata),
        supported_transports: extract_client_supported_transports(template.as_ref(), &runtime_metadata),
        managed,
        description,
        homepage_url,
        docs_url,
        support_url,
        logo_url,
        capability_source: capability_config.capability_source,
        selected_profile_ids: capability_config.selected_profile_ids,
        custom_profile_id: capability_config.custom_profile_id,
        approval_status,
        record_kind,
        governance_kind,
        connection_mode,
        governed_by_default_policy,
        writable_config,
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

    let existing_state = service.fetch_state(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, error = %err, "Failed to load client state before apply");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(state) = existing_state.as_ref() {
        if state.approval_status() != "approved" {
            tracing::warn!(
                client = %request.identifier,
                approval_status = %state.approval_status(),
                "Rejected config apply for non-approved client"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    match service.has_verified_local_config_target(&request.identifier).await {
        Ok(true) => {}
        Ok(false) => return Err(StatusCode::FORBIDDEN),
        Err(err) => {
            tracing::warn!(
                client = %request.identifier,
                error = %err,
                "Rejected config apply without a verified local config target"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let client_name = service.resolve_client_name(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, error = %err, "Failed to resolve client name");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match existing_state.as_ref() {
        Some(state) => {
            service
                .ensure_active_state_row_with_name(
                    &request.identifier,
                    &client_name,
                    None,
                    Some(state.approval_status()),
                )
                .await
                .map_err(|err| {
                    tracing::error!(client = %request.identifier, error = %err, "Failed to refresh active client state before apply");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
        None => {
            service
                .ensure_active_state_row_with_name(&request.identifier, &client_name, Some(true), Some("approved"))
                .await
                .map_err(|err| {
                    tracing::error!(client = %request.identifier, error = %err, "Failed to create active client state before apply");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

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
            ConfigError::DataAccessError(_)
            | ConfigError::PathResolutionError(_)
            | ConfigError::PathNotWritable { .. }
            | ConfigError::FileOperationError(_) => StatusCode::BAD_REQUEST,
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

    emit_client_audit_event(
        &app_state,
        AuditAction::ClientConfigApply,
        AuditStatus::Success,
        "/api/client/config/apply",
        &request.identifier,
        Some(request.identifier.clone()),
        Some(json!({
            "mode": request.mode,
            "preview": request.preview,
            "applied": applied,
            "scheduled": outcome.scheduled,
            "selected_config": request.selected_config,
        })),
        None,
    )
    .await;

    Ok(Json(ClientConfigUpdateResp::success(data)))
}

/// Handler for POST /api/client/config/restore
/// Restores configuration from a named backup snapshot
pub async fn config_restore(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientConfigRestoreReq>,
) -> Result<Json<ClientBackupActionResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    if let Ok(Some(state)) = service.fetch_state(&request.identifier).await {
        if state.is_pending_unknown() {
            tracing::warn!(
                client = %request.identifier,
                approval_status = %state.approval_status(),
                "Rejected config restore for pending unknown client"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let client_name = service.resolve_client_name(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, error = %err, "Failed to resolve client name");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    service
        .ensure_active_state_row_with_name(&request.identifier, &client_name, Some(true), Some("approved"))
        .await
        .map_err(|err| {
            tracing::error!(client = %request.identifier, error = %err, "Failed to activate client state before restore");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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

    emit_client_audit_event(
        &app_state,
        AuditAction::ClientConfigRestore,
        AuditStatus::Success,
        "/api/client/config/restore",
        &data.identifier,
        Some(data.backup.clone()),
        Some(json!({
            "backup": data.backup,
            "message": data.message,
        })),
        None,
    )
    .await;

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

    emit_client_audit_event(
        &app_state,
        AuditAction::ClientConfigImport,
        AuditStatus::Success,
        "/api/client/config/import",
        &request.identifier,
        Some(request.identifier.clone()),
        Some(json!({
            "preview": request.preview,
            "profile_id": request.profile_id,
            "imported_count": data.summary.imported_count,
            "skipped_count": data.summary.skipped_count,
            "failed_count": data.summary.failed_count,
            "scheduled": data.scheduled,
        })),
        None,
    )
    .await;

    Ok(Json(ClientConfigImportResp::success(data)))
}

pub(crate) fn get_client_service(state: &AppState) -> Result<Arc<ClientConfigService>, StatusCode> {
    state
        .client_service
        .as_ref()
        .cloned()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

struct BoundClientRuntimeState {
    effective_mode: String,
    unify_workspace: Option<UnifyDirectExposureConfig>,
}

impl BoundClientRuntimeState {
    fn should_emit_managed_visibility_change(&self, requested: bool) -> bool {
        requested && matches!(self.effective_mode.as_str(), "hosted" | "unify")
    }
}

async fn load_bound_client_runtime_state(
    service: &ClientConfigService,
    identifier: &str,
) -> Option<BoundClientRuntimeState> {
    let effective_mode = match service.get_effective_config_mode(identifier).await {
        Ok(mode) => mode,
        Err(err) => {
            tracing::warn!(client = %identifier, error = %err, "Failed to resolve effective config mode for runtime state sync");
            return None;
        }
    };

    let unify_workspace = if effective_mode == "unify" {
        match service.get_unify_direct_exposure_config(identifier).await {
            Ok(workspace) => Some(workspace.unwrap_or_default()),
            Err(err) => {
                tracing::error!(client = %identifier, error = %err, "Failed to load unify direct exposure config for runtime state sync");
                return None;
            }
        }
    } else {
        None
    };

    Some(BoundClientRuntimeState {
        effective_mode,
        unify_workspace,
    })
}

async fn sync_bound_client_runtime_state(
    service: &ClientConfigService,
    identifier: &str,
    notify_visible_change: bool,
) {
    let Some(runtime_state) = load_bound_client_runtime_state(service, identifier).await else {
        return;
    };

    if let Some(proxy) = crate::core::proxy::server::ProxyServer::global() {
        if let Ok(guard) = proxy.try_lock() {
            if let Err(err) = guard
                .apply_persisted_client_runtime_state(
                    identifier,
                    Some(runtime_state.effective_mode.clone()),
                    runtime_state.unify_workspace.clone(),
                )
                .await
            {
                tracing::warn!(client = %identifier, error = %err, mode = %runtime_state.effective_mode, "Failed to sync bound client runtime state");
            }
        }
    }

    if runtime_state.should_emit_managed_visibility_change(notify_visible_change) {
        crate::core::events::EventBus::global().publish(
            crate::core::events::Event::ClientVisibleDirectSurfaceChanged {
                client_id: identifier.to_string(),
            },
        );
    }
}

async fn emit_client_audit_event(
    app_state: &AppState,
    action: AuditAction,
    status: AuditStatus,
    route: &str,
    client_id: &str,
    target: Option<String>,
    data: Option<Value>,
    error_message: Option<String>,
) {
    let mut event = AuditEvent::new(action, status)
        .with_http_route("POST", route)
        .with_client_id(client_id.to_string());

    if let Some(target) = target {
        event = event.with_target(target);
    }
    if let Some(data) = data {
        event = event.with_data(data);
    }
    if let Some(error_message) = error_message {
        event = event.with_error(None::<String>, error_message);
    }

    crate::audit::interceptor::emit_event(app_state.audit_service.as_ref(), event.build()).await;
}

/// PATCH/POST /api/client/update - partial update client settings
pub async fn update_settings(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<crate::api::models::client::ClientSettingsUpdateReq>,
) -> Result<Json<crate::api::models::client::ClientSettingsUpdateResp>, ClientSettingsErrorResponse> {
    let service =
        get_client_service(&app_state).map_err(|status| client_settings_error(status, "Client service unavailable"))?;

    tracing::info!(
        client = %request.identifier,
        config_mode = ?request.config_mode,
        transport = ?request.transport,
        client_version = ?request.client_version,
        display_name = ?request.display_name,
        connection_mode = ?request.connection_mode,
        config_path = ?request.config_path,
        "update_settings: received request"
    );

    let settings_result = service
        .set_active_client_settings(
            &request.identifier,
            ActiveClientSettingsUpdate {
                display_name: request.display_name.clone(),
                config_mode: request.config_mode.clone(),
                transport: request.transport.clone(),
                client_version: request.client_version.clone(),
                connection_mode: request.connection_mode.clone(),
                config_path: request.config_path.clone(),
                description: request.description.clone(),
                homepage_url: request.homepage_url.clone(),
                docs_url: request.docs_url.clone(),
                support_url: request.support_url.clone(),
                logo_url: request.logo_url.clone(),
                supported_transports: request.supported_transports.clone(),
            },
        )
        .await
        .map_err(|err| {
            let status = match err {
                ConfigError::DataAccessError(_)
                | ConfigError::PathResolutionError(_)
                | ConfigError::PathNotWritable { .. }
                | ConfigError::FileOperationError(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            tracing::error!(
                client = %request.identifier,
                error = %err,
                status = %status.as_u16(),
                "Failed to update client settings"
            );
            client_settings_error(status, err.to_string())
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
            client_settings_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })?
        .unwrap_or((None, "auto".into(), None));
    let state = service
        .fetch_state(&request.identifier)
        .await
        .map_err(|err| {
            tracing::error!(client = %request.identifier, error = %err, "Failed to fetch updated client state");
            client_settings_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
        })?
        .ok_or_else(|| client_settings_error(StatusCode::NOT_FOUND, "Client state not found"))?;
    let runtime_metadata = state.runtime_client_metadata();

    let data = crate::api::models::client::ClientSettingsUpdateData {
        identifier: request.identifier,
        display_name: state.display_name().to_string(),
        config_mode: mode,
        transport,
        client_version: version,
        connection_mode: Some(state.connection_mode().as_str().to_string()),
        config_path: state.config_path().map(str::to_string),
        supported_transports: runtime_metadata.supported_transports.clone(),
        description: runtime_metadata.description.clone(),
        homepage_url: runtime_metadata.homepage_url.clone(),
        docs_url: runtime_metadata.docs_url.clone(),
        support_url: runtime_metadata.support_url.clone(),
        logo_url: runtime_metadata.logo_url.clone(),
    };

    emit_client_audit_event(
        &app_state,
        AuditAction::ClientSettingsUpdate,
        AuditStatus::Success,
        "/api/client/update",
        &data.identifier,
        Some(data.identifier.clone()),
        Some(json!({
            "config_mode": data.config_mode,
            "transport": data.transport,
            "client_version": data.client_version,
            "display_name": data.display_name,
            "connection_mode": data.connection_mode,
            "config_path": data.config_path,
            "supported_transports": data.supported_transports,
        })),
        None,
    )
    .await;

    let visible_mode_changed = settings_result.effective_mode_changed();
    sync_bound_client_runtime_state(&service, &data.identifier, visible_mode_changed).await;

    Ok(Json(crate::api::models::client::ClientSettingsUpdateResp::success(
        data,
    )))
}

pub async fn update_capability_config(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientCapabilityConfigReq>,
) -> Result<Json<ClientCapabilityConfigResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let (state, visible_surface_changed) = service
        .update_capability_config_state_and_invalidate(
            &request.identifier,
            request.capability_source,
            request.selected_profile_ids,
            request.unify_direct_exposure.map(Into::into),
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

    let data = ClientCapabilityConfigData {
        identifier: request.identifier,
        capability_source: state.capability_config.capability_source,
        selected_profile_ids: state.capability_config.selected_profile_ids,
        custom_profile_id: state.capability_config.custom_profile_id,
        unify_direct_exposure: ClientUnifyDirectExposureData {
            config: state.unify_direct_exposure.clone(),
            diagnostics: state.unify_direct_exposure_diagnostics.clone(),
        },
    };

    sync_bound_client_runtime_state(&service, &data.identifier, visible_surface_changed).await;

    emit_client_audit_event(
        &app_state,
        AuditAction::ClientCapabilityUpdate,
        AuditStatus::Success,
        "/api/client/capability-config",
        &data.identifier,
        Some(data.identifier.clone()),
        Some(json!({
            "capability_source": data.capability_source,
            "selected_profile_ids": data.selected_profile_ids,
            "custom_profile_id": data.custom_profile_id,
            "unify_direct_exposure": data.unify_direct_exposure,
        })),
        None,
    )
    .await;

    Ok(Json(ClientCapabilityConfigResp::success(data)))
}

pub async fn get_capability_config(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ClientConfigReq>,
) -> Result<Json<ClientCapabilityConfigResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let config = service
        .get_capability_config_state(&request.identifier)
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

    Ok(Json(ClientCapabilityConfigResp::success(ClientCapabilityConfigData {
        identifier: request.identifier,
        capability_source: config.capability_config.capability_source,
        selected_profile_ids: config.capability_config.selected_profile_ids,
        custom_profile_id: config.capability_config.custom_profile_id,
        unify_direct_exposure: ClientUnifyDirectExposureData {
            config: config.unify_direct_exposure,
            diagnostics: config.unify_direct_exposure_diagnostics,
        },
    })))
}

async fn descriptor_to_client_info(
    service: &ClientConfigService,
    descriptor: ClientDescriptor,
) -> Result<ClientInfo, StatusCode> {
    let template = descriptor.template.clone();
    let state = descriptor.state.clone();
    let runtime_metadata = state.runtime_client_metadata();
    let identifier = state.identifier().to_string();
    let display_name = template
        .as_ref()
        .map(template_display_name)
        .unwrap_or_else(|| state.display_name().to_string());
    let logo_url = template
        .as_ref()
        .and_then(extract_logo_url)
        .or_else(|| runtime_metadata.logo_url.clone());
    let category = template
        .as_ref()
        .map(extract_category)
        .or_else(|| runtime_metadata.category.as_deref().and_then(ClientCategory::parse))
        .unwrap_or_default();
    let supported_transports = extract_client_supported_transports(template.as_ref(), &runtime_metadata);
    let description = template
        .as_ref()
        .and_then(|template| metadata_string(template, "description"))
        .or_else(|| runtime_metadata.description.clone());
    let homepage_url = template
        .as_ref()
        .and_then(|template| metadata_string(template, "homepage_url"))
        .or_else(|| runtime_metadata.homepage_url.clone());
    let docs_url = template
        .as_ref()
        .and_then(|template| metadata_string(template, "docs_url"))
        .or_else(|| runtime_metadata.docs_url.clone());
    let support_url = template
        .as_ref()
        .and_then(|template| metadata_string(template, "support_url"))
        .or_else(|| runtime_metadata.support_url.clone());
    let config_type = template
        .as_ref()
        .and_then(|template| convert_container_type(template.config_mapping.container_type))
        .or_else(|| infer_config_type_from_path(descriptor.config_path.as_deref()));
    let capability_config = service
        .get_capability_config(&identifier)
        .await
        .map_err(|err| {
            tracing::error!(
                client = %identifier,
                error = %err,
                "Failed to load client capability config"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .unwrap_or_default();

    let content = if descriptor.config_exists {
        match template.as_ref() {
            Some(_) => match service.read_current_config(&identifier).await {
                Ok(content) => content,
                Err(err) => {
                    tracing::warn!(
                        client = %identifier,
                        error = %err,
                        "Continuing list operation despite configuration read failure"
                    );
                    None
                }
            },
            None => read_runtime_config(service, &identifier).await.unwrap_or(None),
        }
    } else {
        None
    };

    let (has_mcp_config, mcp_servers_count) = match (content.as_deref(), template.as_ref()) {
        (Some(raw), Some(template)) => analyze_config_content(raw, &identifier, template),
        _ => (false, 0),
    };

    let last_modified = descriptor.config_path.as_deref().and_then(get_config_last_modified);

    let approval_status = Some(state.approval_status().to_string());
    let record_kind = Some(state.record_kind().as_str().to_string());
    let governance_kind = Some(state.governance_kind().as_str().to_string());
    let connection_mode = Some(state.connection_mode().as_str().to_string());
    let governed_by_default_policy = state.governed_by_default_policy();
    let config_mode = service
        .get_client_settings(state.identifier())
        .await
        .ok()
        .and_then(|o| o.and_then(|(mode, _, _)| mode));
    let writable_config = service
        .has_verified_local_config_target(state.identifier())
        .await
        .unwrap_or(false);
    let template_id = state.template_identifier().map(|id| id.to_string());
    let template_known = state.is_template_known();
    let pending_approval = approval_status.as_deref() == Some("pending");

    Ok(ClientInfo {
        identifier,
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
        config_mode,
        transport: service
            .get_client_settings(state.identifier())
            .await
            .ok()
            .map(|o| o.map(|(_, tr, _)| tr).unwrap_or_else(|| "auto".to_string())),
        client_version: service
            .get_client_settings(state.identifier())
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
        template: build_client_template_metadata(template.as_ref(), Some(&state), &runtime_metadata),
        approval_status,
        record_kind,
        governance_kind,
        connection_mode,
        governed_by_default_policy,
        writable_config,
        template_id,
        template_known,
        pending_approval,
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

fn extract_client_supported_transports(
    template: Option<&ClientTemplate>,
    runtime_metadata: &RuntimeClientMetadata,
) -> Vec<String> {
    if !runtime_metadata.supported_transports.is_empty() {
        return runtime_metadata.supported_transports.clone();
    }

    match template {
        Some(template) => extract_supported_transports(template),
        None => runtime_metadata.supported_transports.clone(),
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

fn build_runtime_template_metadata(
    state: Option<&ClientStateRow>,
    runtime_metadata: &RuntimeClientMetadata,
) -> ClientTemplateMetadata {
    ClientTemplateMetadata {
        format: "runtime_record".to_string(),
        protocol_revision: None,
        storage: ClientTemplateStorageMetadata {
            kind: if state.map(ClientStateRow::has_local_config_target).unwrap_or(false) {
                "file".to_string()
            } else {
                "custom".to_string()
            },
            path_strategy: None,
        },
        container_type: crate::api::models::client::ClientConfigType::Standard,
        merge_strategy: "replace".to_string(),
        keep_original_config: false,
        managed_source: None,
        description: runtime_metadata.description.clone(),
        homepage_url: runtime_metadata.homepage_url.clone(),
        docs_url: runtime_metadata.docs_url.clone(),
        support_url: runtime_metadata.support_url.clone(),
    }
}

fn build_client_template_metadata(
    template: Option<&ClientTemplate>,
    state: Option<&ClientStateRow>,
    runtime_metadata: &RuntimeClientMetadata,
) -> ClientTemplateMetadata {
    match template {
        Some(template) => build_template_metadata(template),
        None => build_runtime_template_metadata(state, runtime_metadata),
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

fn parse_runtime_config_value(
    content: &str,
    config_path: Option<&str>,
) -> Value {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Value::Null;
    }

    match infer_template_format_from_path(config_path) {
        Some(TemplateFormat::Json) => {
            serde_json::from_str(trimmed).unwrap_or_else(|_| Value::String(content.to_string()))
        }
        Some(TemplateFormat::Json5) => json5::from_str(trimmed).unwrap_or_else(|_| Value::String(content.to_string())),
        Some(TemplateFormat::Toml) => toml::from_str::<toml::Value>(trimmed)
            .ok()
            .and_then(|value| serde_json::to_value(value).ok())
            .unwrap_or_else(|| Value::String(content.to_string())),
        Some(TemplateFormat::Yaml) => {
            serde_yaml::from_str(trimmed).unwrap_or_else(|_| Value::String(content.to_string()))
        }
        None => Value::String(content.to_string()),
    }
}

fn infer_template_format_from_path(config_path: Option<&str>) -> Option<TemplateFormat> {
    let path = config_path?.to_ascii_lowercase();
    if path.ends_with(".json5") {
        Some(TemplateFormat::Json5)
    } else if path.ends_with(".json") {
        Some(TemplateFormat::Json)
    } else if path.ends_with(".toml") {
        Some(TemplateFormat::Toml)
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        Some(TemplateFormat::Yaml)
    } else {
        None
    }
}

fn infer_config_type_from_path(config_path: Option<&str>) -> Option<crate::api::models::client::ClientConfigType> {
    infer_template_format_from_path(config_path).map(|_| crate::api::models::client::ClientConfigType::Standard)
}

async fn read_runtime_config(
    service: &ClientConfigService,
    client_id: &str,
) -> crate::clients::error::ConfigResult<Option<String>> {
    let Some(path) = service.config_path(client_id).await? else {
        return Ok(None);
    };

    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Ok(Some(content)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(ConfigError::IoError(err)),
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
        ClientConfigMode::Unify => ConfigMode::Managed,
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
    use crate::clients::{
        CapabilitySource,
        source::{ClientConfigSource, DbTemplateSource, FileTemplateSource, TemplateRoot},
    };
    use crate::common::profile::{ProfileRole, ProfileType};
    use crate::config::{
        client::init::{initialize_client_table, initialize_system_settings_table},
        database::Database,
        models::Profile,
        profile::{self, init::initialize_profile_tables},
        server::init::initialize_server_tables,
    };
    use crate::core::{
        cache::{RedbCacheManager, manager::CacheConfig},
        events::{Event, EventBus},
        models::Config,
        pool::UpstreamConnectionPool,
        profile::ConfigApplicationStateManager,
    };
    use crate::inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager};
    use crate::system::metrics::MetricsCollector;
    use axum::Json;
    use axum::extract::{Path, Query, State};
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

    async fn wait_for_client_visible_change_event(
        rx: &mut tokio::sync::broadcast::Receiver<Event>,
        client_id: &str,
    ) {
        let expected = client_id.to_string();
        tokio::time::timeout(Duration::from_secs(2), async move {
            loop {
                match rx.recv().await {
                    Ok(Event::ClientVisibleDirectSurfaceChanged { client_id }) if client_id == expected => {
                        return;
                    }
                    Ok(_) => continue,
                    Err(err) => panic!("event receiver failed: {err}"),
                }
            }
        })
        .await
        .expect("expected client visible change event");
    }

    async fn assert_no_client_visible_change_event(
        rx: &mut tokio::sync::broadcast::Receiver<Event>,
        client_id: &str,
    ) {
        let expected = client_id.to_string();
        let result = tokio::time::timeout(Duration::from_millis(200), async move {
            loop {
                match rx.recv().await {
                    Ok(Event::ClientVisibleDirectSurfaceChanged { client_id }) if client_id == expected => {
                        panic!("unexpected client visible change event")
                    }
                    Ok(_) => continue,
                    Err(err) => panic!("event receiver failed: {err}"),
                }
            }
        })
        .await;

        assert!(result.is_err(), "unexpected client visible change event arrived in timeout window");
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
        initialize_system_settings_table(&db_pool)
            .await
            .expect("init system settings table");

        let database = Arc::new(Database {
            pool: db_pool.clone(),
            path: PathBuf::from(":memory:"),
        });

        let template_root = TemplateRoot::new(temp_dir.path().join("client-templates"));
        let template_source = Arc::new(
            FileTemplateSource::bootstrap(template_root)
                .await
                .expect("template source"),
        );
        ClientConfigService::seed_runtime_template_snapshots(&db_pool, template_source.as_ref())
            .await
            .expect("seed runtime templates");
        ClientConfigService::seed_client_runtime_rows(&db_pool, template_source.as_ref())
            .await
            .expect("seed runtime rows");
        let runtime_source: Arc<dyn ClientConfigSource> =
            Arc::new(DbTemplateSource::new(Arc::new(db_pool.clone())).expect("runtime source"));
        let client_service = Arc::new(
            ClientConfigService::with_source(Arc::new(db_pool.clone()), runtime_source)
                .await
                .expect("client service"),
        );
        crate::core::capability::naming::initialize(db_pool.clone());

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
            audit_database: None,
            audit_service: None,
            config_application_state: Arc::new(ConfigApplicationStateManager::new()),
            redb_cache,
            unified_query: None,
            client_service: Some(client_service.clone()),
            inspector_calls: Arc::new(InspectorCallRegistry::new()),
            inspector_sessions: Arc::new(InspectorSessionManager::new()),
            oauth_manager: Some(Arc::new(crate::core::oauth::OAuthManager::new(db_pool.clone()))),
        });

        TestContext {
            _temp_dir: temp_dir,
            app_state,
            client_service,
            db_pool,
        }
    }

    async fn insert_shared_profile(
        pool: &sqlx::SqlitePool,
        name: &str,
    ) -> String {
        let profile = Profile::new(name.to_string(), ProfileType::Shared);
        profile::upsert_profile(pool, &profile).await.expect("upsert profile")
    }

    async fn insert_active_shared_profile(
        pool: &sqlx::SqlitePool,
        name: &str,
    ) -> String {
        let mut profile = Profile::new(name.to_string(), ProfileType::Shared);
        profile.is_active = true;
        profile.is_default = true;
        profile.multi_select = true;
        profile::upsert_profile(pool, &profile)
            .await
            .expect("upsert active profile")
    }

    async fn insert_unify_server(
        pool: &sqlx::SqlitePool,
        id: &str,
        name: &str,
        eligible: bool,
        tool_names: &[&str],
    ) {
        sqlx::query(
            r#"
            INSERT INTO server_config (id, name, server_type, command, capabilities, enabled, unify_direct_exposure_eligible)
            VALUES (?, ?, 'stdio', 'server-binary', 'tools', 1, ?)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(if eligible { 1 } else { 0 })
        .execute(pool)
        .await
        .expect("insert unify server");

        for tool_name in tool_names {
            crate::config::server::tools::upsert_server_tool(pool, id, name, tool_name, Some("tool"), None)
                .await
                .expect("upsert server tool");
        }
    }

    async fn insert_unify_non_tool_capabilities(
        pool: &sqlx::SqlitePool,
        server_id: &str,
        server_name: &str,
        prompt_names: &[&str],
        resource_uris: &[&str],
        template_uris: &[&str],
    ) {
        for prompt_name in prompt_names {
            sqlx::query(
                "INSERT INTO server_prompts (id, server_id, server_name, prompt_name, unique_name) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(format!("{server_id}-prompt-{prompt_name}"))
            .bind(server_id)
            .bind(server_name)
            .bind(prompt_name)
            .bind(format!("{}_{}", server_name.to_lowercase().replace(' ', "_"), prompt_name))
            .execute(pool)
            .await
            .expect("insert server prompt");
        }

        for resource_uri in resource_uris {
            sqlx::query(
                "INSERT INTO server_resources (id, server_id, server_name, resource_uri, unique_uri, name) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(format!("{server_id}-resource-{resource_uri}"))
            .bind(server_id)
            .bind(server_name)
            .bind(resource_uri)
            .bind(format!("{}:{}", server_name.to_lowercase().replace(' ', "_"), resource_uri))
            .bind(resource_uri)
            .execute(pool)
            .await
            .expect("insert server resource");
        }

        for template_uri in template_uris {
            sqlx::query(
                "INSERT INTO server_resource_templates (id, server_id, server_name, uri_template, unique_name, name) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(format!("{server_id}-template-{template_uri}"))
            .bind(server_id)
            .bind(server_name)
            .bind(template_uri)
            .bind(format!("{}_{}", server_name.to_lowercase().replace(' ', "_"), template_uri))
            .bind(template_uri)
            .execute(pool)
            .await
            .expect("insert server resource template");
        }
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
                unify_direct_exposure: None,
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
        assert_eq!(
            data.unify_direct_exposure.config.route_mode,
            crate::clients::models::UnifyRouteMode::BrokerOnly
        );

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
        assert_eq!(
            data.unify_direct_exposure.config.route_mode,
            crate::clients::models::UnifyRouteMode::BrokerOnly
        );
    }

    #[tokio::test]
    async fn capability_config_roundtrips_unify_route_only() {
        let context = create_test_context().await;

        let Json(response) = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-route-only".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::BrokerOnly,
                    selected_server_ids: Vec::new(),
                    selected_tool_surfaces: Vec::new(),
                    selected_prompt_surfaces: Vec::new(),
                    selected_resource_surfaces: Vec::new(),
                    selected_template_surfaces: Vec::new(),
                }),
            }),
        )
        .await
        .expect("update route-only capability config");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.config.route_mode,
            crate::clients::models::UnifyRouteMode::BrokerOnly
        );
        assert!(data.unify_direct_exposure.config.selected_server_ids.is_empty());
        assert!(data.unify_direct_exposure.config.selected_tool_surfaces.is_empty());
        assert!(data.unify_direct_exposure.diagnostics.invalid_server_ids.is_empty());
        assert!(data.unify_direct_exposure.diagnostics.invalid_tool_surfaces.is_empty());
    }

    #[tokio::test]
    async fn capability_config_roundtrips_unify_server_selection() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-eligible",
            "Eligible Server",
            true,
            &["tool-a", "tool-b"],
        )
        .await;

        let Json(response) = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-server-live".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::ServerLive,
                    selected_server_ids: vec!["server-eligible".to_string()],
                    selected_tool_surfaces: Vec::new(),
                    selected_prompt_surfaces: Vec::new(),
                    selected_resource_surfaces: Vec::new(),
                    selected_template_surfaces: Vec::new(),
                }),
            }),
        )
        .await
        .expect("update server-live capability config");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.config.route_mode,
            crate::clients::models::UnifyRouteMode::ServerLive
        );
        assert_eq!(
            data.unify_direct_exposure.config.selected_server_ids,
            vec!["server-eligible".to_string()]
        );
        assert!(data.unify_direct_exposure.config.selected_tool_surfaces.is_empty());
        assert!(data.unify_direct_exposure.diagnostics.invalid_server_ids.is_empty());
    }

    #[tokio::test]
    async fn capability_config_roundtrips_unify_tool_selection() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-tools",
            "Tool Server",
            true,
            &["tool-a", "tool-b"],
        )
        .await;

        let Json(response) = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-tool-live".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::CapabilityLevel,
                    selected_server_ids: Vec::new(),
                    selected_tool_surfaces: vec![crate::clients::models::UnifyDirectToolSurface {
                        server_id: "server-tools".to_string(),
                        tool_name: "tool-b".to_string(),
                    }],
                    selected_prompt_surfaces: Vec::new(),
                    selected_resource_surfaces: Vec::new(),
                    selected_template_surfaces: Vec::new(),
                }),
            }),
        )
        .await
        .expect("update capability-level config");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.config.selected_tool_surfaces,
            vec![crate::clients::models::UnifyDirectToolSurface {
                server_id: "server-tools".to_string(),
                tool_name: "tool-b".to_string(),
            }]
        );
        assert!(data.unify_direct_exposure.diagnostics.invalid_tool_surfaces.is_empty());
    }

    #[tokio::test]
    async fn capability_config_roundtrips_unify_prompt_resource_and_template_selection() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-mixed",
            "Mixed Server",
            true,
            &["tool-a"],
        )
        .await;
        insert_unify_non_tool_capabilities(
            &context.db_pool,
            "server-mixed",
            "Mixed Server",
            &["prompt-a"],
            &["resource-a"],
            &["template-a"],
        )
        .await;

        let Json(response) = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-mixed-live".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::CapabilityLevel,
                    selected_server_ids: Vec::new(),
                    selected_tool_surfaces: Vec::new(),
                    selected_prompt_surfaces: vec![crate::clients::models::UnifyDirectPromptSurface {
                        server_id: "server-mixed".to_string(),
                        prompt_name: "prompt-a".to_string(),
                    }],
                    selected_resource_surfaces: vec![crate::clients::models::UnifyDirectResourceSurface {
                        server_id: "server-mixed".to_string(),
                        resource_uri: "resource-a".to_string(),
                    }],
                    selected_template_surfaces: vec![crate::clients::models::UnifyDirectTemplateSurface {
                        server_id: "server-mixed".to_string(),
                        uri_template: "template-a".to_string(),
                    }],
                }),
            }),
        )
        .await
        .expect("update mixed capability-level config");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.config.selected_prompt_surfaces,
            vec![crate::clients::models::UnifyDirectPromptSurface {
                server_id: "server-mixed".to_string(),
                prompt_name: "prompt-a".to_string(),
            }]
        );
        assert_eq!(
            data.unify_direct_exposure.config.selected_resource_surfaces,
            vec![crate::clients::models::UnifyDirectResourceSurface {
                server_id: "server-mixed".to_string(),
                resource_uri: "resource-a".to_string(),
            }]
        );
        assert_eq!(
            data.unify_direct_exposure.config.selected_template_surfaces,
            vec![crate::clients::models::UnifyDirectTemplateSurface {
                server_id: "server-mixed".to_string(),
                uri_template: "template-a".to_string(),
            }]
        );
        assert!(data.unify_direct_exposure.diagnostics.invalid_prompt_surfaces.is_empty());
        assert!(data.unify_direct_exposure.diagnostics.invalid_resource_surfaces.is_empty());
        assert!(data.unify_direct_exposure.diagnostics.invalid_template_surfaces.is_empty());
    }

    #[tokio::test]
    async fn capability_config_unify_tool_selection_uses_globally_enabled_servers_for_activated_mode() {
        let context = create_test_context().await;
        insert_active_shared_profile(&context.db_pool, "active-profile-without-target-server").await;
        insert_unify_server(
            &context.db_pool,
            "server-global-only",
            "Global Only Server",
            true,
            &["tool-a", "tool-b"],
        )
        .await;

        let Json(response) = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-tool-live-global-enabled".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::CapabilityLevel,
                    selected_server_ids: Vec::new(),
                    selected_tool_surfaces: vec![crate::clients::models::UnifyDirectToolSurface {
                        server_id: "server-global-only".to_string(),
                        tool_name: "tool-b".to_string(),
                    }],
                    selected_prompt_surfaces: Vec::new(),
                    selected_resource_surfaces: Vec::new(),
                    selected_template_surfaces: Vec::new(),
                }),
            }),
        )
        .await
        .expect("update capability-level config against globally enabled server");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.config.selected_tool_surfaces,
            vec![crate::clients::models::UnifyDirectToolSurface {
                server_id: "server-global-only".to_string(),
                tool_name: "tool-b".to_string(),
            }]
        );
        assert!(data.unify_direct_exposure.diagnostics.invalid_server_ids.is_empty());
        assert!(data.unify_direct_exposure.diagnostics.invalid_tool_surfaces.is_empty());
    }

    #[tokio::test]
    async fn hosted_capability_source_switch_emits_visible_change_event() {
        let context = create_test_context().await;
        let profile = Profile {
            id: Some("PROFHOSTED001".to_string()),
            name: "Hosted Profile".to_string(),
            description: None,
            profile_type: ProfileType::Shared,
            role: ProfileRole::User,
            multi_select: false,
            priority: 0,
            is_active: false,
            is_default: false,
            created_at: None,
            updated_at: None,
        };
        profile::upsert_profile(&context.db_pool, &profile)
            .await
            .expect("insert hosted profile");
        context
            .client_service
            .set_active_client_settings(
                "client-a",
                ActiveClientSettingsUpdate {
                    config_mode: Some("hosted".to_string()),
                    ..ActiveClientSettingsUpdate::default()
                },
            )
            .await
            .expect("seed hosted mode");

        let mut rx = EventBus::global().subscribe_async();
        let Json(response) = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-a".to_string(),
                capability_source: CapabilitySource::Profiles,
                selected_profile_ids: vec!["PROFHOSTED001".to_string()],
                unify_direct_exposure: None,
            }),
        )
        .await
        .expect("switch hosted capability source");

        assert!(response.success);
        wait_for_client_visible_change_event(&mut rx, "client-a").await;
    }

    #[tokio::test]
    async fn mode_switch_to_hosted_emits_visible_change_event() {
        let context = create_test_context().await;
        context
            .client_service
            .set_active_client_settings(
                "client-a",
                ActiveClientSettingsUpdate {
                    config_mode: Some("unify".to_string()),
                    ..ActiveClientSettingsUpdate::default()
                },
            )
            .await
            .expect("seed unify mode");

        let mut rx = EventBus::global().subscribe_async();
        let Json(response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "client-a".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: None,
                client_version: None,
                display_name: None,
                connection_mode: None,
                config_path: None,
                supported_transports: None,
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("switch mode to hosted");

        assert!(response.success);
        wait_for_client_visible_change_event(&mut rx, "client-a").await;
    }

    #[tokio::test]
    async fn mode_switch_to_transparent_does_not_emit_visible_change_event() {
        let context = create_test_context().await;
        context
            .client_service
            .set_active_client_settings(
                "client-a",
                ActiveClientSettingsUpdate {
                    config_mode: Some("hosted".to_string()),
                    ..ActiveClientSettingsUpdate::default()
                },
            )
            .await
            .expect("seed hosted mode");

        let mut rx = EventBus::global().subscribe_async();
        let Json(response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "client-a".to_string(),
                config_mode: Some("transparent".to_string()),
                transport: None,
                client_version: None,
                display_name: None,
                connection_mode: None,
                config_path: None,
                supported_transports: None,
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("switch mode to transparent");

        assert!(response.success);
        assert_no_client_visible_change_event(&mut rx, "client-a").await;
    }

    #[tokio::test]
    async fn capability_config_filters_stale_and_ineligible_unify_selections() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-eligible",
            "Eligible Server",
            true,
            &["tool-a"],
        )
        .await;
        insert_unify_server(
            &context.db_pool,
            "server-ineligible",
            "Ineligible Server",
            false,
            &["tool-x"],
        )
        .await;

        let Json(response) = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-invalid".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::CapabilityLevel,
                    selected_server_ids: vec![
                        "server-eligible".to_string(),
                        "server-ineligible".to_string(),
                        "server-missing".to_string(),
                    ],
                    selected_tool_surfaces: vec![
                        crate::clients::models::UnifyDirectToolSurface {
                            server_id: "server-eligible".to_string(),
                            tool_name: "tool-missing".to_string(),
                        },
                        crate::clients::models::UnifyDirectToolSurface {
                            server_id: "server-ineligible".to_string(),
                            tool_name: "tool-x".to_string(),
                        },
                        crate::clients::models::UnifyDirectToolSurface {
                            server_id: "server-eligible".to_string(),
                            tool_name: "tool-a".to_string(),
                        },
                    ],
                    selected_prompt_surfaces: Vec::new(),
                    selected_resource_surfaces: Vec::new(),
                    selected_template_surfaces: Vec::new(),
                }),
            }),
        )
        .await
        .expect("update invalid unify config");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.config.selected_server_ids,
            vec!["server-eligible".to_string()]
        );
        assert_eq!(
            data.unify_direct_exposure.config.selected_tool_surfaces,
            vec![crate::clients::models::UnifyDirectToolSurface {
                server_id: "server-eligible".to_string(),
                tool_name: "tool-a".to_string(),
            }]
        );
        assert_eq!(
            data.unify_direct_exposure.diagnostics.invalid_server_ids,
            vec!["server-ineligible".to_string(), "server-missing".to_string()]
        );
        assert_eq!(data.unify_direct_exposure.diagnostics.invalid_tool_surfaces.len(), 2);
        assert!(
            data.unify_direct_exposure
                .diagnostics
                .invalid_tool_surfaces
                .iter()
                .any(|item| item.server_id == "server-eligible"
                    && item.tool_name == "tool-missing"
                    && item.reason == "tool_not_found")
        );
        assert!(
            data.unify_direct_exposure
                .diagnostics
                .invalid_tool_surfaces
                .iter()
                .any(|item| item.server_id == "server-ineligible"
                    && item.tool_name == "tool-x"
                    && item.reason == "server_not_eligible_or_missing")
        );
    }

    #[tokio::test]
    async fn server_eligibility_change_prunes_server_live_direct_selection() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-prune-live",
            "Prune Live Server",
            true,
            &["tool-a"],
        )
        .await;

        let _ = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-prune-live".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::ServerLive,
                    selected_server_ids: vec!["server-prune-live".to_string()],
                    selected_tool_surfaces: Vec::new(),
                    selected_prompt_surfaces: Vec::new(),
                    selected_resource_surfaces: Vec::new(),
                    selected_template_surfaces: Vec::new(),
                }),
            }),
        )
        .await
        .expect("seed server-live capability config");

        let _ = crate::api::handlers::server::update_server(
            State(context.app_state.clone()),
            Json(crate::api::models::server::ServerUpdateReq {
                id: "server-prune-live".to_string(),
                kind: None,
                command: None,
                url: None,
                args: None,
                env: None,
                headers: None,
                profile_ids: None,
                enabled: None,
                pending_import: None,
                registry_server_id: None,
                meta: None,
                unify_direct_exposure_eligible: Some(false),
            }),
        )
        .await
        .expect("disable direct exposure eligibility");

        let Json(response) = get_capability_config(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "client-prune-live".to_string(),
            }),
        )
        .await
        .expect("load pruned capability config");

        let data = response.data.expect("response data");
        assert!(data.unify_direct_exposure.config.selected_server_ids.is_empty());
    }

    #[tokio::test]
    async fn server_eligibility_change_prunes_capability_level_direct_surfaces() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-prune-capabilities",
            "Prune Capability Server",
            true,
            &["tool-a"],
        )
        .await;
        insert_unify_non_tool_capabilities(
            &context.db_pool,
            "server-prune-capabilities",
            "Prune Capability Server",
            &["prompt-a"],
            &["resource-a"],
            &["template-a"],
        )
        .await;

        let _ = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-prune-capabilities".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::CapabilityLevel,
                    selected_server_ids: Vec::new(),
                    selected_tool_surfaces: vec![crate::clients::models::UnifyDirectToolSurface {
                        server_id: "server-prune-capabilities".to_string(),
                        tool_name: "tool-a".to_string(),
                    }],
                    selected_prompt_surfaces: vec![crate::clients::models::UnifyDirectPromptSurface {
                        server_id: "server-prune-capabilities".to_string(),
                        prompt_name: "prompt-a".to_string(),
                    }],
                    selected_resource_surfaces: vec![crate::clients::models::UnifyDirectResourceSurface {
                        server_id: "server-prune-capabilities".to_string(),
                        resource_uri: "resource-a".to_string(),
                    }],
                    selected_template_surfaces: vec![crate::clients::models::UnifyDirectTemplateSurface {
                        server_id: "server-prune-capabilities".to_string(),
                        uri_template: "template-a".to_string(),
                    }],
                }),
            }),
        )
        .await
        .expect("seed capability-level direct surfaces");

        let _ = crate::api::handlers::server::update_server(
            State(context.app_state.clone()),
            Json(crate::api::models::server::ServerUpdateReq {
                id: "server-prune-capabilities".to_string(),
                kind: None,
                command: None,
                url: None,
                args: None,
                env: None,
                headers: None,
                profile_ids: None,
                enabled: None,
                pending_import: None,
                registry_server_id: None,
                meta: None,
                unify_direct_exposure_eligible: Some(false),
            }),
        )
        .await
        .expect("disable capability-level direct eligibility");

        let Json(response) = get_capability_config(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "client-prune-capabilities".to_string(),
            }),
        )
        .await
        .expect("load pruned capability-level config");

        let data = response.data.expect("response data");
        assert!(data.unify_direct_exposure.config.selected_tool_surfaces.is_empty());
        assert!(data.unify_direct_exposure.config.selected_prompt_surfaces.is_empty());
        assert!(data.unify_direct_exposure.config.selected_resource_surfaces.is_empty());
        assert!(data.unify_direct_exposure.config.selected_template_surfaces.is_empty());
    }

    #[tokio::test]
    async fn server_disable_prunes_server_live_direct_selection() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-disable-live",
            "Disable Live Server",
            true,
            &["tool-a"],
        )
        .await;

        let _ = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-disable-live".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::ServerLive,
                    selected_server_ids: vec!["server-disable-live".to_string()],
                    selected_tool_surfaces: Vec::new(),
                    selected_prompt_surfaces: Vec::new(),
                    selected_resource_surfaces: Vec::new(),
                    selected_template_surfaces: Vec::new(),
                }),
            }),
        )
        .await
        .expect("seed server-live config before disable");

        let _ = crate::api::handlers::server::disable_server(
            State(context.app_state.clone()),
            Path("server-disable-live".to_string()),
            Query(std::collections::HashMap::new()),
        )
        .await
        .expect("disable server globally");

        let Json(response) = get_capability_config(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "client-disable-live".to_string(),
            }),
        )
        .await
        .expect("load server-live config after disable");

        let data = response.data.expect("response data");
        assert!(data.unify_direct_exposure.config.selected_server_ids.is_empty());
    }

    #[tokio::test]
    async fn server_disable_prunes_capability_level_direct_surfaces() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-disable-capabilities",
            "Disable Capability Server",
            true,
            &["tool-a"],
        )
        .await;
        insert_unify_non_tool_capabilities(
            &context.db_pool,
            "server-disable-capabilities",
            "Disable Capability Server",
            &["prompt-a"],
            &["resource-a"],
            &["template-a"],
        )
        .await;

        let _ = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-disable-capabilities".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::CapabilityLevel,
                    selected_server_ids: Vec::new(),
                    selected_tool_surfaces: vec![crate::clients::models::UnifyDirectToolSurface {
                        server_id: "server-disable-capabilities".to_string(),
                        tool_name: "tool-a".to_string(),
                    }],
                    selected_prompt_surfaces: vec![crate::clients::models::UnifyDirectPromptSurface {
                        server_id: "server-disable-capabilities".to_string(),
                        prompt_name: "prompt-a".to_string(),
                    }],
                    selected_resource_surfaces: vec![crate::clients::models::UnifyDirectResourceSurface {
                        server_id: "server-disable-capabilities".to_string(),
                        resource_uri: "resource-a".to_string(),
                    }],
                    selected_template_surfaces: vec![crate::clients::models::UnifyDirectTemplateSurface {
                        server_id: "server-disable-capabilities".to_string(),
                        uri_template: "template-a".to_string(),
                    }],
                }),
            }),
        )
        .await
        .expect("seed capability config before disable");

        let _ = crate::api::handlers::server::disable_server(
            State(context.app_state.clone()),
            Path("server-disable-capabilities".to_string()),
            Query(std::collections::HashMap::new()),
        )
        .await
        .expect("disable capability server globally");

        let Json(response) = get_capability_config(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "client-disable-capabilities".to_string(),
            }),
        )
        .await
        .expect("load capability config after disable");

        let data = response.data.expect("response data");
        assert!(data.unify_direct_exposure.config.selected_tool_surfaces.is_empty());
        assert!(data.unify_direct_exposure.config.selected_prompt_surfaces.is_empty());
        assert!(data.unify_direct_exposure.config.selected_resource_surfaces.is_empty());
        assert!(data.unify_direct_exposure.config.selected_template_surfaces.is_empty());
    }

    #[tokio::test]
    async fn config_details_supports_active_runtime_only_client() {
        let context = create_test_context().await;

        context
            .client_service
            .set_active_client_settings(
                "custom.runtime",
                ActiveClientSettingsUpdate {
                    config_mode: Some("hosted".to_string()),
                    connection_mode: Some("manual".to_string()),
                    supported_transports: Some(vec!["streamable_http".to_string()]),
                    description: Some("Runtime-only client".to_string()),
                    ..ActiveClientSettingsUpdate::default()
                },
            )
            .await
            .expect("create active runtime-only client");

        let Json(response) = config_details(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "custom.runtime".to_string(),
            }),
        )
        .await
        .expect("config details for runtime-only client");

        assert!(response.success);
        let data = response.data.expect("response data");
        assert_eq!(data.governance_kind.as_deref(), Some("active"));
        assert!(!data.governed_by_default_policy);
        assert!(!data.writable_config);
        assert!(data.warnings.is_empty());
        assert_eq!(data.supported_transports, vec!["streamable_http".to_string()]);
        assert_eq!(data.description.as_deref(), Some("Runtime-only client"));
        assert_eq!(data.template.format, "json");
        assert_eq!(data.template.storage.kind, "file");
        assert_eq!(data.record_kind.as_deref(), Some("template_known"));
    }

    #[tokio::test]
    async fn update_settings_persists_runtime_only_active_payload_fields() {
        let context = create_test_context().await;
        let config_path = context._temp_dir.path().join("custom-runtime-payload.json");
        tokio::fs::write(&config_path, "{}")
            .await
            .expect("seed payload config file");

        let Json(response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "custom.runtime.payload".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: Some("1.2.3".to_string()),
                display_name: Some("Custom Runtime".to_string()),
                connection_mode: Some("local_config_detected".to_string()),
                config_path: Some(config_path.to_string_lossy().to_string()),
                supported_transports: Some(vec!["streamable_http".to_string(), "stdio".to_string()]),
                description: Some("Custom runtime client".to_string()),
                homepage_url: Some("https://example.com".to_string()),
                docs_url: Some("https://example.com/docs".to_string()),
                support_url: Some("https://example.com/support".to_string()),
                logo_url: Some("https://example.com/logo.png".to_string()),
            }),
        )
        .await
        .expect("update settings");

        assert!(response.success);
        let data = response.data.expect("response data");
        assert_eq!(data.display_name, "Custom Runtime");
        assert_eq!(data.connection_mode.as_deref(), Some("local_config_detected"));
        assert_eq!(
            data.supported_transports,
            vec!["streamable_http".to_string(), "stdio".to_string()]
        );
        assert_eq!(data.description.as_deref(), Some("Custom runtime client"));

        let state = context
            .client_service
            .fetch_state("custom.runtime.payload")
            .await
            .expect("fetch state")
            .expect("state exists");
        assert_eq!(state.governance_kind().as_str(), "active");
        assert_eq!(state.record_kind().as_str(), "template_known");
        assert_eq!(state.display_name(), "Custom Runtime");
        assert_eq!(state.connection_mode().as_str(), "local_config_detected");
        assert_eq!(state.config_path(), Some(config_path.to_string_lossy().as_ref()));
        assert_eq!(state.template_identifier(), Some("custom.runtime.payload"));
        assert_eq!(
            state.runtime_client_metadata().homepage_url.as_deref(),
            Some("https://example.com")
        );

        let runtime_template = context
            .client_service
            .get_client_template("custom.runtime.payload")
            .await
            .expect("load runtime template");
        assert_eq!(runtime_template.identifier, "custom.runtime.payload");
        assert_eq!(runtime_template.display_name.as_deref(), Some("Custom Runtime"));
        assert_eq!(runtime_template.version.as_deref(), Some("1.2.3"));
        assert_eq!(
            runtime_template.config_mapping.container_keys,
            vec!["mcpServers".to_string()]
        );
        assert_eq!(
            runtime_template.config_mapping.managed_source.as_deref(),
            Some("runtime_active_client")
        );
        assert!(
            runtime_template
                .config_mapping
                .format_rules
                .contains_key("streamable_http")
        );
        assert!(runtime_template.config_mapping.format_rules.contains_key("stdio"));
        assert_eq!(
            metadata_string(&runtime_template, "homepage_url").as_deref(),
            Some("https://example.com")
        );
    }

    #[tokio::test]
    async fn config_details_prefers_persisted_supported_transports_for_template_client() {
        let context = create_test_context().await;
        let config_path = context._temp_dir.path().join("template-client.json");
        tokio::fs::write(&config_path, "{}")
            .await
            .expect("seed template client config file");

        let Json(update_response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "client-a".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("stdio".to_string()),
                client_version: None,
                display_name: Some("Client A".to_string()),
                connection_mode: Some("local_config_detected".to_string()),
                config_path: Some(config_path.to_string_lossy().to_string()),
                supported_transports: Some(vec!["stdio".to_string()]),
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("update template client settings");

        assert!(update_response.success);

        let Json(details_response) = config_details(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "client-a".to_string(),
            }),
        )
        .await
        .expect("load template client details");

        assert!(details_response.success);
        let details = details_response.data.expect("details data");
        assert_eq!(details.supported_transports, vec!["stdio".to_string()]);

        let Json(list_response) = list(
            State(context.app_state.clone()),
            Query(ClientCheckReq { refresh: false }),
        )
        .await
        .expect("list clients");

        let listed_client = list_response
            .data
            .expect("list data")
            .client
            .into_iter()
            .find(|client| client.identifier == "client-a")
            .expect("client-a in list response");
        assert_eq!(listed_client.supported_transports, vec!["stdio".to_string()]);
    }

    #[tokio::test]
    async fn update_settings_rejects_missing_local_config_target() {
        let context = create_test_context().await;
        let missing_path = context._temp_dir.path().join("missing-client.json");

        let result = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "custom.runtime.invalid".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: None,
                display_name: Some("Invalid Runtime".to_string()),
                connection_mode: Some("local_config_detected".to_string()),
                config_path: Some(missing_path.to_string_lossy().to_string()),
                supported_transports: Some(vec!["streamable_http".to_string()]),
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await;

        let (status, Json(response)) = result.expect_err("missing local config target should fail");
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let error = response.error.expect("error payload");
        assert!(error.message.contains("does not exist"));
    }

    #[tokio::test]
    async fn update_settings_clears_config_path_when_switching_to_manual() {
        let context = create_test_context().await;
        let config_path = context._temp_dir.path().join("client-manual-clear.json");
        tokio::fs::write(&config_path, "{}").await.expect("seed config file");

        let Json(initial_response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "custom.runtime.clear".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: None,
                display_name: Some("Clear Runtime".to_string()),
                connection_mode: Some("local_config_detected".to_string()),
                config_path: Some(config_path.to_string_lossy().to_string()),
                supported_transports: Some(vec!["streamable_http".to_string()]),
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("initial update succeeds");
        assert!(initial_response.success);

        let Json(clear_response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "custom.runtime.clear".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: None,
                display_name: Some("Clear Runtime".to_string()),
                connection_mode: Some("manual".to_string()),
                config_path: None,
                supported_transports: Some(vec!["streamable_http".to_string()]),
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("manual update succeeds");
        assert!(clear_response.success);

        let state = context
            .client_service
            .fetch_state("custom.runtime.clear")
            .await
            .expect("fetch state")
            .expect("state exists");
        assert_eq!(state.connection_mode().as_str(), "manual");
        assert_eq!(state.config_path(), None);
    }

    #[tokio::test]
    async fn update_settings_infers_manual_when_config_path_is_explicitly_cleared() {
        let context = create_test_context().await;
        let config_path = context._temp_dir.path().join("client-manual-empty-string.json");
        tokio::fs::write(&config_path, "{}")
            .await
            .expect("seed config file");

        let Json(initial_response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "custom.runtime.empty-clear".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: None,
                display_name: Some("Empty Clear Runtime".to_string()),
                connection_mode: Some("local_config_detected".to_string()),
                config_path: Some(config_path.to_string_lossy().to_string()),
                supported_transports: Some(vec!["streamable_http".to_string()]),
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("initial update succeeds");
        assert!(initial_response.success);

        let Json(clear_response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "custom.runtime.empty-clear".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: None,
                display_name: Some("Empty Clear Runtime".to_string()),
                connection_mode: None,
                config_path: Some("   ".to_string()),
                supported_transports: Some(vec!["streamable_http".to_string()]),
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("explicit empty config path update succeeds");
        assert!(clear_response.success);

        let state = context
            .client_service
            .fetch_state("custom.runtime.empty-clear")
            .await
            .expect("fetch state")
            .expect("state exists");
        assert_eq!(state.connection_mode().as_str(), "manual");
        assert_eq!(state.config_path(), None);
    }

    #[tokio::test]
    async fn config_details_and_apply_reject_stale_missing_config_target() {
        let context = create_test_context().await;

        context
            .client_service
            .set_client_settings(
                "client-a",
                Some("hosted".to_string()),
                Some("streamable_http".to_string()),
                None,
            )
            .await
            .expect("seed client settings");

        sqlx::query(
            "UPDATE client SET connection_mode = 'local_config_detected', config_path = ?, managed = 1, approval_status = 'approved' WHERE identifier = ?",
        )
        .bind(context._temp_dir.path().join("stale-missing.json").to_string_lossy().to_string())
        .bind("client-a")
        .execute(&context.db_pool)
        .await
        .expect("seed stale config path");

        let Json(details_response) = config_details(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "client-a".to_string(),
            }),
        )
        .await
        .expect("config details response");

        assert!(details_response.success);
        let details = details_response.data.expect("details data");
        assert!(!details.writable_config);

        let apply_result = config_apply(
            State(context.app_state.clone()),
            Json(ClientConfigUpdateReq {
                identifier: "client-a".to_string(),
                mode: ClientConfigMode::Hosted,
                preview: false,
                selected_config: ClientConfigSelected::Default,
            }),
        )
        .await;

        assert!(apply_result.is_err());
        assert_eq!(apply_result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn update_settings_accepts_directory_target_for_kv_client() {
        let context = create_test_context().await;
        let kv_dir = context._temp_dir.path().join("cherry-kv");
        tokio::fs::create_dir_all(&kv_dir).await.expect("create kv directory");

        let Json(response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "cherry_studio".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: None,
                display_name: Some("Cherry Studio".to_string()),
                connection_mode: Some("local_config_detected".to_string()),
                config_path: Some(kv_dir.to_string_lossy().to_string()),
                supported_transports: Some(vec!["streamable_http".to_string()]),
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("kv directory target should be accepted");

        assert!(response.success);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn update_settings_rejects_non_writable_directory_target() {
        use std::os::unix::fs::PermissionsExt;

        let context = create_test_context().await;
        let kv_dir = context._temp_dir.path().join("read-only-kv");
        tokio::fs::create_dir_all(&kv_dir).await.expect("create kv directory");

        let original_permissions = std::fs::metadata(&kv_dir)
            .expect("directory metadata")
            .permissions();
        let mut read_only_permissions = original_permissions.clone();
        read_only_permissions.set_mode(0o555);
        std::fs::set_permissions(&kv_dir, read_only_permissions).expect("set read-only permissions");

        let result = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "cherry_studio".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: None,
                display_name: Some("Cherry Studio".to_string()),
                connection_mode: Some("local_config_detected".to_string()),
                config_path: Some(kv_dir.to_string_lossy().to_string()),
                supported_transports: Some(vec!["streamable_http".to_string()]),
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await;

        std::fs::set_permissions(&kv_dir, original_permissions).expect("restore permissions");

        let (status, Json(response)) = result.expect_err("read-only directory target should fail");
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let error = response.error.expect("error payload");
        assert!(error.message.contains("not writable"));
    }

    #[tokio::test]
    async fn update_settings_preserves_suspended_governance_state() {
        let context = create_test_context().await;
        let config_path = context._temp_dir.path().join("suspended-client.json");
        tokio::fs::write(&config_path, "{}")
            .await
            .expect("seed suspended client config file");

        context
            .client_service
            .set_active_client_settings(
                "client-a",
                ActiveClientSettingsUpdate {
                    display_name: Some("Suspended Client".to_string()),
                    config_mode: Some("hosted".to_string()),
                    transport: Some("streamable_http".to_string()),
                    connection_mode: Some("local_config_detected".to_string()),
                    config_path: Some(config_path.to_string_lossy().to_string()),
                    supported_transports: Some(vec!["streamable_http".to_string()]),
                    ..ActiveClientSettingsUpdate::default()
                },
            )
            .await
            .expect("seed active client state");

        context
            .client_service
            .suspend_client("client-a")
            .await
            .expect("suspend client");

        let Json(response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "client-a".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: Some("2.0.0".to_string()),
                display_name: Some("Suspended Client".to_string()),
                connection_mode: Some("local_config_detected".to_string()),
                config_path: Some(config_path.to_string_lossy().to_string()),
                supported_transports: Some(vec!["streamable_http".to_string()]),
                description: Some("Still editable while denied".to_string()),
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("update denied client settings succeeds");

        assert!(response.success);
        let state = context
            .client_service
            .fetch_state("client-a")
            .await
            .expect("fetch state")
            .expect("state exists");
        assert_eq!(state.approval_status(), "suspended");
        assert!(!state.managed());
        assert_eq!(state.display_name(), "Suspended Client");
    }

    #[tokio::test]
    async fn config_apply_uses_persisted_runtime_template_for_runtime_only_client() {
        let context = create_test_context().await;
        let config_path = context._temp_dir.path().join("runtime-client.json");
        tokio::fs::write(&config_path, "{}")
            .await
            .expect("seed runtime apply config file");

        let Json(update_response) = update_settings(
            State(context.app_state.clone()),
            Json(crate::api::models::client::ClientSettingsUpdateReq {
                identifier: "custom.runtime.apply".to_string(),
                config_mode: Some("hosted".to_string()),
                transport: Some("streamable_http".to_string()),
                client_version: Some("9.9.9".to_string()),
                display_name: Some("Runtime Apply Client".to_string()),
                connection_mode: Some("local_config_detected".to_string()),
                config_path: Some(config_path.to_string_lossy().to_string()),
                supported_transports: Some(vec!["streamable_http".to_string(), "stdio".to_string()]),
                description: Some("Runtime apply test client".to_string()),
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
            }),
        )
        .await
        .expect("update runtime-only client");

        assert!(update_response.success);

        let Json(apply_response) = config_apply(
            State(context.app_state.clone()),
            Json(ClientConfigUpdateReq {
                identifier: "custom.runtime.apply".to_string(),
                mode: ClientConfigMode::Hosted,
                preview: false,
                selected_config: ClientConfigSelected::Default,
            }),
        )
        .await
        .expect("apply runtime-only client config");

        assert!(apply_response.success);
        let data = apply_response.data.expect("response data");
        assert!(data.applied);
        assert_eq!(data.preview["mcpServers"]["MCPMate"]["type"], "streamable_http");

        let written = tokio::fs::read_to_string(&config_path)
            .await
            .expect("read written config");
        let parsed: Value = serde_json::from_str(&written).expect("parse written config");
        assert_eq!(parsed["mcpServers"]["MCPMate"]["type"], "streamable_http");
        assert_eq!(
            parsed["mcpServers"]["MCPMate"]["headers"]["x-mcpmate-client-id"],
            "custom.runtime.apply"
        );

        let Json(details_response) = config_details(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "custom.runtime.apply".to_string(),
            }),
        )
        .await
        .expect("details for applied runtime-only client");

        assert!(details_response.success);
        let details = details_response.data.expect("details data");
        assert!(details.config_exists);
        assert_eq!(
            details.template.managed_source.as_deref(),
            Some("runtime_active_client")
        );
        assert_eq!(details.content["mcpServers"]["MCPMate"]["type"], "streamable_http");
    }

    #[tokio::test]
    async fn config_apply_rejects_pending_unknown_client() {
        use crate::clients::models::OnboardingPolicy;
        use crate::common::constants::database::tables;

        let context = create_test_context().await;

        sqlx::query(&format!(
            "UPDATE {} SET value = ? WHERE key = 'onboarding_policy'",
            tables::SYSTEM_SETTINGS
        ))
        .bind(OnboardingPolicy::RequireApproval.as_str())
        .execute(&context.db_pool)
        .await
        .expect("set onboarding policy");

        sqlx::query(
            r#"
            INSERT INTO client (
                id, name, identifier, managed, approval_status, record_kind, connection_mode, template_identifier
            )
            VALUES ('clnt001', 'Pending Client', 'pending.client', 0, 'pending', 'observed_unknown', 'manual', NULL)
            "#,
        )
        .execute(&context.db_pool)
        .await
        .expect("create pending unknown client");

        let result = config_apply(
            State(context.app_state.clone()),
            Json(ClientConfigUpdateReq {
                identifier: "pending.client".to_string(),
                mode: ClientConfigMode::Hosted,
                preview: false,
                selected_config: ClientConfigSelected::Profile {
                    profile_id: "PROF001".to_string(),
                },
            }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn config_restore_rejects_pending_unknown_client() {
        use crate::clients::models::OnboardingPolicy;
        use crate::common::constants::database::tables;

        let context = create_test_context().await;

        sqlx::query(&format!(
            "UPDATE {} SET value = ? WHERE key = 'onboarding_policy'",
            tables::SYSTEM_SETTINGS
        ))
        .bind(OnboardingPolicy::RequireApproval.as_str())
        .execute(&context.db_pool)
        .await
        .expect("set onboarding policy");

        sqlx::query(
            r#"
            INSERT INTO client (
                id, name, identifier, managed, approval_status, record_kind, connection_mode, template_identifier
            )
            VALUES ('clnt002', 'Pending Client', 'pending.restore', 0, 'pending', 'observed_unknown', 'manual', NULL)
            "#,
        )
        .execute(&context.db_pool)
        .await
        .expect("create pending unknown client");

        let result = config_restore(
            State(context.app_state.clone()),
            Json(ClientConfigRestoreReq {
                identifier: "pending.restore".to_string(),
                backup: "backup-001".to_string(),
            }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }
}
