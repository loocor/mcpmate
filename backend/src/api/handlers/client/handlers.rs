// HTTP handlers for client management API (template-driven)

use super::backups::parse_policy_payload;
use super::config::{analyze_config_content, get_config_last_modified};
use super::import::build_import_payload_from_value;
use crate::api::models::client::{
    ClientAttachData, ClientAttachReq, ClientAttachResp, ClientBackupActionData, ClientBackupActionResp,
    ClientCapabilityConfigData, ClientCapabilityConfigReq, ClientCapabilityConfigResp, ClientCheckData, ClientCheckReq,
    ClientCheckResp, ClientConfigData, ClientConfigFileParseData, ClientConfigFileParseInspectData,
    ClientConfigFileParseInspectExistingReq, ClientConfigFileParseInspectExistingResp, ClientConfigFileParseInspectReq,
    ClientConfigFileParseInspectResp, ClientConfigFileParseValidationData, ClientConfigImportData,
    ClientConfigImportReq, ClientConfigImportResp, ClientConfigMode, ClientConfigReq, ClientConfigResp,
    ClientConfigRestoreReq, ClientConfigSelected, ClientConfigUpdateData, ClientConfigUpdateReq,
    ClientConfigUpdateResp, ClientDetachData, ClientDetachReq, ClientDetachResp, ClientFormatRuleData,
    ClientImportSummary, ClientImportedServer, ClientInfo, ClientTemplateMetadata, ClientTemplateStorageMetadata,
    ClientUnifyDirectExposureData,
};
use crate::api::routes::AppState;
use crate::audit::{AuditAction, AuditEvent, AuditStatus};
use crate::clients::models::{
    AttachmentState, CapabilitySource, ClientCapabilityConfigState, ClientConfigFileParse, ContainerType,
    TemplateFormat, UnifyDirectExposureConfig,
};
use crate::clients::service::core::{ClientStateRow, RuntimeClientMetadata};
use crate::clients::service::settings::ActiveClientSettingsUpdate;
use crate::clients::{
    ClientConfigService, ClientDescriptor, ClientRenderOptions, ConfigError, ConfigMode, TemplateExecutionResult,
};
use crate::common::ClientCategory;
use crate::core::proxy::server::ProxyServer;
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use json5;
use serde_json::{Value, json};
use serde_yaml;
use std::collections::HashMap;
use std::sync::Arc;
use toml;

type ClientSettingsErrorResponse = (StatusCode, Json<crate::api::models::client::ClientSettingsUpdateResp>);

fn convert_parse_container_type(container_type: ContainerType) -> crate::api::models::client::ClientConfigType {
    match container_type {
        ContainerType::Array => crate::api::models::client::ClientConfigType::Array,
        ContainerType::ObjectMap => crate::api::models::client::ClientConfigType::Standard,
    }
}

fn parse_data_from_rule(rule: &ClientConfigFileParse) -> ClientConfigFileParseData {
    ClientConfigFileParseData {
        format: rule.format.as_str().to_string(),
        container_type: convert_parse_container_type(rule.container_type),
        container_keys: rule.container_keys.clone(),
    }
}

fn resolve_effective_config_file_parse(state: Option<&ClientStateRow>) -> Option<ClientConfigFileParse> {
    if let Some(state) = state {
        if let Ok(Some(override_parse)) = state.config_file_parse_override() {
            return Some(override_parse);
        }
        return state.legacy_config_file_parse().ok().flatten();
    }

    None
}

fn build_parse_metadata(
    state: Option<&ClientStateRow>
) -> (
    Option<ClientConfigFileParseData>,
    Option<ClientConfigFileParseData>,
    bool,
) {
    let effective = resolve_effective_config_file_parse(state)
        .as_ref()
        .map(parse_data_from_rule);
    let override_parse = state
        .and_then(|s| s.config_file_parse_override().ok().flatten())
        .as_ref()
        .map(parse_data_from_rule);
    let uses_default = override_parse.is_none();
    (effective, override_parse, uses_default)
}

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

fn build_client_capability_config_data(
    identifier: String,
    state: ClientCapabilityConfigState,
) -> ClientCapabilityConfigData {
    ClientCapabilityConfigData {
        identifier,
        capability_source: state.capability_config.capability_source,
        selected_profile_ids: state.capability_config.selected_profile_ids,
        custom_profile_id: state.capability_config.custom_profile_id,
        custom_profile_missing: state.custom_profile_missing,
        unify_direct_exposure: ClientUnifyDirectExposureData {
            intent: state.unify_direct_exposure_intent,
            diagnostics: state.unify_direct_exposure_diagnostics,
            resolved_capabilities: state.unify_direct_exposure,
        },
    }
}

fn parse_rule_from_api_data(parse: &ClientConfigFileParseData) -> ClientConfigFileParse {
    ClientConfigFileParse {
        format: match parse.format.as_str() {
            "json5" => TemplateFormat::Json5,
            "toml" => TemplateFormat::Toml,
            "yaml" => TemplateFormat::Yaml,
            _ => TemplateFormat::Json,
        },
        container_type: match parse.container_type {
            crate::api::models::client::ClientConfigType::Array => ContainerType::Array,
            _ => ContainerType::ObjectMap,
        },
        container_keys: parse.container_keys.clone(),
    }
}

