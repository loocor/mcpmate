use super::ClientConfigService;
use crate::clients::analyzer::{ConfigInspectionReport, inspect_config_content, inspect_config_value};
use crate::clients::document::{infer_format_from_path, parse_config_to_json_value};
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{ClientConfigFileParse, ContainerType, TemplateFormat};
use crate::clients::service::core::ClientStateRow;
use crate::clients::utils::get_nested_value;
use crate::system::paths::get_path_service;
use serde_json::{Map, Value};

const MAX_PREVIEW_ENTRIES: usize = 4;
const MAX_CREATE_INSPECT_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRuleValidation {
    pub matches: bool,
    pub format_matches: bool,
    pub container_found: bool,
    pub server_count: u32,
}

#[derive(Debug, Clone)]
pub struct ConfigRuleInspection {
    pub normalized_path: String,
    pub detected_format: Option<TemplateFormat>,
    pub inferred_parse: Option<ClientConfigFileParse>,
    pub validation: Option<ConfigRuleValidation>,
    pub preview: Value,
}

#[derive(Debug, Clone)]
struct ParsedConfigDocument {
    format: TemplateFormat,
    value: Value,
}

#[derive(Debug, Clone)]
struct CandidateRule {
    path: String,
    container_type: ContainerType,
    depth: usize,
}

impl ClientConfigService {
    pub(crate) async fn inspect_config_path_for_import(
        &self,
        state: &ClientStateRow,
        raw_path: &str,
        draft: Option<&ClientConfigFileParse>,
    ) -> ConfigResult<ConfigInspectionReport> {
        let normalized_path = get_path_service()
            .resolve_user_path(raw_path)
            .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;
        let raw = tokio::fs::read_to_string(&normalized_path)
            .await
            .map_err(ConfigError::IoError)?;
        self.inspect_config_content_for_import(state, &raw, draft)
    }

    pub(crate) async fn inspect_current_config_for_import(
        &self,
        identifier: &str,
    ) -> ConfigResult<ConfigInspectionReport> {
        let state = self
            .fetch_state(identifier)
            .await?
            .ok_or_else(|| ConfigError::ClientNotFound {
                identifier: identifier.to_string(),
            })?;
        let raw = self
            .read_current_config(identifier)
            .await?
            .ok_or_else(|| ConfigError::DataAccessError(format!("Client {} has no readable config", identifier)))?;
        self.inspect_config_content_for_import(&state, &raw, None)
    }

    fn inspect_config_content_for_import(
        &self,
        state: &ClientStateRow,
        raw: &str,
        draft: Option<&ClientConfigFileParse>,
    ) -> ConfigResult<ConfigInspectionReport> {
        let parse_rule = state.effective_config_file_parse_with(draft, false)?.ok_or_else(|| {
            ConfigError::DataAccessError(
                "Client is missing an effective config_file_parse; cannot scan existing MCP servers".to_string(),
            )
        })?;
        let transports = state.parsed_transports()?;
        let inspected = inspect_config_content(raw, &parse_rule, Some(&transports));
        if !inspected.inspection.matched_container {
            return Err(ConfigError::DataAccessError(
                "Configured parse rule did not match any config container".to_string(),
            ));
        }

        Ok(inspected)
    }

    pub async fn inspect_config_file_parse(
        &self,
        raw_path: &str,
        draft: Option<&ClientConfigFileParse>,
    ) -> ConfigResult<ConfigRuleInspection> {
        let (normalized_path, parsed) = load_document_from_raw_path(raw_path, ConfigPathPolicy::CreateInspect).await?;
        Ok(build_rule_inspection(&normalized_path, &parsed, draft))
    }

    pub async fn inspect_existing_client_config_file_parse(
        &self,
        identifier: &str,
        draft: Option<&ClientConfigFileParse>,
    ) -> ConfigResult<ConfigRuleInspection> {
        let state = self
            .fetch_state(identifier)
            .await?
            .ok_or_else(|| ConfigError::ClientNotFound {
                identifier: identifier.to_string(),
            })?;
        let normalized_path = Self::resolved_config_path_from_state(&state)?
            .ok_or_else(|| ConfigError::PathResolutionError(format!("No config_path for client {}", identifier)))?;
        let draft = state.effective_config_file_parse_with(draft, false)?;
        let parsed = load_document_from_resolved_path(&normalized_path).await?;
        Ok(build_rule_inspection(&normalized_path, &parsed, draft.as_ref()))
    }

    pub async fn validate_config_file_parse_rule(
        &self,
        raw_path: &str,
        rule: &ClientConfigFileParse,
    ) -> ConfigResult<ConfigRuleValidation> {
        let (_, parsed) = load_document_from_raw_path(raw_path, ConfigPathPolicy::General).await?;
        let validation = validate_rule_against_document(&parsed.value, parsed.format, rule);
        ensure_format_matches(rule, parsed.format, validation)
    }

