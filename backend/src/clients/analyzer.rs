use crate::clients::models::{ClientConfigFileParse, ContainerType, FormatRule};
use crate::clients::utils::get_nested_value;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigAnalysis {
    pub has_mcp_config: bool,
    pub server_count: u32,
    pub mcpmate_present: bool,
}

impl ConfigAnalysis {
    fn from_server_count(
        server_count: u32,
        mcpmate_present: bool,
    ) -> Self {
        Self {
            has_mcp_config: true,
            server_count,
            mcpmate_present,
        }
    }

    fn present_without_entries() -> Self {
        Self {
            has_mcp_config: true,
            server_count: 0,
            mcpmate_present: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigImportSkipReason {
    InvalidEntry,
    MissingCommand,
    MissingUrl,
    Unrecognized,
}

impl ConfigImportSkipReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidEntry => "config_invalid_entry",
            Self::MissingCommand => "config_missing_command",
            Self::MissingUrl => "config_missing_url",
            Self::Unrecognized => "config_unrecognized",
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
    pub headers: HashMap<String, String>,
    pub url: Option<String>,
    pub issue: Option<String>,
}

pub struct ResolvedImportTransport<'a> {
    pub kind: &'static str,
    pub command: Option<&'a str>,
    pub url: Option<&'a str>,
}

impl InspectedServerEntry {
    pub fn resolved_import_transport(&self) -> Result<ResolvedImportTransport<'_>, ConfigImportSkipReason> {
        if self.issue.is_some() {
            return Err(ConfigImportSkipReason::InvalidEntry);
        }

        match self.transport.as_str() {
            "stdio" => self
                .command
                .as_deref()
                .map(|command| ResolvedImportTransport {
                    kind: "stdio",
                    command: Some(command),
                    url: None,
                })
                .ok_or(ConfigImportSkipReason::MissingCommand),
            "streamable_http" | "sse" => self
                .url
                .as_deref()
                .map(|url| ResolvedImportTransport {
                    kind: if self.transport == "sse" {
                        "sse"
                    } else {
                        "streamable_http"
                    },
                    command: None,
                    url: Some(url),
                })
                .ok_or(ConfigImportSkipReason::MissingUrl),
            "unclassified" => self
                .inferred_import_transport()
                .ok_or(ConfigImportSkipReason::Unrecognized),
            _ => Err(ConfigImportSkipReason::Unrecognized),
        }
    }

    pub fn import_skip_reason(&self) -> Option<ConfigImportSkipReason> {
        self.resolved_import_transport().err()
    }

    pub fn import_status(&self) -> &'static str {
        if self.import_skip_reason().is_some() {
            "skipped"
        } else {
            "importable"
        }
    }

    fn inferred_import_transport(&self) -> Option<ResolvedImportTransport<'_>> {
        if let Some(url) = self.url.as_deref() {
            return Some(ResolvedImportTransport {
                kind: "streamable_http",
                command: None,
                url: Some(url),
            });
        }

        self.command.as_deref().map(|command| ResolvedImportTransport {
            kind: "stdio",
            command: Some(command),
            url: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigInspection {
    pub matched_container: bool,
    pub entries: Vec<InspectedServerEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigInspectionReport {
    pub document: Value,
    pub inspection: ConfigInspection,
    pub analysis: ConfigAnalysis,
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
    inspect_config_content(content, parse_rule, transports).analysis
}

pub fn inspect_config_content(
    content: &str,
    parse_rule: &ClientConfigFileParse,
    transports: Option<&HashMap<String, FormatRule>>,
) -> ConfigInspectionReport {
    let document = if content.is_empty() {
        Value::Null
    } else {
        parse_config_to_json_value(content, Some(parse_rule.format.as_str())).unwrap_or(Value::Null)
    };
    let inspection = inspect_config_value(&document, parse_rule, transports);
    let analysis = config_analysis_from_inspection(&inspection);

    ConfigInspectionReport {
        document,
        inspection,
        analysis,
    }
}

pub fn inspect_config_value(
    document: &Value,
    parse_rule: &ClientConfigFileParse,
    transports: Option<&HashMap<String, FormatRule>>,
) -> ConfigInspection {
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
        );
    }

    ConfigInspection {
        matched_container,
        entries,
    }
}

fn config_analysis_from_inspection(inspection: &ConfigInspection) -> ConfigAnalysis {
    if !inspection.matched_container {
        return ConfigAnalysis::default();
    }

    let mcpmate_present = inspection
        .entries
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case("MCPMate") && entry_has_transport_fields(entry));
    let server_count = inspection.entries.len() as u32;
    if server_count == 0 {
        return ConfigAnalysis::present_without_entries();
    }

    ConfigAnalysis::from_server_count(server_count, mcpmate_present)
}

fn entry_has_transport_fields(entry: &InspectedServerEntry) -> bool {
    entry.command.is_some() || entry.url.is_some()
}