fn format_rule_data_from_rule(rule: &crate::clients::models::FormatRule) -> ClientFormatRuleData {
    let rule = rule.normalized();
    let extra_fields = if rule.extra_fields.is_empty() {
        None
    } else {
        Some(
            rule.extra_fields
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
    };

    ClientFormatRuleData {
        command_field: rule.command_field.clone(),
        args_field: rule.args_field.clone(),
        env_field: rule.env_field.clone(),
        include_type: rule.include_type,
        type_value: rule.type_value.clone(),
        url_field: rule.url_field.clone(),
        headers_field: rule.headers_field.clone(),
        extra_fields,
        selected: rule.selected,
    }
}

fn format_rule_from_api_data(data: &ClientFormatRuleData) -> crate::clients::models::FormatRule {
    let extra_fields = data
        .extra_fields
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect::<serde_json::Map<String, serde_json::Value>>();

    let mut rule = crate::clients::models::FormatRule {
        template: serde_json::Value::Object(serde_json::Map::new()),
        command_field: data.command_field.clone(),
        args_field: data.args_field.clone(),
        env_field: data.env_field.clone(),
        include_type: data.include_type,
        type_value: data.type_value.clone(),
        url_field: data.url_field.clone(),
        headers_field: data.headers_field.clone(),
        extra_fields,
        selected: data.selected,
    };
    rule.template = rule.to_template();
    rule
}

fn parse_api_transports(
    rules: &HashMap<String, ClientFormatRuleData>
) -> Result<HashMap<String, crate::clients::models::FormatRule>, String> {
    rules
        .iter()
        .map(|(transport, rule)| {
            let parsed_rule = format_rule_from_api_data(rule);
            parsed_rule
                .validate_for_transport(transport)
                .map_err(|err| format!("Invalid format rule for transport '{transport}': {err}"))?;

            Ok((transport.clone(), parsed_rule))
        })
        .collect()
}

fn transports_data_from_state(state: Option<&ClientStateRow>) -> Option<HashMap<String, ClientFormatRuleData>> {
    let state = state?;
    let rules = state.parsed_transports().ok()?;
    if rules.is_empty() {
        return None;
    }

    Some(
        rules
            .into_iter()
            .map(|(transport, rule)| (transport, format_rule_data_from_rule(&rule)))
            .collect(),
    )
}

fn build_config_file_parse_inspect_data(
    inspection: crate::clients::service::config_rules::ConfigRuleInspection,
    include_preview: bool,
) -> ClientConfigFileParseInspectData {
    ClientConfigFileParseInspectData {
        normalized_path: inspection.normalized_path,
        detected_format: inspection.detected_format.map(|format| format.as_str().to_string()),
        inferred_parse: inspection.inferred_parse.as_ref().map(parse_data_from_rule),
        validation: inspection
            .validation
            .map(|validation| ClientConfigFileParseValidationData {
                matches: validation.matches,
                format_matches: validation.format_matches,
                container_found: validation.container_found,
                server_count: validation.server_count,
            }),
        preview: if include_preview {
            inspection.preview
        } else {
            Value::Null
        },
        preview_text: None,
        warnings: inspection.warnings,
    }
}

pub async fn config_file_parse_inspect(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientConfigFileParseInspectReq>,
) -> Result<Json<ClientConfigFileParseInspectResp>, (StatusCode, Json<ClientConfigFileParseInspectResp>)> {
    let service = get_client_service(&app_state).map_err(|status| {
        (
            status,
            Json(ClientConfigFileParseInspectResp::error_simple(
                "client_config_parse_unavailable",
                "Client service unavailable",
            )),
        )
    })?;

    let draft = request.config_file_parse.as_ref().map(parse_rule_from_api_data);
    let inspection = service
        .inspect_config_file_parse(&request.config_path, draft.as_ref())
        .await
        .map_err(|err| {
            let status = map_config_error_status(&err);
            (
                status,
                Json(ClientConfigFileParseInspectResp::error_simple(
                    "client_config_parse_invalid",
                    &err.to_string(),
                )),
            )
        })?;

    Ok(Json(ClientConfigFileParseInspectResp::success(
        build_config_file_parse_inspect_data(inspection, false),
    )))
}

pub async fn config_file_parse_inspect_existing(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientConfigFileParseInspectExistingReq>,
) -> Result<Json<ClientConfigFileParseInspectExistingResp>, (StatusCode, Json<ClientConfigFileParseInspectExistingResp>)>
{
    let service = get_client_service(&app_state).map_err(|status| {
        (
            status,
            Json(ClientConfigFileParseInspectExistingResp::error_simple(
                "client_config_parse_unavailable",
                "Client service unavailable",
            )),
        )
    })?;

    if service
        .fetch_state(&request.identifier)
        .await
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ClientConfigFileParseInspectExistingResp::error_simple(
                    "client_config_parse_invalid",
                    &err.to_string(),
                )),
            )
        })?
        .is_none()
    {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ClientConfigFileParseInspectExistingResp::error_simple(
                "client_config_parse_missing_client",
                "Client record not found.",
            )),
        ));
    }

    let draft = request.config_file_parse.as_ref().map(parse_rule_from_api_data);
    let inspection = service
        .inspect_existing_client_config_file_parse(&request.identifier, draft.as_ref())
        .await
        .map_err(|err| {
            let status = map_config_error_status(&err);
            (
                status,
                Json(ClientConfigFileParseInspectExistingResp::error_simple(
                    "client_config_parse_invalid",
                    &err.to_string(),
                )),
            )
        })?;

    Ok(Json(ClientConfigFileParseInspectExistingResp::success(
        build_config_file_parse_inspect_data(inspection, true),
    )))
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
        match descriptor_to_client_info(service.as_ref(), app_state.as_ref(), descriptor).await {
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
    if state.is_none() {
        tracing::error!("Failed to resolve client config details for {}", request.identifier);
        return Err(StatusCode::NOT_FOUND);
    }

    let config_path = service.config_path(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to resolve config path for {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut warnings: Vec<String> = Vec::new();
    let mut degraded_reasons: Vec<String> = Vec::new();
    let has_local_config_target = state
        .as_ref()
        .map(ClientStateRow::has_local_config_target)
        .unwrap_or(false);
    let use_runtime_config_read = !has_local_config_target;
    let content = if use_runtime_config_read {
        read_runtime_config(service.as_ref(), &request.identifier)
            .await
            .map_err(|err| {
                tracing::error!(client = %request.identifier, error = %err, "Failed to read runtime config");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        match service.read_current_config(&request.identifier).await {
            Ok(content) => content,
            Err(err) => {
                let message = format!("Unable to read current configuration: {}", err);
                tracing::warn!(
                    client = %request.identifier,
                    error = %err,
                    "Gracefully degrading after configuration read failure"
                );
                warnings.push(message);
                degraded_reasons.push("config_read_failed_fallback_none".to_string());
                None
            }
        }
    };

    let config_exists = content.is_some();
    let parsed_content = match (content.as_deref(), state.as_ref()) {
        (Some(raw), Some(state)) => match state.config_format() {
            Some(_) => parse_config_value(raw, state.config_format()),
            None => parse_runtime_config_value(raw, config_path.as_deref()),
        },
        (Some(raw), None) => parse_runtime_config_value(raw, config_path.as_deref()),
        (None, _) => Value::Null,
    };

    let (has_mcp_config, mcp_servers_count) = match (content.as_deref(), state.as_ref()) {
        (Some(raw), Some(state)) => {
            let effective_parse = resolve_effective_config_file_parse(Some(state));
            let container_keys = effective_parse
                .as_ref()
                .map(|parse| parse.container_keys.clone())
                .unwrap_or_else(|| state.container_keys().unwrap_or_default());
            let is_array_container = effective_parse
                .as_ref()
                .map(|parse| parse.container_type == ContainerType::Array)
                .unwrap_or_else(|| state.container_type() == Some("array"));
            let format = effective_parse
                .as_ref()
                .map(|parse| parse.format.as_str())
                .or_else(|| state.config_format());
            analyze_config_content(raw, &container_keys, is_array_container, format)
        }
        _ => (false, 0),
    };

    let last_modified = config_path.as_deref().and_then(get_config_last_modified);

    let config_type = state
        .as_ref()
        .and_then(|row| match row.container_type() {
            Some("array") => Some(crate::api::models::client::ClientConfigType::Array),
            Some("object") => Some(crate::api::models::client::ClientConfigType::Standard),
            _ => None,
        })
        .or_else(|| infer_config_type_from_path(config_path.as_deref()));

    let (imported_servers, import_summary) = (None, None);
    let runtime_metadata = state
        .as_ref()
        .map(ClientStateRow::runtime_client_metadata)
        .unwrap_or_default();
    // Meta information comes only from runtime_metadata (approval_metadata.runtime_client)
    let description = runtime_metadata.description.clone();
    let homepage_url = runtime_metadata.homepage_url.clone();
    let docs_url = runtime_metadata.docs_url.clone();
    let support_url = runtime_metadata.support_url.clone();
    let logo_url = runtime_metadata.logo_url.clone();

    let capability_config_state = service
        .get_capability_config_state(&request.identifier)
        .await
        .map_err(|err| {
            tracing::error!(client = %request.identifier, error = %err, "Failed to load capability config state");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let capability_config = capability_config_state
        .as_ref()
        .map(|state| state.capability_config.clone())
        .unwrap_or_default();
    let custom_profile_missing = capability_config_state
        .as_ref()
        .map(|state| state.custom_profile_missing)
        .unwrap_or(false);
    let governance_kind = state
        .as_ref()
        .map(|row| row.governance_kind().as_str().to_string())
        .or_else(|| Some("passive".to_string()));
    let connection_mode = state
        .as_ref()
        .map(|row| row.connection_mode().as_str().to_string())
        .or_else(|| Some(default_connection_mode_from_path(config_path.as_deref())));
    let governed_by_default_policy = state
        .as_ref()
        .map(|row| row.governed_by_default_policy())
        .unwrap_or(true);
    let approval_status = state.as_ref().map(|row| row.approval_status().to_string());
    let attachment_state = state
        .as_ref()
        .map(|row| row.attachment_state().as_str().to_string())
        .unwrap_or_else(|| "not_applicable".to_string());
    let writable_config = service
        .has_verified_local_config_target(&request.identifier)
        .await
        .unwrap_or_else(|err| {
            tracing::warn!(client = %request.identifier, error = %err, "Failed to verify local config target");
            degraded_reasons.push("writable_target_verification_failed_default_false".to_string());
            false
        });
    let config_file_parse_effective = state
        .as_ref()
        .and_then(|row| resolve_effective_config_file_parse(Some(row)))
        .as_ref()
        .map(parse_data_from_rule);
    let config_file_parse_override = state
        .as_ref()
        .and_then(|row| row.config_file_parse_override().ok().flatten())
        .as_ref()
        .map(parse_data_from_rule);
    let uses_template_parse_default = config_file_parse_override.is_none();
    let transports = transports_data_from_state(state.as_ref());

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
        template: build_client_template_metadata(state.as_ref(), &runtime_metadata),
        transports,
        description,
        homepage_url,
        docs_url,
        support_url,
        logo_url,
        capability_source: capability_config.capability_source,
        selected_profile_ids: capability_config.selected_profile_ids,
        custom_profile_id: capability_config.custom_profile_id,
        custom_profile_missing,
        approval_status,
        attachment_state: Some(attachment_state),
        governance_kind,
        connection_mode,
        governed_by_default_policy,
        writable_config,
        config_file_parse_effective,
        config_file_parse_override,
        uses_template_parse_default,
        warnings,
        degraded_reasons,
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
    let mark_attached_on_apply = !matches!(request.mode, ClientConfigMode::Transparent);
    let applied_config =
        apply_client_config_request(&service, &request, "config_apply", mark_attached_on_apply).await?;
    let data = applied_config.data;

    tracing::info!(
        client = %request.identifier,
        applied = data.applied,
        preview = %request.preview,
        scheduled = data.scheduled.unwrap_or(false),
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
            "applied": data.applied,
            "scheduled": data.scheduled.unwrap_or(false),
            "selected_config": request.selected_config,
        })),
        None,
    )
    .await;

    Ok(Json(ClientConfigUpdateResp::success(data)))
}

struct AppliedClientConfig {
    data: ClientConfigUpdateData,
}

async fn apply_client_config_request(
    service: &ClientConfigService,
    request: &ClientConfigUpdateReq,
    operation: &'static str,
    mark_attached_on_apply: bool,
) -> Result<AppliedClientConfig, StatusCode> {
    let requested_backup_policy = if request.preview {
        None
    } else {
        request.backup_policy.as_ref().map(parse_policy_payload).transpose()?
    };

    let existing_state = service.fetch_state(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, operation, error = %err, "Failed to load client state before apply");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(state) = existing_state.as_ref() {
        if state.approval_status() != "approved" {
            tracing::warn!(
                client = %request.identifier,
                operation,
                approval_status = %state.approval_status(),
                "Rejected config apply for non-approved client"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    match service.has_verified_local_config_target(&request.identifier).await {
        Ok(true) => {}
        Ok(false) => {
            if !existing_state
                .as_ref()
                .map(ClientStateRow::has_local_config_target)
                .unwrap_or(false)
            {
                tracing::warn!(
                    client = %request.identifier,
                    operation,
                    "Rejected config apply without runtime local config target"
                );
                return Err(StatusCode::FORBIDDEN);
            }
        }
        Err(err) => {
            tracing::warn!(
                client = %request.identifier,
                operation,
                error = %err,
                "Rejected config apply without a verified local config target"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let client_name = service.resolve_client_name(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, operation, error = %err, "Failed to resolve client name");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match existing_state.as_ref() {
        Some(state) => {
            service
                .ensure_active_state_row_with_name(&request.identifier, &client_name, Some(state.approval_status()))
                .await
                .map_err(|err| {
                    tracing::error!(client = %request.identifier, operation, error = %err, "Failed to refresh active client state before apply");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
        None => {
            service
                .ensure_active_state_row_with_name(&request.identifier, &client_name, Some("approved"))
                .await
                .map_err(|err| {
                    tracing::error!(client = %request.identifier, operation, error = %err, "Failed to create active client state before apply");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    if let Some(policy) = requested_backup_policy {
        service
            .set_backup_policy(&request.identifier, policy)
            .await
            .map_err(|err| {
                tracing::error!(
                    client = %request.identifier,
                    operation,
                    error = %err,
                    "Failed to persist backup policy before apply"
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    let options = build_render_options(request);
    let outcome = service.apply_with_deferred(options).await.map_err(|err| {
        let status = map_config_error_status(&err);
        tracing::error!(
            client = %request.identifier,
            operation,
            mode = ?request.mode,
            preview = %request.preview,
            selected = ?request.selected_config,
            status = %status.as_u16(),
            error = %err,
            "config apply failed"
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

    let state = service.fetch_state(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, operation, error = %err, "Failed to load client state for preview");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let config_format = state.as_ref().and_then(|state| state.config_format());
    let config_path = state.as_ref().and_then(|state| state.config_path());
    let preview = build_update_preview(config_format, config_path, &synthetic);
    let mut warnings = outcome.warnings.clone();
    warnings.extend(outcome.preview.summary.clone());
    let applied = outcome.applied && !request.preview;

    if applied && mark_attached_on_apply {
        service.mark_client_attached(&request.identifier).await.map_err(|err| {
            tracing::error!(client = %request.identifier, operation, error = %err, "Failed to persist attached state after config apply");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    Ok(AppliedClientConfig {
        data: ClientConfigUpdateData {
            success: true,
            preview,
            applied,
            backup_path: outcome.backup_path.clone(),
            warnings,
            diff_format: Some(outcome.preview.format.as_str().to_string()),
            diff_before: outcome.preview.before.clone(),
            diff_after: outcome.preview.after.clone(),
            scheduled: Some(outcome.scheduled),
            scheduled_reason: outcome.scheduled_reason,
        },
    })
}

fn map_config_error_status(err: &ConfigError) -> StatusCode {
    match err {
        ConfigError::ClientDisabled { .. } => StatusCode::FORBIDDEN,
        ConfigError::DataAccessError(_)
        | ConfigError::PathResolutionError(_)
        | ConfigError::PathNotWritable { .. }
        | ConfigError::FileOperationError(_)
        | ConfigError::IoError(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Handler for POST /api/client/config/restore
/// Restores configuration from a named backup snapshot
pub async fn config_restore(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientConfigRestoreReq>,
) -> Result<Json<ClientBackupActionResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    if let Ok(Some(state)) = service.fetch_state(&request.identifier).await {
        if state.is_pending_approval() {
            tracing::warn!(
                client = %request.identifier,
                approval_status = %state.approval_status(),
                "Rejected config restore for pending client"
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let client_name = service.resolve_client_name(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, error = %err, "Failed to resolve client name");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    service
        .ensure_active_state_row_with_name(&request.identifier, &client_name, Some("approved"))
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
    let state = service
        .fetch_state(&request.identifier)
        .await
        .map_err(|err| {
            tracing::error!("Failed to load client state {}: {}", request.identifier, err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::error!("Client state not found: {}", request.identifier);
            StatusCode::NOT_FOUND
        })?;

    let raw = service.read_current_config(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to read config for {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let config_format_str = state.config_format().filter(|s| !s.trim().is_empty()).ok_or_else(|| {
        tracing::error!(
            client = %request.identifier,
            status = 422u16,
            "config_format is missing; cannot parse configuration"
        );
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    let json_value = raw
        .as_deref()
        .map(|raw| parse_config_value(raw, Some(config_format_str)))
        .unwrap_or(serde_json::Value::Null);

    let db = app_state.database.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Build standard import payload from parsed config
    let items =
        build_import_payload_from_value(&json_value, resolve_effective_config_file_parse(Some(&state)).as_ref());
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
    fn should_emit_managed_visibility_change(
        &self,
        requested: bool,
    ) -> bool {
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

    if let Some(proxy_server) = ProxyServer::global().and_then(|proxy| proxy.try_lock().ok().map(|guard| guard.clone()))
    {
        if let Err(err) = proxy_server
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

    let parsed_transports = request
        .transports
        .as_ref()
        .map(parse_api_transports)
        .transpose()
        .map_err(|err| client_settings_error(StatusCode::UNPROCESSABLE_ENTITY, err))?;

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
                config_file_parse: request.config_file_parse.as_ref().map(parse_rule_from_api_data),
                clear_config_file_parse: request.clear_config_file_parse,
                transports: parsed_transports,
                clear_transports: request.clear_transports,
            },
        )
        .await
        .map_err(|err| {
            let status = map_config_error_status(&err);
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
    let (config_file_parse_effective, config_file_parse_override, uses_template_parse_default) =
        build_parse_metadata(Some(&state));
    let transports = transports_data_from_state(Some(&state));

    let data = crate::api::models::client::ClientSettingsUpdateData {
        identifier: request.identifier,
        display_name: state.display_name().to_string(),
        config_mode: mode,
        transport,
        client_version: version,
        connection_mode: Some(state.connection_mode().as_str().to_string()),
        config_path: state.config_path().map(str::to_string),
        transports,
        description: runtime_metadata.description.clone(),
        homepage_url: runtime_metadata.homepage_url.clone(),
        docs_url: runtime_metadata.docs_url.clone(),
        support_url: runtime_metadata.support_url.clone(),
        logo_url: runtime_metadata.logo_url.clone(),
        config_file_parse_effective,
        config_file_parse_override,
        uses_template_parse_default,
        setting_sources: crate::api::models::client::ClientSettingsSourceData {
            display_name: settings_result.display_name_source.to_string(),
            approval_status: settings_result.approval_status_source.to_string(),
            connection_mode: settings_result.connection_mode_source.to_string(),
        },
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
            "transports": data.transports,
            "config_file_parse_effective": data.config_file_parse_effective,
            "config_file_parse_override": data.config_file_parse_override,
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

    let data = build_client_capability_config_data(request.identifier, state);

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

    Ok(Json(ClientCapabilityConfigResp::success(
        build_client_capability_config_data(request.identifier, config),
    )))
}

/// Handler for POST /api/client/detach
pub async fn client_detach(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientDetachReq>,
) -> Result<Json<ClientDetachResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let changed = service.detach_client(&request.identifier).await.map_err(|err| {
        let status = match err {
            ConfigError::DataAccessError(_) | ConfigError::PathResolutionError(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        tracing::error!(client = %request.identifier, status = %status.as_u16(), error = %err, "Failed to detach client");
        status
    })?;
    super::manage::invalidate_client_runtime_visibility(&request.identifier).await;

    tracing::info!(client = %request.identifier, changed, "Client detached from MCPMate config");

    emit_client_audit_event(
        &app_state,
        AuditAction::ClientConfigDetach,
        AuditStatus::Success,
        "/api/client/detach",
        &request.identifier,
        Some(request.identifier.clone()),
        Some(json!({
            "detach": true,
            "changed": changed,
            "attachment_state": "detached",
        })),
        None,
    )
    .await;

    Ok(Json(ClientDetachResp::success(ClientDetachData {
        identifier: request.identifier,
        attachment_state: "detached".to_string(),
        changed,
    })))
}

fn resolve_persisted_apply_selection(
    mode: ClientConfigMode,
    cap: &ClientCapabilityConfigState,
) -> Result<ClientConfigSelected, ConfigError> {
    match mode {
        ClientConfigMode::Unify => Ok(ClientConfigSelected::Default),
        ClientConfigMode::Hosted | ClientConfigMode::Transparent => {
            if cap.capability_config.capability_source == CapabilitySource::Custom {
                if cap.custom_profile_missing || cap.capability_config.custom_profile_id.is_none() {
                    return Err(ConfigError::DataAccessError(
                        "Custom workspace profile is missing; fix capability configuration before attaching MCPMate."
                            .to_string(),
                    ));
                }
                Ok(ClientConfigSelected::Profile {
                    profile_id: cap.capability_config.custom_profile_id.clone().unwrap(),
                })
            } else {
                Ok(ClientConfigSelected::Default)
            }
        }
    }
}

fn client_config_mode_from_effective(mode: &str) -> Result<ClientConfigMode, StatusCode> {
    match mode.to_ascii_lowercase().as_str() {
        "unify" => Ok(ClientConfigMode::Unify),
        "hosted" => Ok(ClientConfigMode::Hosted),
        "transparent" => Ok(ClientConfigMode::Transparent),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Handler for POST /api/client/attach
pub async fn client_attach(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientAttachReq>,
) -> Result<Json<ClientAttachResp>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let existing_state = service.fetch_state(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, error = %err, "Failed to load client state before attach");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(state) = existing_state.as_ref() else {
        return Err(StatusCode::NOT_FOUND);
    };

    if state.attachment_state() != AttachmentState::Detached {
        tracing::warn!(
            client = %request.identifier,
            attachment = ?state.attachment_state(),
            "Rejected client attach while not detached"
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    let effective_mode = service.get_effective_config_mode(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, error = %err, "Failed to resolve effective config mode for attach");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let api_mode = client_config_mode_from_effective(&effective_mode)?;

    let cap_state = service
        .get_capability_config_state(&request.identifier)
        .await
        .map_err(|err| {
            tracing::error!(client = %request.identifier, error = %err, "Failed to load capability config for attach");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let selected_config = resolve_persisted_apply_selection(api_mode.clone(), &cap_state).map_err(|err| {
        tracing::warn!(client = %request.identifier, error = %err, "Invalid capability selection for attach");
        StatusCode::BAD_REQUEST
    })?;

    let apply_request = ClientConfigUpdateReq {
        identifier: request.identifier.clone(),
        mode: api_mode,
        preview: false,
        selected_config,
        backup_policy: None,
    };

    let applied_config = apply_client_config_request(&service, &apply_request, "client_attach", true).await?;
    if !applied_config.data.applied {
        tracing::error!(client = %request.identifier, "client_attach produced no applied write");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    tracing::info!(client = %request.identifier, "Client MCPMate entry re-attached to external config");

    emit_client_audit_event(
        &app_state,
        AuditAction::ClientConfigAttach,
        AuditStatus::Success,
        "/api/client/attach",
        &request.identifier,
        Some(request.identifier.clone()),
        Some(json!({
            "attach": true,
            "applied": applied_config.data.applied,
            "mode": apply_request.mode,
            "selected_config": apply_request.selected_config,
            "attachment_state": "attached",
        })),
        None,
    )
    .await;

    Ok(Json(ClientAttachResp::success(ClientAttachData {
        identifier: request.identifier,
        attachment_state: "attached".to_string(),
        changed: applied_config.data.applied,
    })))
}

async fn descriptor_to_client_info(
    service: &ClientConfigService,
    _app_state: &AppState,
    descriptor: ClientDescriptor,
) -> Result<ClientInfo, StatusCode> {
    let state = descriptor.state.clone();
    let runtime_metadata = state.runtime_client_metadata();
    let (config_file_parse_effective, config_file_parse_override, uses_template_parse_default) =
        build_parse_metadata(Some(&state));
    let transports = transports_data_from_state(Some(&state));
    let identifier = state.identifier().to_string();
    let display_name = state.display_name().to_string();
    let logo_url = runtime_metadata.logo_url.clone();
    let category = runtime_metadata
        .category
        .as_deref()
        .and_then(ClientCategory::parse)
        .unwrap_or_default();
    let description = runtime_metadata.description.clone();
    let homepage_url = runtime_metadata.homepage_url.clone();
    let docs_url = runtime_metadata.docs_url.clone();
    let support_url = runtime_metadata.support_url.clone();
    let config_type = match state.container_type() {
        Some("array") => Some(crate::api::models::client::ClientConfigType::Array),
        Some("object") => Some(crate::api::models::client::ClientConfigType::Standard),
        _ => infer_config_type_from_path(descriptor.config_path.as_deref()),
    };
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
    let custom_profile_missing = service
        .get_capability_config_state(&identifier)
        .await
        .map_err(|err| {
            tracing::error!(client = %identifier, error = %err, "Failed to load client capability config state");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(|state| state.custom_profile_missing)
        .unwrap_or(false);

    let content = if descriptor.config_exists {
        match service.read_current_config(&identifier).await {
            Ok(content) => content,
            Err(err) => {
                tracing::warn!(
                    client = %identifier,
                    error = %err,
                    "Continuing list operation despite configuration read failure"
                );
                None
            }
        }
    } else {
        None
    };

    let (has_mcp_config, mcp_servers_count) = match content.as_deref() {
        Some(raw) => {
            let container_keys = config_file_parse_effective
                .as_ref()
                .map(|parse| parse.container_keys.clone())
                .unwrap_or_else(|| state.container_keys().unwrap_or_default());
            let is_array_container = config_file_parse_effective
                .as_ref()
                .map(|parse| parse.container_type == crate::api::models::client::ClientConfigType::Array)
                .unwrap_or_else(|| state.container_type() == Some("array"));
            let format = config_file_parse_effective
                .as_ref()
                .map(|parse| parse.format.as_str())
                .or_else(|| state.config_format());
            analyze_config_content(raw, &container_keys, is_array_container, format)
        }
        _ => (false, 0),
    };

    let last_modified = descriptor.config_path.as_deref().and_then(get_config_last_modified);

    let approval_status = Some(state.approval_status().to_string());
    let governance_kind = Some(state.governance_kind().as_str().to_string());
    let connection_mode = Some(state.connection_mode().as_str().to_string());
    let governed_by_default_policy = state.governed_by_default_policy();

    let (config_mode, transport, client_version) = service
        .get_client_settings(state.identifier())
        .await
        .ok()
        .flatten()
        .map(|(mode, transport, version)| (mode, Some(transport), version))
        .unwrap_or((None, Some("auto".to_string()), None));

    let writable_config = service
        .has_verified_local_config_target(state.identifier())
        .await
        .unwrap_or(false);
    let pending_approval = approval_status.as_deref() == Some("pending");

    Ok(ClientInfo {
        identifier,
        display_name,
        logo_url,
        category,
        enabled: state.is_approved(),
        detected: descriptor.detection.is_some(),
        install_path: None,
        config_path: descriptor.config_path.unwrap_or_default(),
        config_exists: descriptor.config_exists,
        has_mcp_config,
        transports,
        description,
        homepage_url,
        docs_url,
        support_url,
        config_mode,
        transport,
        client_version,
        capability_source: capability_config.capability_source,
        selected_profile_ids: capability_config.selected_profile_ids,
        custom_profile_id: capability_config.custom_profile_id,
        custom_profile_missing,
        config_type,
        last_detected: descriptor.detected_at.map(|dt| dt.to_rfc3339()),
        last_modified,
        mcp_servers_count: Some(mcp_servers_count),
        template: build_client_template_metadata(Some(&state), &runtime_metadata),
        approval_status,
        attachment_state: Some(state.attachment_state().as_str().to_string()),
        governance_kind,
        connection_mode,
        governed_by_default_policy,
        writable_config,
        pending_approval,
        config_file_parse_effective,
        config_file_parse_override,
        uses_template_parse_default,
    })
}

// moved to POST /api/client/config/import

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

fn build_template_metadata_from_state(
    state: &ClientStateRow,
    runtime_metadata: &RuntimeClientMetadata,
) -> ClientTemplateMetadata {
    let container_type = match state.container_type() {
        Some("array") => crate::api::models::client::ClientConfigType::Array,
        _ => crate::api::models::client::ClientConfigType::Standard,
    };

    ClientTemplateMetadata {
        format: state.config_format().unwrap_or("json").to_string(),
        protocol_revision: state.protocol_revision().map(|s| s.to_string()),
        storage: ClientTemplateStorageMetadata {
            kind: state.storage_kind().unwrap_or("file").to_string(),
            path_strategy: state.storage_path_strategy().map(|s| s.to_string()),
        },
        container_type,
        merge_strategy: state.merge_strategy().unwrap_or("replace").to_string(),
        keep_original_config: state.keep_original_config(),
        managed_source: state.managed_source().map(|s| s.to_string()),
        description: runtime_metadata.description.clone(),
        homepage_url: runtime_metadata.homepage_url.clone(),
        docs_url: runtime_metadata.docs_url.clone(),
        support_url: runtime_metadata.support_url.clone(),
    }
}

fn build_client_template_metadata(
    state: Option<&ClientStateRow>,
    runtime_metadata: &RuntimeClientMetadata,
) -> ClientTemplateMetadata {
    match state {
        Some(state) => build_template_metadata_from_state(state, runtime_metadata),
        None => build_runtime_template_metadata(None, runtime_metadata),
    }
}

fn default_connection_mode_from_path(config_path: Option<&str>) -> String {
    if config_path.unwrap_or_default().is_empty() {
        "manual".to_string()
    } else {
        "local_config_detected".to_string()
    }
}

fn parse_config_value(
    content: &str,
    config_format: Option<&str>,
) -> Value {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Value::Null;
    }

    let format = match config_format {
        Some("json") => TemplateFormat::Json,
        Some("json5") => TemplateFormat::Json5,
        Some("toml") => TemplateFormat::Toml,
        Some("yaml") => TemplateFormat::Yaml,
        _ => return Value::Null,
    };

    match format {
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
    config_format: Option<&str>,
    config_path: Option<&str>,
    execution: &TemplateExecutionResult,
) -> Value {
    let content = match execution {
        TemplateExecutionResult::Applied { content, .. } => content,
        TemplateExecutionResult::DryRun { content, .. } => content,
    };

    if config_format.is_some() {
        return parse_config_value(content, config_format);
    }

    parse_runtime_config_value(content, config_path)
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

        assert!(
            result.is_err(),
            "unexpected client visible change event arrived in timeout window"
        );
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
        crate::core::capability::naming::initialize(pool.clone());
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
            sqlx::query(
                "INSERT INTO server_tools (id, server_id, server_name, tool_name, unique_name, description) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(format!("{id}-tool-{tool_name}"))
            .bind(id)
            .bind(name)
            .bind(tool_name)
            .bind(format!("{}_{}", name.to_lowercase().replace(' ', "_"), tool_name))
            .bind(Some("tool"))
            .execute(pool)
            .await
            .expect("insert server tool");
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
        crate::core::capability::naming::initialize(pool.clone());
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
            data.unify_direct_exposure.intent.route_mode,
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
            data.unify_direct_exposure.intent.route_mode,
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
                    server_ids: Vec::new(),
                    capability_ids: Default::default(),
                }),
            }),
        )
        .await
        .expect("update route-only capability config");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.intent.route_mode,
            crate::clients::models::UnifyRouteMode::BrokerOnly
        );
        assert!(data.unify_direct_exposure.intent.server_ids.is_empty());
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_tool_surfaces
                .is_empty()
        );
        assert!(data.unify_direct_exposure.diagnostics.invalid_server_ids.is_empty());
        assert!(data.unify_direct_exposure.diagnostics.invalid_tool_surfaces.is_empty());
    }

    #[tokio::test]
    async fn capability_config_roundtrips_unify_server_level_selection() {
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
                identifier: "client-server-level".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::ServerLevel,
                    server_ids: vec!["server-eligible".to_string()],
                    capability_ids: Default::default(),
                }),
            }),
        )
        .await
        .expect("update server-level capability config");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.intent.route_mode,
            crate::clients::models::UnifyRouteMode::ServerLevel
        );
        assert_eq!(
            data.unify_direct_exposure.intent.server_ids,
            vec!["server-eligible".to_string()]
        );
        assert_eq!(
            data.unify_direct_exposure.resolved_capabilities.selected_tool_surfaces,
            vec![
                crate::clients::models::UnifyDirectToolSurface {
                    server_id: "server-eligible".to_string(),
                    tool_name: "tool-a".to_string(),
                },
                crate::clients::models::UnifyDirectToolSurface {
                    server_id: "server-eligible".to_string(),
                    tool_name: "tool-b".to_string(),
                },
            ]
        );
        assert!(data.unify_direct_exposure.diagnostics.invalid_server_ids.is_empty());

        let stored_intent: Option<String> =
            sqlx::query_scalar("SELECT unify_direct_exposure_intent FROM client WHERE identifier = ?")
                .bind("client-server-level")
                .fetch_one(&context.db_pool)
                .await
                .expect("load stored unify intent");
        let stored_intent: serde_json::Value =
            serde_json::from_str(stored_intent.as_deref().expect("stored intent payload"))
                .expect("parse stored intent payload");
        assert_eq!(stored_intent["route_mode"], "server_level");
        assert_eq!(stored_intent["server_ids"], serde_json::json!(["server-eligible"]));
        assert!(stored_intent.get("selected_tool_surfaces").is_none());

        sqlx::query(
            "INSERT INTO server_tools (id, server_id, server_name, tool_name, unique_name, description) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("server-eligible-tool-tool-c")
        .bind("server-eligible")
        .bind("Eligible Server")
        .bind("tool-c")
        .bind("eligible_server_tool-c")
        .bind(Some("tool"))
        .execute(&context.db_pool)
        .await
        .expect("insert refreshed server tool");

        let Json(response) = get_capability_config(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "client-server-level".to_string(),
            }),
        )
        .await
        .expect("reload server-level capability config after inventory refresh");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.intent.server_ids,
            vec!["server-eligible".to_string()]
        );
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_tool_surfaces
                .iter()
                .any(|surface| surface.server_id == "server-eligible" && surface.tool_name == "tool-c")
        );
    }

    #[tokio::test]
    async fn initial_unify_server_level_selection_survives_late_server_enable() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-late-enable",
            "Late Enable Server",
            true,
            &["tool-a"],
        )
        .await;
        sqlx::query("UPDATE server_config SET enabled = 0 WHERE id = ?")
            .bind("server-late-enable")
            .execute(&context.db_pool)
            .await
            .expect("disable server before initial selection");

        context
            .client_service
            .set_active_client_settings(
                "client-late-server-level",
                ActiveClientSettingsUpdate {
                    config_mode: Some("unify".to_string()),
                    ..ActiveClientSettingsUpdate::default()
                },
            )
            .await
            .expect("seed unify client mode");

        let Json(response) = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-late-server-level".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::ServerLevel,
                    server_ids: vec!["server-late-enable".to_string()],
                    capability_ids: Default::default(),
                }),
            }),
        )
        .await
        .expect("store server-level intent before server is enabled");

        let data = response.data.expect("response data");
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_tool_surfaces
                .is_empty()
        );

        let stored_intent: Option<String> =
            sqlx::query_scalar("SELECT unify_direct_exposure_intent FROM client WHERE identifier = ?")
                .bind("client-late-server-level")
                .fetch_one(&context.db_pool)
                .await
                .expect("load stored unify intent");
        let stored_intent: serde_json::Value =
            serde_json::from_str(stored_intent.as_deref().expect("stored intent payload"))
                .expect("parse stored intent payload");
        assert_eq!(stored_intent["route_mode"], "server_level");
        assert_eq!(stored_intent["server_ids"], serde_json::json!(["server-late-enable"]));

        sqlx::query("UPDATE server_config SET enabled = 1 WHERE id = ?")
            .bind("server-late-enable")
            .execute(&context.db_pool)
            .await
            .expect("enable server after initial selection");

        let reconciled = context
            .client_service
            .reconcile_unify_direct_exposure_for_server("server-late-enable")
            .await
            .expect("reconcile direct exposure after enable");
        assert_eq!(reconciled.len(), 1);
        assert_eq!(reconciled[0].identifier, "client-late-server-level");
        assert_eq!(
            reconciled[0].unify_direct_exposure.selected_tool_surfaces,
            vec![crate::clients::models::UnifyDirectToolSurface {
                server_id: "server-late-enable".to_string(),
                tool_name: "tool-a".to_string(),
            }]
        );
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
                    server_ids: Vec::new(),
                    capability_ids: crate::clients::models::UnifyDirectCapabilityIds {
                        tool_ids: vec!["tool_server_tool-b".to_string()],
                        ..Default::default()
                    },
                }),
            }),
        )
        .await
        .expect("update capability-level config");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.resolved_capabilities.selected_tool_surfaces,
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
        insert_unify_server(&context.db_pool, "server-mixed", "Mixed Server", true, &["tool-a"]).await;
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
                    server_ids: Vec::new(),
                    capability_ids: crate::clients::models::UnifyDirectCapabilityIds {
                        prompt_ids: vec!["mixed_server_prompt-a".to_string()],
                        resource_ids: vec!["mixed_server:resource-a".to_string()],
                        template_ids: vec!["mixed_server_template-a".to_string()],
                        ..Default::default()
                    },
                }),
            }),
        )
        .await
        .expect("update mixed capability-level config");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_prompt_surfaces,
            vec![crate::clients::models::UnifyDirectPromptSurface {
                server_id: "server-mixed".to_string(),
                prompt_name: "prompt-a".to_string(),
            }]
        );
        assert_eq!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_resource_surfaces,
            vec![crate::clients::models::UnifyDirectResourceSurface {
                server_id: "server-mixed".to_string(),
                resource_uri: "resource-a".to_string(),
            }]
        );
        assert_eq!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_template_surfaces,
            vec![crate::clients::models::UnifyDirectTemplateSurface {
                server_id: "server-mixed".to_string(),
                uri_template: "template-a".to_string(),
            }]
        );
        assert!(
            data.unify_direct_exposure
                .diagnostics
                .invalid_prompt_surfaces
                .is_empty()
        );
        assert!(
            data.unify_direct_exposure
                .diagnostics
                .invalid_resource_surfaces
                .is_empty()
        );
        assert!(
            data.unify_direct_exposure
                .diagnostics
                .invalid_template_surfaces
                .is_empty()
        );
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
                    server_ids: Vec::new(),
                    capability_ids: crate::clients::models::UnifyDirectCapabilityIds {
                        tool_ids: vec!["global_only_server_tool-b".to_string()],
                        ..Default::default()
                    },
                }),
            }),
        )
        .await
        .expect("update capability-level config against globally enabled server");

        let data = response.data.expect("response data");
        assert_eq!(
            data.unify_direct_exposure.resolved_capabilities.selected_tool_surfaces,
            vec![crate::clients::models::UnifyDirectToolSurface {
                server_id: "server-global-only".to_string(),
                tool_name: "tool-b".to_string(),
            }]
        );
        assert!(data.unify_direct_exposure.diagnostics.invalid_server_ids.is_empty());
        assert!(data.unify_direct_exposure.diagnostics.invalid_tool_surfaces.is_empty());
    }

    #[tokio::test]
    #[serial_test::serial]
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
    #[serial_test::serial]
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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
            }),
        )
        .await
        .expect("switch mode to hosted");

        assert!(response.success);
        wait_for_client_visible_change_event(&mut rx, "client-a").await;
    }

    #[tokio::test]
    #[serial_test::serial]
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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
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
                    server_ids: Vec::new(),
                    capability_ids: crate::clients::models::UnifyDirectCapabilityIds {
                        tool_ids: vec![
                            "eligible_server_tool-missing".to_string(),
                            "ineligible_server_tool-x".to_string(),
                            "eligible_server_tool-a".to_string(),
                        ],
                        ..Default::default()
                    },
                }),
            }),
        )
        .await
        .expect("update invalid unify config");

        let data = response.data.expect("response data");
        assert!(data.unify_direct_exposure.intent.server_ids.is_empty());
        assert_eq!(
            data.unify_direct_exposure.resolved_capabilities.selected_tool_surfaces,
            vec![crate::clients::models::UnifyDirectToolSurface {
                server_id: "server-eligible".to_string(),
                tool_name: "tool-a".to_string(),
            }]
        );
        assert_eq!(
            data.unify_direct_exposure.intent.capability_ids.tool_ids,
            vec!["eligible_server_tool-a".to_string()]
        );
        assert_eq!(
            data.unify_direct_exposure.diagnostics.invalid_server_ids,
            Vec::<String>::new()
        );
        assert_eq!(data.unify_direct_exposure.diagnostics.invalid_capability_ids.len(), 2);
        assert!(
            data.unify_direct_exposure
                .diagnostics
                .invalid_capability_ids
                .iter()
                .any(|item| item == "eligible_server_tool-missing")
        );
        assert!(
            data.unify_direct_exposure
                .diagnostics
                .invalid_capability_ids
                .iter()
                .any(|item| item == "ineligible_server_tool-x")
        );

        let stored_intent: Option<String> =
            sqlx::query_scalar("SELECT unify_direct_exposure_intent FROM client WHERE identifier = ?")
                .bind("client-invalid")
                .fetch_one(&context.db_pool)
                .await
                .expect("load stored unify intent");
        let stored_intent: serde_json::Value =
            serde_json::from_str(stored_intent.as_deref().expect("stored intent payload"))
                .expect("parse stored intent payload");
        assert_eq!(stored_intent["route_mode"], "capability_level");
        assert_eq!(
            stored_intent["capability_ids"]["tool_ids"],
            serde_json::json!(["eligible_server_tool-a"])
        );
    }

    #[tokio::test]
    async fn server_eligibility_change_prunes_server_level_direct_surfaces() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-prune-level",
            "Prune Level Server",
            true,
            &["tool-a"],
        )
        .await;

        let _ = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-prune-level".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::ServerLevel,
                    server_ids: vec!["server-prune-level".to_string()],
                    capability_ids: Default::default(),
                }),
            }),
        )
        .await
        .expect("seed server-level capability config");

        let _ = crate::api::handlers::server::update_server(
            State(context.app_state.clone()),
            Json(crate::api::models::server::ServerUpdateReq {
                id: "server-prune-level".to_string(),
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
                identifier: "client-prune-level".to_string(),
            }),
        )
        .await
        .expect("load pruned capability config");

        let data = response.data.expect("response data");
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_tool_surfaces
                .is_empty()
        );
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
                    server_ids: Vec::new(),
                    capability_ids: crate::clients::models::UnifyDirectCapabilityIds {
                        tool_ids: vec!["prune_capability_server_tool-a".to_string()],
                        prompt_ids: vec!["prune_capability_server_prompt-a".to_string()],
                        resource_ids: vec!["prune_capability_server:resource-a".to_string()],
                        template_ids: vec!["prune_capability_server_template-a".to_string()],
                    },
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
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_tool_surfaces
                .is_empty()
        );
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_prompt_surfaces
                .is_empty()
        );
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_resource_surfaces
                .is_empty()
        );
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_template_surfaces
                .is_empty()
        );
        assert_eq!(
            data.unify_direct_exposure.intent.capability_ids,
            crate::clients::models::UnifyDirectCapabilityIds::default()
        );
    }

    #[tokio::test]
    async fn server_disable_prunes_server_level_direct_surfaces() {
        let context = create_test_context().await;
        insert_unify_server(
            &context.db_pool,
            "server-disable-level",
            "Disable Level Server",
            true,
            &["tool-a"],
        )
        .await;

        let _ = update_capability_config(
            State(context.app_state.clone()),
            Json(ClientCapabilityConfigReq {
                identifier: "client-disable-level".to_string(),
                capability_source: CapabilitySource::Activated,
                selected_profile_ids: Vec::new(),
                unify_direct_exposure: Some(crate::api::models::client::ClientUnifyDirectExposureReq {
                    route_mode: crate::clients::models::UnifyRouteMode::ServerLevel,
                    server_ids: vec!["server-disable-level".to_string()],
                    capability_ids: Default::default(),
                }),
            }),
        )
        .await
        .expect("seed server-level config before disable");

        let _ = crate::api::handlers::server::disable_server(
            State(context.app_state.clone()),
            Path("server-disable-level".to_string()),
            Query(std::collections::HashMap::new()),
        )
        .await
        .expect("disable server globally");

        let Json(response) = get_capability_config(
            State(context.app_state.clone()),
            Query(ClientConfigReq {
                identifier: "client-disable-level".to_string(),
            }),
        )
        .await
        .expect("load server-level config after disable");

        let data = response.data.expect("response data");
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_tool_surfaces
                .is_empty()
        );
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
                    server_ids: Vec::new(),
                    capability_ids: crate::clients::models::UnifyDirectCapabilityIds {
                        tool_ids: vec!["disable_capability_server_tool-a".to_string()],
                        prompt_ids: vec!["disable_capability_server_prompt-a".to_string()],
                        resource_ids: vec!["disable_capability_server:resource-a".to_string()],
                        template_ids: vec!["disable_capability_server_template-a".to_string()],
                    },
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
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_tool_surfaces
                .is_empty()
        );
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_prompt_surfaces
                .is_empty()
        );
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_resource_surfaces
                .is_empty()
        );
        assert!(
            data.unify_direct_exposure
                .resolved_capabilities
                .selected_template_surfaces
                .is_empty()
        );
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
        assert!(data.transports.is_none());
        assert_eq!(data.description.as_deref(), Some("Runtime-only client"));
        assert_eq!(data.template.format, "json");
        assert_eq!(data.template.storage.kind, "file");
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
                description: Some("Custom runtime client".to_string()),
                homepage_url: Some("https://example.com".to_string()),
                docs_url: Some("https://example.com/docs".to_string()),
                support_url: Some("https://example.com/support".to_string()),
                logo_url: Some("https://example.com/logo.png".to_string()),
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
            }),
        )
        .await
        .expect("update settings");

        assert!(response.success);
        let data = response.data.expect("response data");
        assert_eq!(data.display_name, "Custom Runtime");
        assert_eq!(data.connection_mode.as_deref(), Some("local_config_detected"));
        assert!(data.transports.is_none());
        assert_eq!(data.description.as_deref(), Some("Custom runtime client"));

        let state = context
            .client_service
            .fetch_state("custom.runtime.payload")
            .await
            .expect("fetch state")
            .expect("state exists");
        assert_eq!(state.governance_kind().as_str(), "active");
        assert_eq!(state.display_name(), "Custom Runtime");
        assert_eq!(state.connection_mode().as_str(), "local_config_detected");
        assert_eq!(state.config_path(), Some(config_path.to_string_lossy().as_ref()));
        assert_eq!(state.template_identifier(), None);
        assert_eq!(
            state.runtime_client_metadata().homepage_url.as_deref(),
            Some("https://example.com")
        );

        assert_eq!(state.config_format(), None);
        assert_eq!(state.container_keys().expect("container keys"), Vec::<String>::new());
        assert_eq!(state.managed_source(), None);
        let transports = state.parsed_transports().expect("transports");
        assert!(transports.is_empty());
    }

    #[tokio::test]
    async fn config_details_returns_transports_as_support_source() {
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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: Some(crate::api::models::client::ClientConfigFileParseData {
                    format: "json".to_string(),
                    container_type: crate::api::models::client::ClientConfigType::Standard,
                    container_keys: vec!["mcpServers".to_string()],
                }),
                clear_config_file_parse: false,
                transports: Some(HashMap::from([(
                    "streamable_http".to_string(),
                    crate::api::models::client::ClientFormatRuleData {
                        include_type: true,
                        type_value: Some("streamable_http".to_string()),
                        url_field: Some("url".to_string()),
                        headers_field: Some("headers".to_string()),
                        ..Default::default()
                    },
                )])),
                clear_transports: false,
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
        assert!(
            details
                .transports
                .as_ref()
                .is_some_and(|transports| transports.contains_key("streamable_http"))
        );

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
        assert!(
            listed_client
                .transports
                .as_ref()
                .is_some_and(|transports| transports.contains_key("streamable_http"))
        );
    }

    #[test]
    fn transport_support_derivation_does_not_infer_sse_from_http_keys() {
        let mut rules: HashMap<String, crate::clients::models::FormatRule> = HashMap::new();
        rules.insert(
            "http".to_string(),
            crate::clients::models::FormatRule {
                template: serde_json::json!({
                    "type": "streamable_http",
                    "url": "{{{url}}}",
                    "headers": "{{{json headers}}}"
                }),
                include_type: false,
                ..Default::default()
            },
        );

        let transports = crate::clients::service::core::supported_transports_from_transports(&rules);
        assert_eq!(transports, vec!["streamable_http".to_string()]);
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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
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
        tokio::fs::write(&config_path, "{}").await.expect("seed config file");

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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
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
            "UPDATE client SET connection_mode = 'local_config_detected', config_path = ?, approval_status = 'approved' WHERE identifier = ?",
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
                backup_policy: None,
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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
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

        let original_permissions = std::fs::metadata(&kv_dir).expect("directory metadata").permissions();
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
                description: None,
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
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
                description: Some("Still editable while denied".to_string()),
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: None,
                clear_config_file_parse: false,
                transports: None,
                clear_transports: false,
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
        assert_eq!(state.display_name(), "Suspended Client");
    }

    #[tokio::test]
    async fn config_apply_uses_db_runtime_definition_for_runtime_only_client() {
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
                description: Some("Runtime apply test client".to_string()),
                homepage_url: None,
                docs_url: None,
                support_url: None,
                logo_url: None,
                config_file_parse: Some(crate::api::models::client::ClientConfigFileParseData {
                    format: "json".to_string(),
                    container_type: crate::api::models::client::ClientConfigType::Standard,
                    container_keys: vec!["mcpServers".to_string()],
                }),
                clear_config_file_parse: false,
                transports: Some(HashMap::from([(
                    "streamable_http".to_string(),
                    crate::api::models::client::ClientFormatRuleData {
                        include_type: true,
                        type_value: Some("streamable_http".to_string()),
                        url_field: Some("url".to_string()),
                        headers_field: Some("headers".to_string()),
                        ..Default::default()
                    },
                )])),
                clear_transports: false,
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
                backup_policy: Some(crate::api::models::client::ClientBackupPolicyPayload {
                    policy: "off".to_string(),
                    limit: None,
                }),
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
            parsed["mcpServers"]["MCPMate"]["headers"][crate::common::constants::client_headers::MCPMATE_CLIENT_ID],
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
        assert_eq!(details.attachment_state.as_deref(), Some("attached"));
        assert_eq!(details.template.managed_source.as_deref(), None);
        assert_eq!(details.content["mcpServers"]["MCPMate"]["type"], "streamable_http");
    }

    #[tokio::test]
    async fn config_apply_rejects_pending_client() {
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
                id, name, identifier, approval_status, connection_mode, template_identifier
            )
            VALUES ('clnt001', 'Pending Client', 'pending.client', 'pending', 'manual', NULL)
            "#,
        )
        .execute(&context.db_pool)
        .await
        .expect("create pending client");

        let result = config_apply(
            State(context.app_state.clone()),
            Json(ClientConfigUpdateReq {
                identifier: "pending.client".to_string(),
                mode: ClientConfigMode::Hosted,
                preview: false,
                selected_config: ClientConfigSelected::Profile {
                    profile_id: "PROF001".to_string(),
                },
                backup_policy: None,
            }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn config_restore_rejects_pending_client() {
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
                id, name, identifier, approval_status, connection_mode, template_identifier
            )
            VALUES ('clnt002', 'Pending Client', 'pending.restore', 'pending', 'manual', NULL)
            "#,
        )
        .execute(&context.db_pool)
        .await
        .expect("create pending client");

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
