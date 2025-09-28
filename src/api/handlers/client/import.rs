// Client import helpers: parse only. Dedup/write are handled by config::server::import core.

use crate::api::models::server::ServersImportConfig;
use crate::common::constants::config_keys;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedTransport {
    Stdio,
    Sse,
    StreamableHttp,
}

#[derive(Debug, Clone)]
struct ServerConfigParsed {
    transport: ParsedTransport,
    command: Option<String>,
    args: Vec<String>,
    env: std::collections::HashMap<String, String>,
    url: Option<String>,
}

/// Build a unified import payload from client config JSON value (object or array form)
pub fn build_import_payload_from_value(config: &Value) -> HashMap<String, ServersImportConfig> {
    let mut out = HashMap::new();
    if let Ok(map) = extract_servers(config) {
        for (name, sc) in map.into_iter() {
            let (kind, command, url) = match sc.transport {
                ParsedTransport::Stdio => ("stdio".to_string(), sc.command, None),
                ParsedTransport::Sse => ("sse".to_string(), None, sc.url),
                ParsedTransport::StreamableHttp => ("streamable_http".to_string(), None, sc.url),
            };
            out.insert(
                name,
                ServersImportConfig {
                    kind,
                    command,
                    args: Some(sc.args),
                    url,
                    env: Some(sc.env),
                    registry_server_id: None,
                },
            );
        }
    }
    out
}

fn extract_servers(config: &Value) -> anyhow::Result<HashMap<String, ServerConfigParsed>> {
    let mut servers = HashMap::new();

    // Object form (e.g., Claude Desktop)
    if let Some(mcp_servers) = config.get(config_keys::MCP_SERVERS).and_then(|v| v.as_object()) {
        for (name, sc) in mcp_servers {
            if let Some(parsed) = parse_server_config(sc) {
                servers.insert(name.clone(), parsed);
            }
        }
    } else if let Some(array) = config.as_array() {
        // Array form (e.g., Augment)
        for (idx, sc) in array.iter().enumerate() {
            let name = sc
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("server_{}", idx));
            if let Some(parsed) = parse_server_config(sc) {
                servers.insert(name, parsed);
            }
        }
    }

    Ok(servers)
}

fn parse_server_config(config: &Value) -> Option<ServerConfigParsed> {
    let hint = config
        .get("type")
        .or_else(|| config.get("transport"))
        .or_else(|| config.get("kind"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase());

    let command = config.get("command").and_then(|v| v.as_str()).map(|s| s.to_string());
    let url = config.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());

    let transport = match (command.as_ref(), url.as_ref(), hint.as_deref()) {
        (Some(_), _, _) => ParsedTransport::Stdio,
        (None, Some(_), Some("streamable_http" | "http" | "streamablehttp")) => ParsedTransport::StreamableHttp,
        (None, Some(_), _) => ParsedTransport::Sse,
        _ => return None,
    };

    let mut args: Vec<String> = config
        .get("args")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
        .unwrap_or_default();

    let mut env = config
        .get("env")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    // Merge KEY=VALUE patterns from args into env (safer handling for credentials)
    args = merge_key_equals_into_env(args, &mut env);

    Some(ServerConfigParsed {
        transport,
        command,
        args,
        env,
        url,
    })
}

/// Move KEY=VALUE items from args to env. Returns filtered args.
fn merge_key_equals_into_env(
    args: Vec<String>,
    env: &mut std::collections::HashMap<String, String>,
) -> Vec<String> {
    let mut filtered = Vec::with_capacity(args.len());
    for a in args.into_iter() {
        if let Some((k, v)) = parse_env_assignment(&a) {
            use std::collections::hash_map::Entry;
            match env.entry(k) {
                Entry::Occupied(_) => {}
                Entry::Vacant(e) => {
                    e.insert(v);
                }
            }
        } else {
            filtered.push(a);
        }
    }
    filtered
}

fn parse_env_assignment(s: &str) -> Option<(String, String)> {
    // Reject flags like --foo=bar
    if s.starts_with('-') {
        return None;
    }
    let eq = s.find('=')?;
    let (k, v) = s.split_at(eq);
    if k.is_empty() {
        return None;
    }
    // v starts with '='
    let mut value = v[1..].trim().to_string();
    // Strip matching quotes
    if ((value.starts_with('"') && value.ends_with('"')) || (value.starts_with('\'') && value.ends_with('\'')))
        && value.len() >= 2
    {
        value = value[1..value.len() - 1].to_string();
    }
    // Accept typical env var keys
    let valid_key = {
        let mut chars = k.chars();
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => (),
            _ => return None,
        };
        chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    if !valid_key {
        return None;
    }
    Some((k.to_string(), value))
}
