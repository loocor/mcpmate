// Configuration file processing for client handlers

use crate::clients::analyzer::analyze_config_content;
use crate::clients::models::{ClientConfigFileParse, ContainerType, TemplateFormat};
use crate::common::ConfigChecker;

/// Fallback analysis when database lookup fails
/// Checks common top-level keys for compatibility
/// Helper function to check if a config file contains MCP configuration
/// Now supports client-specific top-level keys
pub async fn check_mcp_config_exists(
    config_path: &std::path::Path,
    container_keys: &[String],
    is_array_container: bool,
) -> bool {
    // Use the unified configuration checker for basic checks
    let checker = ConfigChecker::new();
    if !checker.check_mcp_config_exists(config_path).await {
        return false;
    }

    // If basic checks pass, perform more detailed client-specific checks
    match std::fs::read_to_string(config_path) {
        Ok(content) => {
            let format = match config_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_ascii_lowercase())
            {
                Some(ext) if ext == "json5" => TemplateFormat::Json5,
                Some(ext) if ext == "toml" => TemplateFormat::Toml,
                Some(ext) if ext == "yaml" || ext == "yml" => TemplateFormat::Yaml,
                _ => TemplateFormat::Json,
            };
            let parse_rule = ClientConfigFileParse {
                format,
                container_type: if is_array_container {
                    ContainerType::Array
                } else {
                    ContainerType::ObjectMap
                },
                container_keys: container_keys.to_vec(),
            };
            analyze_config_content(&content, &parse_rule, None).has_mcp_config
        }
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
