use crate::clients::utils::get_nested_value;

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
    container_keys: &[String],
    is_array_container: bool,
    format: Option<&str>,
) -> ConfigAnalysis {
    if content.is_empty() {
        return ConfigAnalysis::default();
    }

    match parse_config_to_json_value(content, format) {
        Some(json) => match is_array_container {
            true => {
                if let Some(arr) = json.as_array() {
                    return analyze_array_entries(arr);
                }
                for key in container_keys {
                    if let Some(val) = get_nested_value(&json, key) {
                        if let Some(arr) = val.as_array() {
                            return analyze_array_entries(arr);
                        } else if !val.is_null() {
                            return ConfigAnalysis::present_without_entries();
                        }
                    }
                }
                ConfigAnalysis::default()
            }
            false => {
                for key in container_keys {
                    if let Some(servers) = get_nested_value(&json, key) {
                        if let Some(obj) = servers.as_object() {
                            return analyze_object_entries(obj);
                        } else if servers.is_null() || servers.is_array() || servers.is_string() {
                            return ConfigAnalysis::present_without_entries();
                        }
                    }
                }
                ConfigAnalysis::default()
            }
        },
        None => ConfigAnalysis::default(),
    }
}

fn analyze_array_entries(entries: &[serde_json::Value]) -> ConfigAnalysis {
    let server_names: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry.get("name").and_then(|name| name.as_str()).map(str::to_string))
        .collect();
    let has_mcp_config = entries
        .iter()
        .any(|entry| entry.get("name").is_some() && (entry.get("command").is_some() || entry.get("url").is_some()));

    ConfigAnalysis {
        has_mcp_config,
        server_count: entries.len() as u32,
        server_names,
    }
}

fn analyze_object_entries(entries: &serde_json::Map<String, serde_json::Value>) -> ConfigAnalysis {
    ConfigAnalysis::from_server_names(entries.keys().cloned().collect())
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
        let result = analyze_config_content(
            r#"{"context_servers":{"MCPMate":{},"mcp-server-context7":{"enabled":true}}}"#,
            &["context_servers".to_string()],
            false,
            Some("json"),
        );

        assert!(result.has_mcp_config);
        assert_eq!(result.server_count, 2);
        assert_eq!(
            result.server_names,
            vec!["MCPMate".to_string(), "mcp-server-context7".to_string()]
        );
    }

    #[test]
    fn analyze_config_content_ignores_non_matching_containers() {
        let result = analyze_config_content(
            r#"{"context_servers":{"MCPMate":{}},"agent_servers":{"claude-acp":{"type":"registry"}}}"#,
            &["context_servers".to_string()],
            false,
            Some("json"),
        );

        assert_eq!(result.server_names, vec!["MCPMate".to_string()]);
    }

    #[test]
    fn analyze_config_content_does_not_guess_when_parse_fails() {
        let result = analyze_config_content("{not-json", &["context_servers".to_string()], false, Some("json"));

        assert_eq!(result, ConfigAnalysis::default());
    }
}
