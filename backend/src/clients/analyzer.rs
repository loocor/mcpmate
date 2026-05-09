use crate::clients::models::{ClientConfigFileParse, ContainerType, FormatRule};
use crate::clients::utils::get_nested_value;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigAnalysis {
    pub has_mcp_config: bool,
    pub server_count: u32,
    pub server_names: Vec<String>,
}

impl ConfigAnalysis {
    fn from_server_names(server_names: Vec<String>) -> Self {
        Self {
            has_mcp_config: true,
            server_count: server_names.len() as u32,
            server_names,
        }
    }

    fn present_without_entries() -> Self {
        Self {
            has_mcp_config: true,
            server_count: 0,
            server_names: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectedServerEntry {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigInspection {
    pub matched_container: bool,
    pub entries: Vec<InspectedServerEntry>,
}

struct MatchedTransport<'a> {
    transport: &'a str,
    rule: FormatRule,
}

pub fn parse_config_to_json_value(
    content: &str,
    format: Option<&str>,
) -> Option<serde_json::Value> {
    let normalized = format.map(|v| v.trim().to_ascii_lowercase());

    match normalized.as_deref() {
        Some("json") => serde_json::from_str::<serde_json::Value>(content).ok(),
        Some("json5") => json5::from_str::<serde_json::Value>(content).ok(),
        Some("toml") => toml::from_str::<toml::Value>(content)
            .ok()
            .and_then(|value| serde_json::to_value(value).ok()),
        Some("yaml") => serde_yaml::from_str::<serde_json::Value>(content).ok(),
        Some(_) => None,
        None => serde_json::from_str::<serde_json::Value>(content).ok(),
    }
}

/// Analyze config content for MCP presence and server count according to container configuration.
pub fn analyze_config_content(
    content: &str,
    parse_rule: &ClientConfigFileParse,
    transports: Option<&HashMap<String, FormatRule>>,
) -> ConfigAnalysis {
    if content.is_empty() {
        return ConfigAnalysis::default();
    }

    match parse_config_to_json_value(content, Some(parse_rule.format.as_str())) {
        Some(json) => match inspect_config_value(&json, parse_rule, transports) {
            Ok(inspection) => config_analysis_from_inspection(inspection),
            Err(_) => ConfigAnalysis::present_without_entries(),
        },
        None => ConfigAnalysis::default(),
    }
}

pub fn inspect_config_value(
    document: &Value,
    parse_rule: &ClientConfigFileParse,
    transports: Option<&HashMap<String, FormatRule>>,
) -> Result<ConfigInspection, String> {
    let mut matched_container = false;
    let mut entries = Vec::new();
    let mut seen_names = HashSet::new();

    for container_key in &parse_rule.container_keys {
        let Some(container) = get_nested_value(document, container_key) else {
            continue;
        };

        matched_container = true;
        collect_entries_from_container(
            &mut entries,
            &mut seen_names,
            container,
            parse_rule.container_type,
            container_key,
            transports,
        )?;
    }

    Ok(ConfigInspection {
        matched_container,
        entries,
    })
}

fn config_analysis_from_inspection(inspection: ConfigInspection) -> ConfigAnalysis {
    if !inspection.matched_container {
        return ConfigAnalysis::default();
    }

    let server_names: Vec<String> = inspection.entries.into_iter().map(|entry| entry.name).collect();
    if server_names.is_empty() {
        return ConfigAnalysis::present_without_entries();
    }

    ConfigAnalysis::from_server_names(server_names)
}

fn collect_entries_from_container(
    entries: &mut Vec<InspectedServerEntry>,
    seen_names: &mut HashSet<String>,
    container: &Value,
    container_type: ContainerType,
    container_key: &str,
    transports: Option<&HashMap<String, FormatRule>>,
) -> Result<(), String> {
    match container_type {
        ContainerType::ObjectMap => {
            let Some(map) = container.as_object() else {
                return Err(format!(
                    "configured config node '{container_key}' must resolve to an object map"
                ));
            };

            for (name, value) in map {
                if let Some(entry) = inspect_named_entry(name, value, transports) {
                    if seen_names.insert(entry.name.clone()) {
                        entries.push(entry);
                    }
                }
            }
        }
        ContainerType::Array => {
            let Some(items) = container.as_array() else {
                return Err(format!(
                    "configured config node '{container_key}' must resolve to an array"
                ));
            };

            for value in items {
                let Some(name) = value.get("name").and_then(Value::as_str).map(str::to_string) else {
                    continue;
                };

                if let Some(entry) = inspect_named_entry(&name, value, transports) {
                    if seen_names.insert(entry.name.clone()) {
                        entries.push(entry);
                    }
                }
            }
        }
    }

    Ok(())
}

fn inspect_named_entry(
    name: &str,
    value: &Value,
    transports: Option<&HashMap<String, FormatRule>>,
) -> Option<InspectedServerEntry> {
    let object = value.as_object()?;
    let transports = transports?;
    let matched = matched_transport(object, transports)?;

    Some(InspectedServerEntry {
        name: name.to_string(),
        transport: matched.transport.to_string(),
        command: first_string_value(object, matched.rule.command_field.as_deref(), &["command"]),
        args: first_string_array(object, matched.rule.args_field.as_deref(), &["args"]).unwrap_or_default(),
        env: first_string_map(object, matched.rule.env_field.as_deref(), &["env"]).unwrap_or_default(),
        url: first_string_value(object, matched.rule.url_field.as_deref(), &["url", "baseUrl"]),
    })
}

fn matched_transport<'a>(
    entry: &Map<String, Value>,
    transports: &'a HashMap<String, FormatRule>,
) -> Option<MatchedTransport<'a>> {
    let hint = transport_hint(entry).and_then(normalize_transport_name);
    if let Some(transport) = hint {
        let (name, rule) = transports.get_key_value(transport)?;
        let rule = normalized_transport_rule(name, rule)?;
        return entry_matches_transport(entry, name, &rule).then_some(MatchedTransport { transport: name, rule });
    }

    for transport in ["stdio", "streamable_http", "sse"] {
        let Some((name, rule)) = transports.get_key_value(transport) else {
            continue;
        };
        let Some(rule) = normalized_transport_rule(name, rule) else {
            continue;
        };
        if entry_matches_transport(entry, name, &rule) {
            return Some(MatchedTransport { transport: name, rule });
        }
    }

    None
}

fn normalized_transport_rule(
    transport: &str,
    rule: &FormatRule,
) -> Option<FormatRule> {
    let normalized = rule.normalized();
    normalized.validate_for_transport(transport).ok()?;
    Some(normalized)
}

fn entry_matches_transport(
    entry: &Map<String, Value>,
    transport: &str,
    rule: &FormatRule,
) -> bool {
    if rule.include_type {
        let Some(expected_type) = rule.type_value.as_deref() else {
            return false;
        };
        let Some(expected) = normalize_transport_name(expected_type) else {
            return false;
        };
        let Some(actual) = transport_hint(entry).and_then(normalize_transport_name) else {
            return false;
        };
        if actual != expected {
            return false;
        }
    }

    match transport {
        "stdio" => has_non_empty_string(entry, rule.command_field.as_deref(), &["command"]),
        "sse" | "streamable_http" => has_non_empty_string(entry, rule.url_field.as_deref(), &["url", "baseUrl"]),
        _ => false,
    }
}

fn has_non_empty_string(
    entry: &Map<String, Value>,
    configured: Option<&str>,
    aliases: &[&str],
) -> bool {
    first_string_value(entry, configured, aliases).is_some()
}

fn first_matching_value<'a>(
    entry: &'a Map<String, Value>,
    configured: Option<&str>,
    aliases: &[&str],
) -> Option<&'a Value> {
    if let Some(field) = configured.map(str::trim).filter(|value| !value.is_empty())
        && let Some(value) = entry.get(field)
    {
        return Some(value);
    }

    aliases.iter().filter_map(|field| entry.get(*field)).next()
}

