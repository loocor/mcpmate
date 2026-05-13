use crate::api::models::client::{
    ClientConfigFileParseData, ClientConfigFileParseInspectData, ClientConfigFileParseValidationData,
    ClientFormatRuleData, ServerEntryData,
};
use crate::clients::ClientConfigService;
use crate::clients::analyzer::{ConfigAnalysis, ConfigInspectionReport, inspect_config_content};
use crate::clients::models::{
    ClientConfigFileParse, ContainerType, FormatRule, TemplateFormat, canonical_config_transport_key,
};
use crate::clients::service::core::ClientStateRow;
use crate::clients::service::rules::ConfigRuleInspection;
use std::collections::HashMap;

fn parse_data_from_rule(rule: &ClientConfigFileParse) -> ClientConfigFileParseData {
    ClientConfigFileParseData {
        format: rule.format.as_str().to_string(),
        container_type: convert_parse_container_type(rule.container_type),
        container_keys: rule.container_keys.clone(),
    }
}

fn validation_data_from_rule(
    validation: crate::clients::service::rules::ConfigRuleValidation
) -> ClientConfigFileParseValidationData {
    ClientConfigFileParseValidationData {
        matches: validation.matches,
        format_matches: validation.format_matches,
        container_found: validation.container_found,
        server_count: validation.server_count,
    }
}

fn resolve_effective_config_file_parse(state: &ClientStateRow) -> Option<ClientConfigFileParse> {
    state.effective_config_file_parse().ok().flatten()
}

pub(super) fn build_parse_metadata(
    state: &ClientStateRow
) -> (
    Option<ClientConfigFileParseData>,
    Option<ClientConfigFileParseData>,
    bool,
) {
    let effective = resolve_effective_config_file_parse(state)
        .as_ref()
        .map(parse_data_from_rule);
    let override_parse = state
        .config_file_parse_override()
        .ok()
        .flatten()
        .as_ref()
        .map(parse_data_from_rule);
    let uses_default = override_parse.is_none();
    (effective, override_parse, uses_default)
}

pub(super) fn inspect_client_config_lenient(
    raw: Option<&str>,
    state: &ClientStateRow,
) -> Option<ConfigInspectionReport> {
    let parse_rule = resolve_effective_config_file_parse(state)?;
    let transports = state.parsed_transports().ok();
    Some(inspect_config_content(
        raw.unwrap_or_default(),
        &parse_rule,
        transports.as_ref(),
    ))
}

pub(super) async fn detect_mcpmate_in_client_config(
    service: &ClientConfigService,
    state: &ClientStateRow,
) -> Option<bool> {
    let raw = service.read_current_config(state.identifier()).await.ok()??;
    let inspected = inspect_client_config_lenient(Some(&raw), state)?;
    Some(inspected.analysis.mcpmate_present)
}

pub(super) fn derive_attachment_state(
    state: &ClientStateRow,
    config_analysis: &ConfigAnalysis,
) -> String {
    if config_analysis.mcpmate_present {
        return "attached".to_string();
    }

    if state.has_local_config_target() {
        return "detached".to_string();
    }

    state.attachment_state().as_str().to_string()
}

pub(super) fn configured_server_entries_data(
    report: &ConfigInspectionReport
) -> (ConfigAnalysis, Vec<ServerEntryData>) {
    let entries = report
        .inspection
        .entries
        .clone()
        .into_iter()
        .map(ServerEntryData::from)
        .collect::<Vec<_>>();
    (report.analysis.clone(), entries)
}

pub(crate) fn parse_rule_from_api_data(parse: &ClientConfigFileParseData) -> ClientConfigFileParse {
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

fn format_rule_data_from_rule(rule: &FormatRule) -> ClientFormatRuleData {
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

fn format_rule_from_api_data(data: &ClientFormatRuleData) -> FormatRule {
    let extra_fields = data
        .extra_fields
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect::<serde_json::Map<String, serde_json::Value>>();

    let mut rule = FormatRule {
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

pub(super) fn parse_api_transports(
    rules: &HashMap<String, ClientFormatRuleData>
) -> Result<HashMap<String, FormatRule>, String> {
    rules
        .iter()
        .map(|(transport, rule)| {
            if canonical_config_transport_key(transport).is_none() {
                return Err(format!(
                    "Invalid transport key '{transport}'; expected one of: streamable_http, sse, stdio"
                ));
            }

            let parsed_rule = format_rule_from_api_data(rule);
            parsed_rule
                .validate_for_transport(transport)
                .map_err(|err| format!("Invalid format rule for transport '{transport}': {err}"))?;

            Ok((transport.clone(), parsed_rule))
        })
        .collect()
}

pub(super) fn transports_data_from_state(state: &ClientStateRow) -> Option<HashMap<String, ClientFormatRuleData>> {
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

pub(super) fn build_config_file_parse_inspect_data(
    inspection: ConfigRuleInspection
) -> ClientConfigFileParseInspectData {
    ClientConfigFileParseInspectData {
        normalized_path: inspection.normalized_path,
        detected_format: inspection.detected_format.map(|format| format.as_str().to_string()),
        inferred_parse: inspection.inferred_parse.as_ref().map(parse_data_from_rule),
        validation: inspection.validation.map(validation_data_from_rule),
        preview: inspection.preview,
    }
}

fn convert_parse_container_type(container_type: ContainerType) -> crate::api::models::client::ClientConfigType {
    match container_type {
        ContainerType::Array => crate::api::models::client::ClientConfigType::Array,
        ContainerType::ObjectMap => crate::api::models::client::ClientConfigType::Standard,
    }
}
