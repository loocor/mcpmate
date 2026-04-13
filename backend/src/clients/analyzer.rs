use crate::clients::utils::get_nested_value;

fn parse_to_json_value(
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
) -> (bool, u32) {
    if content.is_empty() {
        return (false, 0);
    }
    let keys = container_keys;
    let is_array = is_array_container;

    match parse_to_json_value(content, format) {
        Some(json) => {
            if is_array {
                if let Some(arr) = json.as_array() {
                    let has = !arr.is_empty()
                        && arr.iter().any(|it| {
                            it.get("name").is_some() && (it.get("command").is_some() || it.get("url").is_some())
                        });
                    return (has, arr.len() as u32);
                }
                for key in keys {
                    if let Some(val) = get_nested_value(&json, key) {
                        if let Some(arr) = val.as_array() {
                            let has = !arr.is_empty()
                                && arr.iter().any(|it| {
                                    it.get("name").is_some() && (it.get("command").is_some() || it.get("url").is_some())
                                });
                            return (has, arr.len() as u32);
                        } else if !val.is_null() {
                            return (true, 0);
                        }
                    }
                }
                (false, 0)
            } else {
                for key in keys {
                    if let Some(servers) = get_nested_value(&json, key) {
                        if let Some(obj) = servers.as_object() {
                            return (true, obj.len() as u32);
                        } else if servers.is_null() || servers.is_array() || servers.is_string() {
                            return (true, 0);
                        }
                    }
                }
                (false, 0)
            }
        }
        None => {
            if is_array {
                let has = content.contains("[") && (content.contains("\"command\"") || content.contains("\"url\""));
                return (has, 0);
            }
            if keys.is_empty() {
                return (false, 0);
            }
            let has = keys.iter().any(|k| {
                let leaf = k.rsplit('.').next().unwrap_or(k);
                content.contains(leaf)
            });
            (has, 0)
        }
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