fn first_string_value(
    entry: &Map<String, Value>,
    configured: Option<&str>,
    aliases: &[&str],
) -> Option<String> {
    first_matching_value(entry, configured, aliases).and_then(|value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn first_string_array(
    entry: &Map<String, Value>,
    configured: Option<&str>,
    aliases: &[&str],
) -> Option<Vec<String>> {
    first_matching_value(entry, configured, aliases).and_then(|value| {
        value.as_array().map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect()
        })
    })
}

fn first_string_map(
    entry: &Map<String, Value>,
    configured: Option<&str>,
    aliases: &[&str],
) -> Option<HashMap<String, String>> {
    first_matching_value(entry, configured, aliases).and_then(|value| {
        value.as_object().map(|map| {
            map.iter()
                .filter_map(|(key, value)| value.as_str().map(|text| (key.clone(), text.to_string())))
                .collect()
        })
    })
}

fn transport_hint(entry: &Map<String, Value>) -> Option<&str> {
    ["type", "transport", "kind"]
        .iter()
        .filter_map(|field| entry.get(*field))
        .find_map(Value::as_str)
}

fn normalize_transport_name(raw: &str) -> Option<&str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "stdio" => Some("stdio"),
        "sse" => Some("sse"),
        "http" | "streamablehttp" | "streamable_http" => Some("streamable_http"),
        _ => None,
    }
}

