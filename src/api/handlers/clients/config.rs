// Configuration file processing for client handlers

/// Helper function to check if a config file contains MCP configuration
/// Now supports client-specific top-level keys
pub async fn check_mcp_config_exists(
    config_path: &std::path::Path,
    client_identifier: &str,
    db_pool: &sqlx::SqlitePool,
) -> bool {
    if !config_path.exists() {
        return false;
    }

    // Try to read and parse the config file
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

    // Get the top-level key for this specific client
    let top_level_key = match get_client_top_level_key(client_identifier, db_pool).await {
        Ok(key) => key,
        Err(_) => {
            return analyze_with_fallback_keys(content);
        }
    };

    // Try to parse as JSON
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(json) => {
            // Check for MCP servers using the client-specific top-level key
            if let Some(servers) = json.get(&top_level_key) {
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
            // If not valid JSON, do simple text search with the specific key
            let has_mcp = content.contains(&top_level_key);
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
                .or_else(|| json.get("context_servers"));

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
            let has_mcp = content.contains("mcpServers")
                || content.contains("mcp_servers")
                || content.contains("context_servers");
            (has_mcp, 0)
        }
    }
}

/// Get the top-level key for a specific client from database
async fn get_client_top_level_key(
    client_identifier: &str,
    db_pool: &sqlx::SqlitePool,
) -> Result<String, sqlx::Error> {
    let top_level_key: String = sqlx::query_scalar(
        "SELECT top_level_key FROM client_config_rules WHERE client_identifier = ?",
    )
    .bind(client_identifier)
    .fetch_one(db_pool)
    .await?;

    Ok(top_level_key)
}

/// Helper function to get file modification time
pub fn get_config_last_modified(config_path: &str) -> Option<String> {
    use std::fs;
    use std::time::SystemTime;

    // Expand tilde in path
    let expanded_path = if config_path.starts_with("~/") {
        config_path.replacen("~", &std::env::var("HOME").unwrap_or_default(), 1)
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