fn collect_entries_from_container(
    entries: &mut Vec<InspectedServerEntry>,
    seen_names: &mut HashSet<String>,
    container: &Value,
    container_type: ContainerType,
    container_key: &str,
    transports: Option<&HashMap<String, FormatRule>>,
) {
    match container_type {
        ContainerType::ObjectMap => {
            let Some(map) = container.as_object() else {
                tracing::warn!("config node '{container_key}' resolved to non-object, skipping");
                return;
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
                tracing::warn!("config node '{container_key}' resolved to non-array, skipping");
                return;
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
}

fn inspect_named_entry(
    name: &str,
    value: &Value,
    transports: Option<&HashMap<String, FormatRule>>,
) -> Option<InspectedServerEntry> {
    let Some(object) = value.as_object() else {
        return Some(InspectedServerEntry {
            name: name.to_string(),
            transport: "unclassified".to_string(),
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            headers: HashMap::new(),
            url: None,
            issue: Some(ConfigImportSkipReason::InvalidEntry.as_str().to_string()),
        });
    };

    let (transport, rule) = transports
        .and_then(|t| matched_transport(object, t))
        .map(|m| (m.transport.to_string(), m.rule))
        .unwrap_or_else(|| ("unclassified".to_string(), FormatRule::default()));

    Some(InspectedServerEntry {
        name: name.to_string(),
        transport,
        command: first_string_value(object, rule.command_field.as_deref(), &["command"]),
        args: first_string_array(object, rule.args_field.as_deref(), &["args"]).unwrap_or_default(),
        env: first_string_map(object, rule.env_field.as_deref(), &["env"]).unwrap_or_default(),
        headers: first_string_map(object, rule.headers_field.as_deref(), &["headers"]).unwrap_or_default(),
        url: first_string_value(object, rule.url_field.as_deref(), &["url", "baseUrl"]),
        issue: None,
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

    fn entry_names(report: &ConfigInspectionReport) -> Vec<String> {
        report
            .inspection
            .entries
            .iter()
            .map(|entry| entry.name.clone())
            .collect()
    }

    #[test]
    fn inspect_config_content_returns_object_map_entries() {
        let transports = HashMap::from([(
            "stdio".to_string(),
            FormatRule {
                command_field: Some("command".to_string()),
                ..FormatRule::default()
            },
        )]);
        let report = inspect_config_content(
            r#"{"context_servers":{"MCPMate":{},"mcp-server-context7":{"enabled":true}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            Some(&transports),
        );

        assert!(report.analysis.has_mcp_config);
        assert_eq!(report.analysis.server_count, 2);
        assert!(entry_names(&report).contains(&"MCPMate".to_string()));
        assert!(entry_names(&report).contains(&"mcp-server-context7".to_string()));
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
        let report = inspect_config_content(
            r#"{"context_servers":{"MCPMate":{}},"agent_servers":{"claude-acp":{"type":"registry"}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            Some(&transports),
        );

        assert_eq!(entry_names(&report), vec!["MCPMate".to_string()]);
        assert_eq!(report.analysis.server_count, 1);
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
    fn inspect_config_content_returns_document_entries_and_analysis() {
        let transports = HashMap::from([(
            "stdio".to_string(),
            FormatRule {
                command_field: Some("command".to_string()),
                ..FormatRule::default()
            },
        )]);
        let report = inspect_config_content(
            r#"{"context_servers":{"MCPMate":{"command":"uvx"}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            Some(&transports),
        );

        assert!(report.document.is_object());
        assert!(report.inspection.matched_container);
        assert_eq!(report.inspection.entries.len(), 1);
        assert_eq!(entry_names(&report), vec!["MCPMate".to_string()]);
        assert!(report.analysis.mcpmate_present);
    }

    #[test]
    fn analyze_config_content_returns_entries_when_transports_are_empty() {
        let result = analyze_config_content(
            r#"{"context_servers":{"MCPMate":{"url":"http://127.0.0.1:8000/mcp"}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            None,
        );

        assert!(result.has_mcp_config);
        assert_eq!(result.server_count, 1);
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
        let report = inspect_config_content(
            r#"{"context_servers":{"context7":{"baseUrl":"https://example.com/mcp"}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            Some(&transports),
        );

        assert_eq!(entry_names(&report), vec!["context7".to_string()]);
        assert_eq!(report.analysis.server_count, 1);
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
        let report = inspect_config_content(
            r#"{"servers":[{"name":123,"command":"uvx"},{"name":"valid","command":"bun"}]}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::Array,
                container_keys: vec!["servers".to_string()],
            },
            Some(&transports),
        );

        assert_eq!(entry_names(&report), vec!["valid".to_string()]);
        assert_eq!(report.analysis.server_count, 1);
    }

    #[test]
    fn analyze_config_content_sets_mcpmate_present_when_entry_matches() {
        let result = analyze_config_content(
            r#"{"context_servers":{"MCPMate":{"url":"http://127.0.0.1:8000/mcp"},"shadcn":{"enabled":true}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            None,
        );

        assert!(result.mcpmate_present);
        assert_eq!(result.server_count, 2);
    }

    #[test]
    fn analyze_config_content_sets_mcpmate_present_case_insensitive() {
        let result = analyze_config_content(
            r#"{"context_servers":{"mcpmate":{"url":"http://127.0.0.1:8000/mcp"}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            None,
        );

        assert!(result.mcpmate_present);
    }

    #[test]
    fn analyze_config_content_mcpmate_present_false_when_no_match() {
        let result = analyze_config_content(
            r#"{"context_servers":{"shadcn":{"enabled":true},"context7":{"url":"https://example.com"}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            None,
        );

        assert!(!result.mcpmate_present);
        assert_eq!(result.server_count, 2);
    }

    #[test]
    fn analyze_config_content_does_not_mark_empty_mcpmate_object_as_present() {
        let result = analyze_config_content(
            r#"{"context_servers":{"MCPMate":{"enabled":true}}}"#,
            &ClientConfigFileParse {
                format: crate::clients::models::TemplateFormat::Json,
                container_type: ContainerType::ObjectMap,
                container_keys: vec!["context_servers".to_string()],
            },
            None,
        );

        assert!(!result.mcpmate_present);
        assert_eq!(result.server_count, 1);
    }
}
