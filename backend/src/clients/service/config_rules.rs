use super::ClientConfigService;
use crate::clients::analyzer::parse_config_to_json_value;
use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::{ClientConfigFileParse, ContainerType, TemplateFormat};
use crate::clients::utils::get_nested_value;
use crate::system::paths::get_path_service;
use serde_json::{Map, Value};

const MAX_PREVIEW_ENTRIES: usize = 4;

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
    pub preview_text: String,
    pub warnings: Vec<String>,
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
    server_count: u32,
    depth: usize,
}

impl ClientConfigService {
    pub async fn inspect_config_file_parse(
        &self,
        raw_path: &str,
        draft: Option<&ClientConfigFileParse>,
    ) -> ConfigResult<ConfigRuleInspection> {
        let normalized_path = resolve_config_path(raw_path)?;
        let content = tokio::fs::read_to_string(&normalized_path)
            .await
            .map_err(ConfigError::IoError)?;
        let parsed = parse_document(&content, Some(&normalized_path))?;
        let inferred_parse = infer_rule_from_document(&parsed.value, parsed.format);
        let validation = draft.map(|rule| validate_rule_against_document(&parsed.value, parsed.format, rule));
        let preview_rule = draft.or(inferred_parse.as_ref());
        let preview = preview_rule
            .and_then(|rule| build_preview_for_rule(&parsed.value, rule))
            .unwrap_or_else(|| limit_preview_value(&parsed.value));
        let preview_text = render_preview_text(&preview, parsed.format);

        Ok(ConfigRuleInspection {
            normalized_path,
            detected_format: Some(parsed.format),
            inferred_parse,
            validation,
            preview,
            preview_text,
            warnings: Vec::new(),
        })
    }

    pub async fn validate_config_file_parse_rule(
        &self,
        raw_path: &str,
        rule: &ClientConfigFileParse,
    ) -> ConfigResult<ConfigRuleValidation> {
        let normalized_path = resolve_config_path(raw_path)?;
        let content = tokio::fs::read_to_string(&normalized_path)
            .await
            .map_err(ConfigError::IoError)?;
        let parsed = parse_document(&content, Some(&normalized_path))?;
        let validation = validate_rule_against_document(&parsed.value, parsed.format, rule);

        if validation.format_matches {
            return Ok(validation);
        }

        let reason = format!(
            "Configured parse rule format '{}' does not match file format '{}'.",
            rule.format.as_str(),
            parsed.format.as_str()
        );

        Err(ConfigError::DataAccessError(reason))
    }

    pub async fn infer_config_file_parse_rule(
        &self,
        raw_path: &str,
    ) -> ConfigResult<Option<ClientConfigFileParse>> {
        let normalized_path = resolve_config_path(raw_path)?;
        let content = tokio::fs::read_to_string(&normalized_path)
            .await
            .map_err(ConfigError::IoError)?;
        let parsed = parse_document(&content, Some(&normalized_path))?;
        Ok(infer_rule_from_document(&parsed.value, parsed.format))
    }
}

fn resolve_config_path(raw_path: &str) -> ConfigResult<String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err(ConfigError::DataAccessError(
            "A configuration file path is required for parse rule inspection.".to_string(),
        ));
    }

    let resolved = get_path_service()
        .resolve_user_path(trimmed)
        .map_err(|err| ConfigError::PathResolutionError(err.to_string()))?;

    Ok(resolved.to_string_lossy().to_string())
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

fn infer_format_from_path(path_hint: Option<&str>) -> Option<TemplateFormat> {
    let path = path_hint?.to_ascii_lowercase();
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

fn validate_rule_against_document(
    document: &Value,
    detected_format: TemplateFormat,
    rule: &ClientConfigFileParse,
) -> ConfigRuleValidation {
    let format_matches = detected_format == rule.format;
    let mut container_found = false;
    let mut server_count = 0;
    let mut matches = false;

    for key in &rule.container_keys {
        if let Some(container) = get_nested_value(document, key) {
            container_found = true;
            let count = server_count_for_container(container, rule.container_type);
            if count > 0 {
                server_count = count;
                matches = true;
                break;
            }
        }
    }

    ConfigRuleValidation {
        matches: format_matches && matches,
        format_matches,
        container_found,
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
        .max_by(|left, right| score_candidate(left).cmp(&score_candidate(right)))?;

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
            let object_count = object_map_server_count(map);
            if object_count > 0 && !path.is_empty() {
                out.push(CandidateRule {
                    path: path.to_string(),
                    container_type: ContainerType::ObjectMap,
                    server_count: object_count,
                    depth: path.matches('.').count(),
                });
            }

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
            let array_count = array_server_count(items);
            if array_count > 0 && !path.is_empty() {
                out.push(CandidateRule {
                    path: path.to_string(),
                    container_type: ContainerType::Array,
                    server_count: array_count,
                    depth: path.matches('.').count(),
                });
            }

            for child in items {
                if child.is_object() {
                    collect_candidates(child, path, out);
                }
            }
        }
        _ => {}
    }
}

fn score_candidate(candidate: &CandidateRule) -> (u32, i32, i32) {
    (
        candidate.server_count,
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

fn render_preview_text(
    value: &Value,
    format: TemplateFormat,
) -> String {
    match format {
        TemplateFormat::Json | TemplateFormat::Json5 => {
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        }
        TemplateFormat::Yaml => serde_yaml::to_string(value)
            .unwrap_or_else(|_| serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()))
            .trim()
            .to_string(),
        TemplateFormat::Toml => render_toml_preview_text(value),
    }
}

fn render_toml_preview_text(value: &Value) -> String {
    if value.is_object() {
        toml::to_string_pretty(value)
            .unwrap_or_else(|_| serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()))
            .trim()
            .to_string()
    } else {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    }
}

fn server_count_for_container(
    container: &Value,
    container_type: ContainerType,
) -> u32 {
    match container_type {
        ContainerType::ObjectMap => container.as_object().map(object_map_server_count).unwrap_or(0),
        ContainerType::Array => container
            .as_array()
            .map(|items| array_server_count(items.as_slice()))
            .unwrap_or(0),
    }
}

fn object_map_server_count(map: &Map<String, Value>) -> u32 {
    map.values().filter(|value| is_server_entry(value)).count() as u32
}

fn array_server_count(items: &[Value]) -> u32 {
    items.iter().filter(|value| is_server_entry(value)).count() as u32
}

fn is_server_entry(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    object.contains_key("command")
        || object.contains_key("url")
        || object.contains_key("baseUrl")
        || object.contains_key("transport")
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
}