/// Best-effort last modified timestamp extraction in RFC3339.
pub fn get_config_last_modified(config_path: &str) -> Option<String> {
    use std::fs;
    use std::time::SystemTime;
    let expanded = if config_path.starts_with("~/") {
        let home = std::env::var("HOME").ok()?;
        config_path.replacen("~", &home, 1)
    } else {
        config_path.to_string()
    };
    let meta = fs::metadata(&expanded).ok()?;
    let modified = meta.modified().ok()?;
    let dur = modified.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    chrono::DateTime::from_timestamp(dur.as_secs() as i64, 0).map(|dt| dt.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_config_content_returns_object_map_server_names() {
        let transports = HashMap::from([(
            "stdio".to_string(),
            FormatRule {
                command_field: Some("command".to_string()),
                ..FormatRule::default()
            },
        )]);
        let result = analyze_config_content(
            r#"{"context_servers":{"MCPMate":{},"mcp-server-context7":{"enabled":true}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            Some(&transports),
        );

        assert!(result.has_mcp_config);
        assert_eq!(result.server_count, 0);
        assert!(result.server_names.is_empty());
    }

    #[test]
    fn analyze_config_content_ignores_non_matching_containers() {
        let transports = HashMap::from([(
            "stdio".to_string(),
            FormatRule {
                command_field: Some("command".to_string()),
                ..FormatRule::default()
            },
        )]);
        let result = analyze_config_content(
            r#"{"context_servers":{"MCPMate":{}},"agent_servers":{"claude-acp":{"type":"registry"}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            Some(&transports),
        );

        assert!(result.server_names.is_empty());
    }

    #[test]
    fn analyze_config_content_does_not_guess_when_parse_fails() {
        let result = analyze_config_content(
            "{not-json",
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            None,
        );

        assert_eq!(result, ConfigAnalysis::default());
    }

    #[test]
    fn analyze_config_content_accepts_base_url_alias_for_http_entries() {
        let transports = HashMap::from([(
            "streamable_http".to_string(),
            FormatRule {
                url_field: Some("url".to_string()),
                ..FormatRule::default()
            },
        )]);
        let result = analyze_config_content(
            r#"{"context_servers":{"context7":{"baseUrl":"https://example.com/mcp"}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            Some(&transports),
        );

        assert_eq!(result.server_names, vec!["context7".to_string()]);
        assert_eq!(result.server_count, 1);
    }

    #[test]
    fn analyze_config_content_requires_array_name_to_be_string() {
        let transports = HashMap::from([(
            "stdio".to_string(),
            FormatRule {
                command_field: Some("command".to_string()),
                ..FormatRule::default()
            },
        )]);
        let result = analyze_config_content(
            r#"{"servers":[{"name":123,"command":"uvx"},{"name":"valid","command":"bun"}]}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::Array,
                container_keys: vec!["servers".to_string()],
            },
            Some(&transports),
        );

        assert_eq!(result.server_names, vec!["valid".to_string()]);
        assert_eq!(result.server_count, 1);
    }
}
