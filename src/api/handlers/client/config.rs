// Configuration file processing for client handlers

use crate::common::ConfigChecker;
use crate::common::constants::config_keys;
use crate::config::client::utils::get_nested_value;
use sqlx::Row;

/// Helper function to check if a config file contains MCP configuration
/// Now supports client-specific top-level keys
pub async fn check_mcp_config_exists(
    config_path: &std::path::Path,
    client_identifier: &str,
    db_pool: &sqlx::SqlitePool,
) -> bool {
    // Use the unified configuration checker for basic checks
    let checker = ConfigChecker::new();
    if !checker.check_mcp_config_exists(config_path).await {
        return false;
    }

    // If basic checks pass, perform more detailed client-specific checks
    match std::fs::read_to_string(config_path) {
        Ok(content) => {
            let (has_mcp, _) = analyze_config_content(&content, client_identifier, db_pool).await;
            has_mcp
        }
        Err(_) => false,
    }
}

/// Helper function to analyze config content for MCP information
pub async fn analyze_config_content(
    content: &str,
    client_identifier: &str,
    db_pool: &sqlx::SqlitePool,
) -> (bool, u32) {
    if content.is_empty() {
        return (false, 0);
    }

    // Get the client configuration details
    let client_config = match get_client_config_details(client_identifier, db_pool).await {
        Ok((top_level_key, is_array_config)) => (top_level_key, is_array_config),
        Err(_) => {
            return analyze_with_fallback_keys(content);
        }
    };

    let (top_level_key, is_array_config) = client_config;

    // Try to parse as JSON
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(json) => {
            // Check if this is an array configuration (Augment style)
            if is_array_config {
                // For array config (like Augment), the JSON itself should be an array
                if let Some(arr) = json.as_array() {
                    // Consider it as MCP config if it's a non-empty array with objects that have common MCP fields
                    let has_mcp_items = !arr.is_empty()
                        && arr.iter().any(|item| {
                            item.get("name").is_some() && (item.get("command").is_some() || item.get("url").is_some())
                        });

                    return (has_mcp_items, arr.len() as u32);
                }

                return (false, 0);
            }

            // For object configs with a top-level key (supports nested paths like "mcp.servers")
            if let Some(servers) = get_nested_value(&json, &top_level_key) {
                if let Some(obj) = servers.as_object() {
                    return (true, obj.len() as u32);
                } else {
                    return (true, 0);
                }
            }

            (false, 0)
        }
        Err(_) => {
            // If not valid JSON, do simple text search

            // Early return for array configs
            if is_array_config {
                let has_mcp =
                    content.contains("[{") && (content.contains("\"command\"") || content.contains("\"url\""));
                return (has_mcp, 0);
            }

            // Early return if no top-level key
            if top_level_key.is_empty() {
                return (false, 0);
            }

            // Handle object configs with top-level key search
            let search_key = if top_level_key.contains('.') {
                // For nested paths like "mcp.servers", search for the last part
                top_level_key.split('.').next_back().unwrap_or(&top_level_key)
            } else {
                &top_level_key
            };

            let has_mcp = content.contains(search_key);
            (has_mcp, 0)
        }
    }
}

/// Fallback analysis when database lookup fails
/// Checks common top-level keys for compatibility
fn analyze_with_fallback_keys(content: &str) -> (bool, u32) {
    // Try to parse as JSON
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(json) => {
            // Check for MCP servers in various formats (fallback)
            let mcp_servers = json
                .get("mcpServers")
                .or_else(|| json.get("mcp_servers"))
                .or_else(|| json.get("context_servers"))
                .or_else(|| get_nested_value(&json, "mcp.servers"));

            if let Some(servers) = mcp_servers {
                if let Some(obj) = servers.as_object() {
                    (true, obj.len() as u32)
                } else {
                    (true, 0)
                }
            } else {
                (false, 0)
            }
        }
        Err(_) => {
            // If not valid JSON, do simple text search with fallback keys
            let has_mcp = content.contains(config_keys::MCP_SERVERS)
                || content.contains(config_keys::MCP_SERVERS_SNAKE)
                || content.contains(config_keys::CONTEXT_SERVERS)
                || content.contains("\"mcp\"") && content.contains("\"servers\"");
            (has_mcp, 0)
        }
    }
}

/// Get the configuration details for a specific client from database
async fn get_client_config_details(
    client_identifier: &str,
    db_pool: &sqlx::SqlitePool,
) -> Result<(String, bool), sqlx::Error> {
    let row = sqlx::query("SELECT top_level_key, config_type FROM client_config_rules WHERE client_identifier = ?")
        .bind(client_identifier)
        .fetch_one(db_pool)
        .await?;

    let top_level_key: String = row.get("top_level_key");
    let config_type: String = row.get("config_type");

    // Check if config_type is 'array' (default to 'standard' if empty)
    let config_type = if config_type.is_empty() {
        "standard"
    } else {
        &config_type
    };
    let is_array_config = config_type == "array";

    Ok((top_level_key, is_array_config))
}

/// Helper function to get file modification time
pub fn get_config_last_modified(config_path: &str) -> Option<String> {
    use std::fs;
    use std::time::SystemTime;

    // Expand tilde in path
    let expanded_path = if config_path.starts_with("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        config_path.replacen("~", &home, 1)
    } else {
        config_path.to_string()
    };

    match fs::metadata(&expanded_path) {
        Ok(metadata) => {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                    // Convert to RFC3339 format
                    let datetime = chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)?;
                    Some(datetime.to_rfc3339())
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(_) => None,
    }
}
