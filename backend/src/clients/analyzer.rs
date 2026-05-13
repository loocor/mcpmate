use crate::clients::document::parse_config_to_json_value;
use crate::clients::models::{
    CONFIG_TRANSPORT_PRIORITY, ClientConfigFileParse, ContainerType, FormatRule, TemplateFormat,
};
use crate::clients::utils::get_nested_value;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ConfigAnalysis {
    pub(crate) has_mcp_config: bool,
    pub(crate) server_count: u32,
    pub(crate) mcpmate_present: bool,
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
pub(crate) enum ConfigImportSkipReason {
    InvalidEntry,
    MissingCommand,
    MissingUrl,
    Unrecognized,
}

impl ConfigImportSkipReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InvalidEntry => "config_invalid_entry",
            Self::MissingCommand => "config_missing_command",
            Self::MissingUrl => "config_missing_url",
            Self::Unrecognized => "config_unrecognized",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectedServerEntry {
    pub(crate) name: String,
    pub(crate) transport: String,
    pub(crate) command: Option<String>,
    pub(crate) args: Vec<String>,
    pub(crate) env: HashMap<String, String>,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) url: Option<String>,
    pub(crate) issue: Option<String>,
}

pub(crate) struct ResolvedImportTransport<'a> {
    pub(crate) kind: &'static str,
    pub(crate) command: Option<&'a str>,
    pub(crate) url: Option<&'a str>,
}

impl InspectedServerEntry {
    pub(crate) fn has_transport_target(&self) -> bool {
        self.command.is_some() || self.url.is_some()
    }

    pub(crate) fn resolved_import_transport(&self) -> Result<ResolvedImportTransport<'_>, ConfigImportSkipReason> {
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
pub(crate) struct ConfigInspection {
    pub(crate) matched_container: bool,
    pub(crate) entries: Vec<InspectedServerEntry>,
}

impl ConfigInspection {
    pub(crate) fn server_count(&self) -> u32 {
        self.entries.iter().filter(|entry| entry.has_transport_target()).count() as u32
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConfigInspectionReport {
    pub(crate) document: Value,
    pub(crate) inspection: ConfigInspection,
    pub(crate) analysis: ConfigAnalysis,
}

struct MatchedTransport<'a> {
    transport: &'a str,
    rule: FormatRule,
}

pub(crate) fn inspect_config_content(
    content: &str,
    parse_rule: &ClientConfigFileParse,
    transports: Option<&HashMap<String, FormatRule>>,
) -> ConfigInspectionReport {
    let document = if content.is_empty() {
        Value::Null
    } else {
        parse_config_to_json_value(content, Some(parse_rule.format.as_str()))
            .or_else(|| {
                if matches!(parse_rule.format, TemplateFormat::Json) {
                    parse_config_to_json_value(content, Some("json5"))
                } else {
                    None
                }
            })
            .unwrap_or(Value::Null)
    };
    let inspection = inspect_config_value(&document, parse_rule, transports);
    let analysis = config_analysis_from_inspection(&inspection);

    ConfigInspectionReport {
        document,
        inspection,
        analysis,
    }
}

pub(crate) fn inspect_config_value(
    document: &Value,
    parse_rule: &ClientConfigFileParse,
    transports: Option<&HashMap<String, FormatRule>>,
) -> ConfigInspection {
    let mut matched_container = false;
    let mut entries = Vec::new();
    let mut seen_names = HashSet::new();

    for container_key in &parse_rule.container_keys {
        for container in matching_nested_values(document, container_key) {
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
    }

    ConfigInspection {
        matched_container,
        entries,
    }
}

fn matching_nested_values<'a>(
    document: &'a Value,
    container_key: &str,
) -> Vec<&'a Value> {
    if !container_key.contains('*') {
        return get_nested_value(document, container_key).into_iter().collect();
    }

    let parts = container_key.split('.').collect::<Vec<_>>();
    let mut matches = Vec::new();
    collect_matching_nested_values(document, &parts, &mut matches);
    matches
}

fn collect_matching_nested_values<'a>(
    current: &'a Value,
    parts: &[&str],
    matches: &mut Vec<&'a Value>,
) {
    let Some((part, remaining)) = parts.split_first() else {
        matches.push(current);
        return;
    };

    if *part == "*" {
        if let Some(map) = current.as_object() {
            for value in map.values() {
                collect_matching_nested_values(value, remaining, matches);
            }
        } else if let Some(items) = current.as_array() {
            for value in items {
                collect_matching_nested_values(value, remaining, matches);
            }
        }
        return;
    }

    if let Some(next) = current.get(*part) {
        collect_matching_nested_values(next, remaining, matches);
    }
}

fn config_analysis_from_inspection(inspection: &ConfigInspection) -> ConfigAnalysis {
    if !inspection.matched_container {
        return ConfigAnalysis::default();
    }

    let mcpmate_present = inspection
        .entries
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case("MCPMate") && entry.has_transport_target());
    let server_count = inspection.server_count();
    if server_count == 0 {
        return ConfigAnalysis::present_without_entries();
    }

    ConfigAnalysis::from_server_count(server_count, mcpmate_present)
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
                let entry = inspect_named_entry(name, value, transports);
                push_entry_if_new(entries, seen_names, entry);
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

                let entry = inspect_named_entry(&name, value, transports);
                push_entry_if_new(entries, seen_names, entry);
            }
        }
    }
}

