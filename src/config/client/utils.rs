// Utility functions for client configuration processing
// Contains shared helper functions for JSON path manipulation

use serde_json::Value;

/// Helper function to set a value at a nested path in a JSON object
/// Supports paths like "mcp.servers" which creates nested structure
pub fn set_nested_value(config: &mut Value, path: &str, value: Value) {
    if path.is_empty() {
        return;
    }

    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() == 1 {
        // Simple case: single key
        config[path] = value;
        return;
    }

    // Navigate/create nested structure
    let mut current = config;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part: set the value
            current[part] = value;
            break;
        } else {
            // Intermediate part: ensure object exists
            if !current[part].is_object() {
                current[part] = serde_json::json!({});
            }
            current = &mut current[part];
        }
    }
}

/// Helper function to get a value from a nested path in a JSON object
/// Supports paths like "mcp.servers"
pub fn get_nested_value<'a>(config: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return Some(config);
    }

    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() == 1 {
        // Simple case: single key
        return config.get(path);
    }

    // Navigate nested structure
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
    fn test_set_nested_value_simple() {
        let mut config = json!({});
        set_nested_value(&mut config, "key", json!("value"));
        assert_eq!(config["key"], "value");
    }

    #[test]
    fn test_set_nested_value_nested() {
        let mut config = json!({});
        set_nested_value(&mut config, "mcp.servers", json!({"server1": {}}));
        assert_eq!(config["mcp"]["servers"]["server1"], json!({}));
    }

    #[test]
    fn test_get_nested_value_simple() {
        let config = json!({"key": "value"});
        assert_eq!(get_nested_value(&config, "key"), Some(&json!("value")));
    }

    #[test]
    fn test_get_nested_value_nested() {
        let config = json!({"mcp": {"servers": {"server1": {}}}});
        assert_eq!(get_nested_value(&config, "mcp.servers"), Some(&json!({"server1": {}})));
    }

    #[test]
    fn test_get_nested_value_missing() {
        let config = json!({"key": "value"});
        assert_eq!(get_nested_value(&config, "missing"), None);
        assert_eq!(get_nested_value(&config, "mcp.servers"), None);
    }

    #[test]
    fn test_empty_path() {
        let config = json!({"key": "value"});
        let mut config_mut = config.clone();

        assert_eq!(get_nested_value(&config, ""), Some(&config));
        set_nested_value(&mut config_mut, "", json!("ignored"));
        assert_eq!(config_mut, config); // Should remain unchanged
    }
}