    pub async fn infer_config_file_parse_rule(
        &self,
        raw_path: &str,
    ) -> ConfigResult<Option<ClientConfigFileParse>> {
        let (_, parsed) = load_document_from_raw_path(raw_path, ConfigPathPolicy::General).await?;
        Ok(infer_rule_from_document(&parsed.value, parsed.format))
    }
}

#[derive(Debug, Clone, Copy)]
enum ConfigPathPolicy {
    CreateInspect,
    General,
}

async fn resolve_config_path(
    raw_path: &str,
    policy: ConfigPathPolicy,
) -> ConfigResult<String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::DataAccessError(
            "A configuration file path is required for parse rule inspection.".to_string(),
        ));
    }

    let resolved = get_path_service()
        .resolve_user_path(trimmed)
        .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;

    if matches!(policy, ConfigPathPolicy::General) {
        return Ok(resolved.to_string_lossy().to_string());
    }

    infer_format_from_path(resolved.to_str()).ok_or_else(|| {
        ConfigError::DataAccessError(
            "Only json, json5, toml, yaml, or yml files can be inspected before creating a client record.".to_string(),
        )
    })?;
    let metadata = tokio::fs::metadata(&resolved).await.map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ConfigError::DataAccessError(format!("Configured MCP file does not exist: {}", trimmed))
        } else {
            ConfigError::FileOperationError(format!("Failed to inspect configured MCP file {}: {}", trimmed, err))
        }
    })?;

    if !metadata.is_file() {
        return Err(ConfigError::DataAccessError(format!(
            "Parse rule inspection requires a regular file, but got: {}",
            trimmed
        )));
    }

    if metadata.len() > MAX_CREATE_INSPECT_BYTES {
        return Err(ConfigError::DataAccessError(format!(
            "Selected config file is too large to inspect safely ({} bytes, max {}).",
            metadata.len(),
            MAX_CREATE_INSPECT_BYTES
        )));
    }
    Ok(resolved.to_string_lossy().to_string())
}

async fn load_document_from_raw_path(
    raw_path: &str,
    policy: ConfigPathPolicy,
) -> ConfigResult<(String, ParsedConfigDocument)> {
    let normalized_path = resolve_config_path(raw_path, policy).await?;
    let parsed = load_document_from_resolved_path(&normalized_path).await?;
    Ok((normalized_path, parsed))
}

async fn load_document_from_resolved_path(normalized_path: &str) -> ConfigResult<ParsedConfigDocument> {
    let content = tokio::fs::read_to_string(normalized_path)
        .await
        .map_err(ConfigError::IoError)?;
    parse_document(&content, Some(normalized_path))
}

fn build_rule_inspection(
    normalized_path: &str,
    parsed: &ParsedConfigDocument,
    draft: Option<&ClientConfigFileParse>,
) -> ConfigRuleInspection {
    let inferred_parse = infer_rule_from_document(&parsed.value, parsed.format);
    let validation = draft.map(|rule| validate_rule_against_document(&parsed.value, parsed.format, rule));
    let preview = build_preview_value(&parsed.value, draft.or(inferred_parse.as_ref()));

    ConfigRuleInspection {
        normalized_path: normalized_path.to_string(),
        detected_format: Some(parsed.format),
        inferred_parse,
        validation,
        preview,
    }
}

fn build_preview_value(
    document: &Value,
    preview_rule: Option<&ClientConfigFileParse>,
) -> Value {
    preview_rule
        .and_then(|rule| build_preview_for_rule(document, rule))
        .unwrap_or_else(|| limit_preview_value(document))
}

fn ensure_format_matches(
    rule: &ClientConfigFileParse,
    detected_format: TemplateFormat,
    validation: ConfigRuleValidation,
) -> ConfigResult<ConfigRuleValidation> {
    if !validation.format_matches {
        let reason = format!(
            "Configured parse rule format '{}' does not match file format '{}'.",
            rule.format.as_str(),
            detected_format.as_str()
        );
        return Err(ConfigError::DataAccessError(reason));
    }

    Ok(validation)
}

fn parse_document(
    content: &str,
    path_hint: Option<&str>,
) -> ConfigResult<ParsedConfigDocument> {
    let hinted = infer_format_from_path(path_hint);
    let try_order = build_parse_order(hinted);

    for format in try_order {
        if let Some(value) = parse_config_to_json_value(content, Some(format.as_str())) {
            return Ok(ParsedConfigDocument { format, value });
        }
    }

    Err(ConfigError::DataAccessError(
        "Unable to parse the selected configuration file as json, json5, toml, or yaml.".to_string(),
    ))
}

