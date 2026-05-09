// Client import helpers: parse only. Dedup/write are handled by config::server::import core.

use crate::api::models::server::ServersImportConfig;
use crate::clients::analyzer::inspect_config_value;
use crate::clients::models::{ClientConfigFileParse, FormatRule};
use serde_json::Value;
use std::collections::HashMap;

/// Build a unified import payload from a known client config value using its effective
/// parse rule and transport format rules.
pub fn build_import_payload_from_value(
    config: &Value,
    parse_rule: &ClientConfigFileParse,
    transports: &HashMap<String, FormatRule>,
) -> anyhow::Result<HashMap<String, ServersImportConfig>> {
    let inspection = inspect_config_value(config, parse_rule, Some(transports)).map_err(anyhow::Error::msg)?;
    if !inspection.matched_container {
        anyhow::bail!(
            "none of the configured config nodes matched: {}",
            parse_rule.container_keys.join(", ")
        );
    }

    let mut out = HashMap::new();
    for entry in inspection.entries {
        let (kind, command, url) = match entry.transport.as_str() {
            "stdio" => ("stdio".to_string(), entry.command.clone(), None),
            "streamable_http" | "sse" => ("streamable_http".to_string(), None, entry.url.clone()),
            _ => continue,
        };

        let mut env = entry.env;
        let args = merge_key_equals_into_env(entry.args, &mut env);
        out.insert(
            entry.name,
            ServersImportConfig {
                kind,
                command,
                args: Some(args),
                url,
                env: Some(env),
                headers: None,
                registry_server_id: None,
                meta: None,
            },
        );
    }

    Ok(out)
}

/// Move KEY=VALUE items from args to env. Returns filtered args.
fn merge_key_equals_into_env(
    args: Vec<String>,
    env: &mut HashMap<String, String>,
) -> Vec<String> {
    let mut filtered = Vec::with_capacity(args.len());
    for arg in args {
        if let Some((key, value)) = parse_env_assignment(&arg) {
            use std::collections::hash_map::Entry;
            match env.entry(key) {
                Entry::Occupied(_) => {}
                Entry::Vacant(entry) => {
                    entry.insert(value);
                }
            }
            continue;
        }

        filtered.push(arg);
    }
    filtered
}

