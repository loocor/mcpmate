// Utility helpers for client configuration JSON manipulation
// Moved from legacy config::client::utils to live alongside the template engine.

use serde_json::Value;

/// Set a value at a dot-delimited path within a JSON object, e.g. `mcp.servers`
pub fn set_nested_value(
    config: &mut Value,
    path: &str,
    value: Value,
) {
    if path.is_empty() {
        return;
    }

    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() == 1 {
        config[path] = value;
        return;
    }

    let mut current = config;
    let last_index = parts.len() - 1;

    for (i, part) in parts.iter().enumerate() {
        if i == last_index {
            current[part] = value;
            return;
        }

        if !current[part].is_object() {
            current[part] = serde_json::json!({});
        }
        current = &mut current[part];
    }
}

/// Retrieve a value from a dot-delimited path within a JSON object.
pub fn get_nested_value<'a>(
    config: &'a Value,
    path: &str,
) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(config);
    }

    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() == 1 {
        return config.get(path);
    }

    let mut current = config;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn set_nested_value_simple() {
        let mut config = json!({});
        set_nested_value(&mut config, "key", json!("value"));
        assert_eq!(config["key"], "value");
    }

    #[test]
    fn set_nested_value_nested() {
        let mut config = json!({});
        set_nested_value(&mut config, "mcp.servers", json!({"server1": {}}));
        assert_eq!(config["mcp"]["servers"]["server1"], json!({}));
    }

    #[test]
    fn get_nested_value_simple() {
        let config = json!({"key": "value"});
        assert_eq!(get_nested_value(&config, "key"), Some(&json!("value")));
    }

    #[test]
    fn get_nested_value_nested() {
        let config = json!({"mcp": {"servers": {"server1": {}}}});
        assert_eq!(get_nested_value(&config, "mcp.servers"), Some(&json!({"server1": {}})));
    }

    #[test]
    fn get_nested_value_missing() {
        let config = json!({"key": "value"});
        assert_eq!(get_nested_value(&config, "missing"), None);
        assert_eq!(get_nested_value(&config, "mcp.servers"), None);
    }

    #[test]
    fn empty_path_returns_root() {
        let config = json!({"key": "value"});
        let mut mutated = config.clone();

        assert_eq!(get_nested_value(&config, ""), Some(&config));
        set_nested_value(&mut mutated, "", json!("ignored"));
        assert_eq!(mutated, config);
    }
}