fn build_parse_order(hinted: Option<TemplateFormat>) -> Vec<TemplateFormat> {
    let mut order = Vec::new();
    if let Some(format) = hinted {
        order.push(format);
    }

    for format in [
        TemplateFormat::Json,
        TemplateFormat::Json5,
        TemplateFormat::Toml,
        TemplateFormat::Yaml,
    ] {
        if !order.contains(&format) {
            order.push(format);
        }
    }

    order
}

fn validate_rule_against_document(
    document: &Value,
    detected_format: TemplateFormat,
    rule: &ClientConfigFileParse,
) -> ConfigRuleValidation {
    let format_matches = detected_format == rule.format;
    let inspection = inspect_config_value(document, rule, None);
    let server_count = inspection.server_count();

    ConfigRuleValidation {
        matches: format_matches && inspection.matched_container && server_count > 0,
        format_matches,
        container_found: inspection.matched_container,
        server_count,
    }
}

fn infer_rule_from_document(
    document: &Value,
    format: TemplateFormat,
) -> Option<ClientConfigFileParse> {
    let mut candidates = Vec::new();
    collect_candidates(document, "", &mut candidates);
    let best = candidates
        .into_iter()
        .filter_map(|candidate| {
            let rule = ClientConfigFileParse {
                format,
                container_type: candidate.container_type,
                container_keys: vec![candidate.path.clone()],
            };
            let inspection = inspect_config_value(document, &rule, None);
            let server_count = inspection.server_count();

            if !inspection.matched_container || server_count == 0 {
                return None;
            }

            Some((candidate, server_count))
        })
        .max_by(|(left, left_count), (right, right_count)| {
            score_candidate(left, *left_count).cmp(&score_candidate(right, *right_count))
        })?
        .0;

    Some(ClientConfigFileParse {
        format,
        container_type: best.container_type,
        container_keys: vec![best.path],
    })
}

fn collect_candidates(
    value: &Value,
    path: &str,
    out: &mut Vec<CandidateRule>,
) {
    match value {
        Value::Object(map) => {
            push_candidate(out, path, ContainerType::ObjectMap);

            for (key, child) in map {
                let next_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                collect_candidates(child, &next_path, out);
            }
        }
        Value::Array(items) => {
            push_candidate(out, path, ContainerType::Array);

            for child in items {
                if child.is_object() {
                    collect_candidates(child, path, out);
                }
            }
        }
        _ => {}
    }
}

fn push_candidate(
    out: &mut Vec<CandidateRule>,
    path: &str,
    container_type: ContainerType,
) {
    if path.is_empty() {
        return;
    }

    out.push(CandidateRule {
        path: path.to_string(),
        container_type,
        depth: path.matches('.').count(),
    });
}

fn score_candidate(
    candidate: &CandidateRule,
    server_count: u32,
) -> (u32, i32, i32) {
    (
        server_count,
        if candidate.path == "mcpServers" { 3 } else { 0 },
        -(candidate.depth as i32),
    )
}

fn build_preview_for_rule(
    document: &Value,
    rule: &ClientConfigFileParse,
) -> Option<Value> {
    for key in &rule.container_keys {
        if let Some(container) = get_nested_value(document, key) {
            return Some(limit_preview_value(container));
        }
    }

    None
}

fn limit_preview_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let limited = map
                .iter()
                .take(MAX_PREVIEW_ENTRIES)
                .map(|(key, value)| (key.clone(), limit_preview_value(value)))
                .collect::<Map<String, Value>>();
            Value::Object(limited)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .take(MAX_PREVIEW_ENTRIES)
                .map(limit_preview_value)
                .collect(),
        ),
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn infer_rule_prefers_mcpservers_object_map() {
        let document = json!({
            "mcpServers": {
                "alpha": { "command": "node" }
            },
            "other": {
                "servers": {
                    "beta": { "command": "bun" }
                }
            }
        });

        let inferred = infer_rule_from_document(&document, TemplateFormat::Json).expect("inferred rule");
        assert_eq!(inferred.container_keys, vec!["mcpServers"]);
        assert_eq!(inferred.container_type, ContainerType::ObjectMap);
    }

    #[test]
    fn validate_rule_checks_container_and_format() {
        let document = json!({
            "mcpServers": {
                "alpha": { "command": "node" }
            }
        });
        let rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["mcpServers".to_string()],
        };

        let validation = validate_rule_against_document(&document, TemplateFormat::Json, &rule);
        assert!(validation.matches);
        assert!(validation.container_found);
        assert_eq!(validation.server_count, 1);
    }

    #[test]
    fn validate_rule_ignores_entries_without_transport_fields() {
        let document = json!({
            "mcpServers": {
                "alpha": { "label": "not-a-server" }
            }
        });
        let rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["mcpServers".to_string()],
        };

        let validation = validate_rule_against_document(&document, TemplateFormat::Json, &rule);
        assert!(!validation.matches);
        assert!(validation.container_found);
        assert_eq!(validation.server_count, 0);
    }
}