fn parse_env_assignment(raw: &str) -> Option<(String, String)> {
    if raw.starts_with('-') {
        return None;
    }

    let eq = raw.find('=')?;
    let (key, value_with_equals) = raw.split_at(eq);
    if key.is_empty() {
        return None;
    }

    let mut value = value_with_equals[1..].trim().to_string();
    if ((value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\'')))
        && value.len() >= 2
    {
        value = value[1..value.len() - 1].to_string();
    }

    let mut chars = key.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {}
        _ => return None,
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return None;
    }

    Some((key.to_string(), value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::models::{ClientConfigFileParse, ContainerType, TemplateFormat};

    fn transport_rules() -> HashMap<String, FormatRule> {
        HashMap::from([
            (
                "stdio".to_string(),
                FormatRule {
                    command_field: Some("command".to_string()),
                    args_field: Some("args".to_string()),
                    env_field: Some("env".to_string()),
                    ..FormatRule::default()
                },
            ),
            (
                "streamable_http".to_string(),
                FormatRule {
                    url_field: Some("url".to_string()),
                    ..FormatRule::default()
                },
            ),
        ])
    }

    #[test]
    fn build_import_payload_uses_config_file_parse_for_zed_context_servers() {
        let config = serde_json::json!({
            "context_servers": {
                "zed-mcp": {
                    "command": "uvx",
                    "args": ["server.py"],
                    "env": {"DEBUG": "1"}
                }
            }
        });

        let parse_rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["context_servers".to_string()],
        };

        let payload =
            build_import_payload_from_value(&config, &parse_rule, &transport_rules()).expect("zed import payload");
        let server = payload.get("zed-mcp").expect("zed server entry");

        assert_eq!(server.kind, "stdio");
        assert_eq!(server.command.as_deref(), Some("uvx"));
        assert_eq!(server.args.as_deref(), Some(&["server.py".to_string()][..]));
        assert_eq!(
            server.env.as_ref().and_then(|env| env.get("DEBUG")),
            Some(&"1".to_string())
        );
    }

    #[test]
    fn build_import_payload_does_not_fallback_when_parse_rule_is_set() {
        let config = serde_json::json!({
            "context_servers": {
                "zed-mcp": {"url": "http://localhost:8000/mcp"}
            },
            "mcpServers": {
                "wrong-node": {"command": "uvx", "args": ["wrong.py"]}
            },
            "agent_servers": {
                "registry-entry": {"type": "registry"}
            }
        });

        let parse_rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["context_servers".to_string()],
        };

        let payload =
            build_import_payload_from_value(&config, &parse_rule, &transport_rules()).expect("zed import payload");

        assert!(payload.contains_key("zed-mcp"));
        assert!(!payload.contains_key("wrong-node"));
        assert!(!payload.contains_key("registry-entry"));
    }

    #[test]
    fn build_import_payload_combines_multiple_config_nodes() {
        let config = serde_json::json!({
            "primary": {
                "one": {"command": "uvx", "args": ["one.py"]}
            },
            "secondary": {
                "two": {"url": "http://localhost:9000/mcp"}
            }
        });

        let parse_rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["primary".to_string(), "secondary".to_string()],
        };

        let payload = build_import_payload_from_value(&config, &parse_rule, &transport_rules())
            .expect("multi-node import payload");

        assert!(payload.contains_key("one"));
        assert!(payload.contains_key("two"));
        assert_eq!(payload.len(), 2);
    }

    #[test]
    fn build_import_payload_keeps_first_duplicate_name_across_config_nodes() {
        let config = serde_json::json!({
            "primary": {
                "shared": {"command": "uvx", "args": ["primary.py"]}
            },
            "secondary": {
                "shared": {"command": "uvx", "args": ["secondary.py"]}
            }
        });

        let parse_rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["primary".to_string(), "secondary".to_string()],
        };

        let payload =
            build_import_payload_from_value(&config, &parse_rule, &transport_rules()).expect("deduplicated payload");
        let server = payload.get("shared").expect("shared server entry");

        assert_eq!(server.command.as_deref(), Some("uvx"));
        assert_eq!(server.args.as_deref(), Some(&["primary.py".to_string()][..]));
    }

    #[test]
    fn build_import_payload_errors_when_config_nodes_do_not_match() {
        let config = serde_json::json!({
            "context_servers": {
                "zed-mcp": {"url": "http://localhost:8000/mcp"}
            }
        });

        let parse_rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["missing.node".to_string()],
        };

        let err = build_import_payload_from_value(&config, &parse_rule, &transport_rules())
            .expect_err("missing nodes should error");

        assert!(
            err.to_string()
                .contains("none of the configured config nodes matched: missing.node")
        );
    }

    #[test]
    fn build_import_payload_errors_when_config_node_shape_is_invalid() {
        let config = serde_json::json!({
            "context_servers": []
        });

        let parse_rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["context_servers".to_string()],
        };

        let err = build_import_payload_from_value(&config, &parse_rule, &transport_rules())
            .expect_err("invalid container shape should error");

        assert!(
            err.to_string()
                .contains("configured config node 'context_servers' must resolve to an object map")
        );
    }

    #[test]
    fn build_import_payload_accepts_base_url_alias_from_inspection_core() {
        let config = serde_json::json!({
            "context_servers": {
                "remote": {
                    "baseUrl": "https://example.com/mcp"
                }
            }
        });

        let parse_rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::ObjectMap,
            container_keys: vec!["context_servers".to_string()],
        };

        let payload =
            build_import_payload_from_value(&config, &parse_rule, &transport_rules()).expect("baseUrl import payload");
        let server = payload.get("remote").expect("remote server entry");

        assert_eq!(server.kind, "streamable_http");
        assert_eq!(server.url.as_deref(), Some("https://example.com/mcp"));
    }

    #[test]
    fn build_import_payload_ignores_array_entries_without_string_name() {
        let config = serde_json::json!({
            "servers": [
                {"name": 7, "command": "uvx"},
                {"name": "valid", "command": "bun"}
            ]
        });

        let parse_rule = ClientConfigFileParse {
            format: TemplateFormat::Json,
            container_type: ContainerType::Array,
            container_keys: vec!["servers".to_string()],
        };

        let payload =
            build_import_payload_from_value(&config, &parse_rule, &transport_rules()).expect("array import payload");

        assert_eq!(payload.len(), 1);
        assert!(payload.contains_key("valid"));
    }
}