fn push_entry_if_new(
    entries: &mut Vec<InspectedServerEntry>,
    seen_names: &mut HashSet<String>,
    entry: InspectedServerEntry,
) {
    if seen_names.insert(entry.name.clone()) {
        entries.push(entry);
    }
}

fn inspect_named_entry(
    name: &str,
    value: &Value,
    transports: Option<&HashMap<String, FormatRule>>,
) -> InspectedServerEntry {
    let Some(object) = value.as_object() else {
        return InspectedServerEntry {
            name: name.to_string(),
            transport: "unclassified".to_string(),
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            headers: HashMap::new(),
            url: None,
            issue: Some(ConfigImportSkipReason::InvalidEntry.as_str().to_string()),
        };
    };

    let (transport, rule) = transports
        .and_then(|t| matched_transport(object, t))
        .map(|m| (m.transport.to_string(), m.rule))
        .unwrap_or_else(|| ("unclassified".to_string(), FormatRule::default()));

    InspectedServerEntry {
        name: name.to_string(),
        transport,
        command: first_string_value(object, rule.command_field.as_deref(), &["command"]),
        args: first_string_array(object, rule.args_field.as_deref(), &["args"]).unwrap_or_default(),
        env: first_string_map(object, rule.env_field.as_deref(), &["env"]).unwrap_or_default(),
        headers: first_string_map(object, rule.headers_field.as_deref(), &["headers"]).unwrap_or_default(),
        url: first_string_value(object, rule.url_field.as_deref(), &["url", "baseUrl"]),
        issue: None,
    }
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

    for transport in CONFIG_TRANSPORT_PRIORITY {
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
        "http" | "streamablehttp" | "streamable-http" | "streamable_http" => Some("streamable_http"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_map_rule(container_key: &str) -> ClientConfigFileParse {
        ClientConfigFileParse {
            format: crate::clients::models::TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec![container_key.to_string()],
        }
    }

    fn array_rule(container_key: &str) -> ClientConfigFileParse {
        ClientConfigFileParse {
            format: crate::clients::models::TemplateFormat::Json,
            container_type: ContainerType::Array,
            container_keys: vec![container_key.to_string()],
        }
    }

    fn stdio_transports() -> HashMap<String, FormatRule> {
        HashMap::from([(
            "stdio".to_string(),
            FormatRule {
                command_field: Some("command".to_string()),
                ..FormatRule::default()
            },
        )])
    }

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
        let transports = stdio_transports();
        let report = inspect_config_content(
            r#"{"context_servers":{"MCPMate":{"command":"uvx"},"mcp-server-context7":{"command":"bunx"}}}"#,
            &object_map_rule("context_servers"),
            Some(&transports),
        );

        assert!(report.analysis.has_mcp_config);
        assert_eq!(report.analysis.server_count, 2);
        assert!(entry_names(&report).contains(&"MCPMate".to_string()));
        assert!(entry_names(&report).contains(&"mcp-server-context7".to_string()));
    }

    #[test]
    fn inspect_config_content_ignores_non_matching_containers() {
        let transports = stdio_transports();
        let report = inspect_config_content(
            r#"{"context_servers":{"MCPMate":{"command":"uvx"}},"agent_servers":{"claude-acp":{"type":"registry"}}}"#,
            &object_map_rule("context_servers"),
            Some(&transports),
        );

        assert_eq!(entry_names(&report), vec!["MCPMate".to_string()]);
        assert_eq!(report.analysis.server_count, 1);
    }

    #[test]
    fn inspect_config_content_matches_wildcard_container_segments() {
        let transports = stdio_transports();
        let report = inspect_config_content(
            r#"{
                "projects": {
                    "/Volumes/External/GitHub/MCPMate": {
                        "mcpServers": {
                            "project-server": {"command":"node","args":["server.js"]}
                        }
                    }
                }
            }"#,
            &object_map_rule("projects.*.mcpServers"),
            Some(&transports),
        );

        assert!(report.inspection.matched_container);
        assert_eq!(entry_names(&report), vec!["project-server".to_string()]);
        assert_eq!(report.analysis.server_count, 1);
    }

    #[test]
    fn inspect_config_content_does_not_guess_when_parse_fails() {
        let result = inspect_config_content("{not-json", &object_map_rule("context_servers"), None).analysis;

        assert_eq!(result, ConfigAnalysis::default());
    }

    #[test]
    fn inspect_config_content_returns_document_entries_and_analysis() {
        let transports = stdio_transports();
        let report = inspect_config_content(
            r#"{"context_servers":{"MCPMate":{"command":"uvx"}}}"#,
            &object_map_rule("context_servers"),
            Some(&transports),
        );

        assert!(report.document.is_object());
        assert!(report.inspection.matched_container);
        assert_eq!(report.inspection.entries.len(), 1);
        assert_eq!(entry_names(&report), vec!["MCPMate".to_string()]);
        assert!(report.analysis.mcpmate_present);
    }

    #[test]
    fn inspect_config_content_returns_entries_when_transports_are_empty() {
        let result = inspect_config_content(
            r#"{"context_servers":{"MCPMate":{"url":"http://127.0.0.1:8000/mcp"}}}"#,
            &object_map_rule("context_servers"),
            None,
        )
        .analysis;

        assert!(result.has_mcp_config);
        assert_eq!(result.server_count, 1);
    }

    #[test]
    fn inspect_config_content_accepts_base_url_alias_for_http_entries() {
        let transports = HashMap::from([(
            "streamable_http".to_string(),
            FormatRule {
                url_field: Some("url".to_string()),
                ..FormatRule::default()
            },
        )]);
        let report = inspect_config_content(
            r#"{"context_servers":{"context7":{"baseUrl":"https://example.com/mcp"}}}"#,
            &object_map_rule("context_servers"),
            Some(&transports),
        );

        assert_eq!(entry_names(&report), vec!["context7".to_string()]);
        assert_eq!(report.analysis.server_count, 1);
    }

    #[test]
    fn inspect_config_content_requires_array_name_to_be_string() {
        let transports = stdio_transports();
        let report = inspect_config_content(
            r#"{"servers":[{"name":123,"command":"uvx"},{"name":"valid","command":"bun"}]}"#,
            &array_rule("servers"),
            Some(&transports),
        );

        assert_eq!(entry_names(&report), vec!["valid".to_string()]);
        assert_eq!(report.analysis.server_count, 1);
    }

    #[test]
    fn inspect_config_content_sets_mcpmate_present_when_entry_matches() {
        let result = inspect_config_content(
            r#"{"context_servers":{"MCPMate":{"url":"http://127.0.0.1:8000/mcp"},"shadcn":{"enabled":true}}}"#,
            &object_map_rule("context_servers"),
            None,
        )
        .analysis;

        assert!(result.mcpmate_present);
        assert_eq!(result.server_count, 1);
    }

    #[test]
    fn inspect_config_content_sets_mcpmate_present_case_insensitive() {
        let result = inspect_config_content(
            r#"{"context_servers":{"mcpmate":{"url":"http://127.0.0.1:8000/mcp"}}}"#,
            &object_map_rule("context_servers"),
            None,
        )
        .analysis;

        assert!(result.mcpmate_present);
    }

    #[test]
    fn inspect_config_content_mcpmate_present_false_when_no_match() {
        let result = inspect_config_content(
            r#"{"context_servers":{"shadcn":{"enabled":true},"context7":{"url":"https://example.com"}}}"#,
            &object_map_rule("context_servers"),
            None,
        )
        .analysis;

        assert!(!result.mcpmate_present);
        assert_eq!(result.server_count, 1);
    }

    #[test]
    fn inspect_config_content_does_not_mark_empty_mcpmate_object_as_present() {
        let result = inspect_config_content(
            r#"{"context_servers":{"MCPMate":{"enabled":true}}}"#,
            &object_map_rule("context_servers"),
            None,
        )
        .analysis;

        assert!(!result.mcpmate_present);
        assert_eq!(result.server_count, 0);
    }

    /// Cursor templates ship static `type` in the outbound schema while `requires_type_field: false`.
    /// User entries often omit `type` (Cursor remote example is URL-only). `FormatRule::normalized` must
    /// not set inbound `include_type` unless the rule explicitly opted in.
    #[test]
    fn cursor_like_streamable_rule_matches_url_only_entry_without_user_type() {
        let mut transports = HashMap::new();
        transports.insert(
            "streamable_http".to_string(),
            FormatRule {
                template: serde_json::json!({
                    "type": "streamable_http",
                    "url": "{{{url}}}"
                }),
                include_type: false,
                ..FormatRule::default()
            },
        );
        let value = serde_json::json!({
            "url": "http://127.0.0.1:9/mcp"
        });
        let entry = inspect_named_entry("context-mode", &value, Some(&transports));
        assert_eq!(entry.transport, "streamable_http");
        let resolved = entry.resolved_import_transport().expect("resolved transport");
        assert_eq!(resolved.kind, "streamable_http");
    }

    #[test]
    fn cursor_like_stdio_rule_matches_command_only_entry_without_user_type() {
        let mut transports = HashMap::new();
        transports.insert(
            "stdio".to_string(),
            FormatRule {
                template: serde_json::json!({
                    "type": "stdio",
                    "command": "{{command}}",
                    "args": "{{{json args}}}"
                }),
                include_type: false,
                ..FormatRule::default()
            },
        );
        let value = serde_json::json!({
            "command": "npx",
            "args": ["-y", "@lobster/context-mode"]
        });
        let entry = inspect_named_entry("context-mode", &value, Some(&transports));
        assert_eq!(entry.transport, "stdio");
        let resolved = entry.resolved_import_transport().expect("resolved transport");
        assert_eq!(resolved.kind, "stdio");
    }
}
