// Configuration file processing for client handlers

use crate::clients::models::ClientTemplate;
use crate::common::ConfigChecker;
// use crate::common::constants::config_keys; // unused after analyzer refactor
use crate::clients::analyzer::analyze_config_content as analyze;

/// Helper function to analyze config content for MCP information
pub fn analyze_config_content(content: &str, _client_identifier: &str, template: &ClientTemplate) -> (bool, u32) { analyze(content, template) }

/// Fallback analysis when database lookup fails
/// Checks common top-level keys for compatibility

/// Helper function to check if a config file contains MCP configuration
/// Now supports client-specific top-level keys
pub async fn check_mcp_config_exists(
    config_path: &std::path::Path,
    client_identifier: &str,
    template: &ClientTemplate,
) -> bool {
    // Use the unified configuration checker for basic checks
    let checker = ConfigChecker::new();
    if !checker.check_mcp_config_exists(config_path).await {
        return false;
    }

    // If basic checks pass, perform more detailed client-specific checks
    match std::fs::read_to_string(config_path) {
        Ok(content) => analyze_config_content(&content, client_identifier, template).0,
        Err(_) => false,
    }
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
