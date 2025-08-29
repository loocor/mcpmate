// Database operations for client handlers

use crate::api::models::client::{ClientDetectedApp, ClientInfo, ClientRow};
use crate::common::{ConfigChecker, json::strip_comments};
use crate::config::client::models::ClientConfigType;
use crate::system::detection::detector::AppDetector;
use anyhow::Result;
use std::sync::Arc;

/// Helper function to get all client from database
pub async fn get_all_client(db_pool: &sqlx::SqlitePool) -> Result<Vec<ClientRow>, sqlx::Error> {
    sqlx::query_as::<_, ClientRow>(
        r#"
        SELECT id, identifier, display_name, description, logo_url, category, enabled, detected,
               last_detected_at, install_path, config_path, version, detection_method,
               config_mode, created_at, updated_at
        FROM client
        ORDER BY display_name
        "#,
    )
    .fetch_all(db_pool)
    .await
}

/// Helper function to get supported transports for a client from database
pub async fn get_supported_transports(
    client_id: &str,
    db_pool: &sqlx::SqlitePool,
) -> Vec<String> {
    // Query the database for supported transports (stored as JSON)
    let query = "
        SELECT supported_transports
        FROM client_config_rules
        WHERE client_id = (
            SELECT id FROM client WHERE identifier = ?
        )
        LIMIT 1
    ";

    match sqlx::query_scalar::<_, String>(query)
        .bind(client_id)
        .fetch_optional(db_pool)
        .await
    {
        Ok(Some(json_str)) => {
            // Parse JSON array of transports
            match serde_json::from_str::<Vec<String>>(&json_str) {
                Ok(transports) => transports,
                Err(_) => vec!["stdio".to_string()],
            }
        }
        _ => vec!["stdio".to_string()],
    }
}

/// Get current platform identifier
fn get_current_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux" // Default fallback
    }
}

/// Extract runtimes from JSON array value
fn extract_runtimes_from_array(array: &serde_json::Value) -> Option<Vec<String>> {
    array
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
}

/// Helper function to get supported runtimes for a client from database
pub async fn get_supported_runtimes(
    client_id: &str,
    db_pool: &sqlx::SqlitePool,
) -> Vec<String> {
    const DEFAULT_RUNTIME: &str = "npx";

    // Query the database for supported runtimes (stored as JSON by platform)
    let query = "
        SELECT supported_runtimes
        FROM client_config_rules
        WHERE client_id = (
            SELECT id FROM client WHERE identifier = ?
        )
        LIMIT 1
    ";

    // Early return if database query fails
    let json_str = match sqlx::query_scalar::<_, String>(query)
        .bind(client_id)
        .fetch_optional(db_pool)
        .await
    {
        Ok(Some(json_str)) => json_str,
        _ => return vec![DEFAULT_RUNTIME.to_string()],
    };

    // Early return if JSON parsing fails
    let platforms = match serde_json::from_str::<serde_json::Value>(&json_str) {
        Ok(platforms) => platforms,
        Err(_) => return vec![DEFAULT_RUNTIME.to_string()],
    };

    let current_platform = get_current_platform();

    // Try current platform first
    if let Some(platform_runtimes) = platforms.get(current_platform) {
        if let Some(runtimes) = extract_runtimes_from_array(platform_runtimes) {
            if !runtimes.is_empty() {
                return runtimes;
            }
        }
    }

    // Fallback: try any platform's runtimes
    if let Some(platforms_obj) = platforms.as_object() {
        for (_, platform_runtimes) in platforms_obj {
            if let Some(runtimes) = extract_runtimes_from_array(platform_runtimes) {
                if !runtimes.is_empty() {
                    return runtimes;
                }
            }
        }
    }

    // Final fallback
    vec![DEFAULT_RUNTIME.to_string()]
}

/// Helper function to get the config type for a client from database
pub async fn get_config_type(
    client_id: &str,
    db_pool: &sqlx::SqlitePool,
) -> Option<ClientConfigType> {
    // Query the database for config_type
    let query = "
        SELECT config_type
        FROM client_config_rules
        WHERE client_id = (
            SELECT id FROM client WHERE identifier = ?
        )
        LIMIT 1
    ";

    match sqlx::query_scalar::<_, String>(query)
        .bind(client_id)
        .fetch_optional(db_pool)
        .await
    {
        Ok(Some(config_type_str)) => {
            match config_type_str.as_str() {
                "mixed" => Some(ClientConfigType::Mixed),
                "array" => Some(ClientConfigType::Array),
                "standard" => Some(ClientConfigType::Standard),
                _ => Some(ClientConfigType::Standard), // Default fallback
            }
        }
        _ => None,
    }
}

/// Helper function to get the actual config path for a client using unified path service
pub async fn get_client_config_path(
    client_id: &str,
    db_pool: &sqlx::SqlitePool,
) -> String {
    // Use the unified path service
    let path_service = crate::system::paths::service::get_path_service();

    match path_service.get_client_config_path(db_pool, client_id).await {
        Ok(path) => path,
        Err(e) => {
            tracing::warn!("Failed to get client config path for '{}': {}", client_id, e);
            format!("~/.config/{}/config.json", client_id)
        }
    }
}

/// Perform detection for a specific client
pub async fn perform_client_detection(
    client_id: &str,
    db_pool: &sqlx::SqlitePool,
) -> Result<Option<ClientDetectedApp>, anyhow::Error> {
    // Create app detector
    let detector = AppDetector::new(Arc::new(db_pool.clone())).await?;

    // Detect the specific client
    match detector.detect_by_identifier(client_id).await? {
        Some(detected_app) => Ok(Some(ClientDetectedApp {
            install_path: detected_app.install_path,
            config_path: detected_app.config_path,
        })),
        None => Ok(None),
    }
}

/// Update client detection status in database
pub async fn update_client_detection_status(
    client_id: &str,
    detected: bool,
    install_path: Option<&str>,
    db_pool: &sqlx::SqlitePool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE client
        SET detected = ?,
            install_path = ?,
            last_detected_at = CURRENT_TIMESTAMP,
            updated_at = CURRENT_TIMESTAMP
        WHERE identifier = ?
        "#,
    )
    .bind(detected)
    .bind(install_path)
    .bind(client_id)
    .execute(db_pool)
    .await?;

    Ok(())
}

/// Build ClientInfo from database row and optional detected app data
pub async fn build_client_info(
    client: &ClientRow,
    detected_app: Option<&ClientDetectedApp>,
    db_pool: &sqlx::SqlitePool,
) -> ClientInfo {
    let client_id = &client.identifier;
    let category = client.get_category();

    // Get supported transports, runtimes, and config type from database
    let supported_transports = get_supported_transports(client_id, db_pool).await;
    let supported_runtimes = get_supported_runtimes(client_id, db_pool).await;
    let config_type = get_config_type(client_id, db_pool).await;

    // Determine detection status and paths
    let (detected, install_path, config_path_buf) = if let Some(detected_app) = detected_app {
        (
            true,
            Some(detected_app.install_path.to_string_lossy().to_string()),
            detected_app.config_path.clone(),
        )
    } else {
        // Use database values or get config path from detection rules
        let config_path = get_client_config_path(client_id, db_pool).await;
        (
            client.detected,
            client.install_path.clone(),
            std::path::PathBuf::from(config_path),
        )
    };

    // Unified config checks
    let config_path = config_path_buf.to_string_lossy().to_string();
    let config_exists = config_path_buf.exists();
    let checker = ConfigChecker::new();
    let has_mcp_config = checker.check_mcp_config_exists(&config_path_buf).await;

    ClientInfo {
        identifier: client.identifier.clone(),
        display_name: client.display_name.clone(),
        logo_url: client.logo_url.clone(),
        category,
        enabled: client.enabled,
        detected,
        install_path,
        config_path,
        config_exists,
        has_mcp_config,
        supported_transports,
        supported_runtimes,
        config_mode: client.config_mode.clone(),
        config_type,
        last_detected_at: client.last_detected_at.map(|dt| dt.to_rfc3339()),
        last_modified: None,
        mcp_servers_count: None,
    }
}

/// Parse JSON content with fallback for JSONC (JSON with comments)
pub fn parse_json_resilient(content: &str) -> serde_json::Value {
    if content.is_empty() {
        return serde_json::Value::Object(serde_json::Map::new());
    }

    // Try to parse as standard JSON first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
        return json;
    }

    // If standard JSON parsing fails, try to strip JSONC comments and parse again
    let cleaned_content = strip_comments(content);
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&cleaned_content) {
        tracing::debug!("Successfully parsed JSONC content after stripping comments");
        return json;
    }

    // If both attempts fail, wrap the raw content as a string value
    tracing::warn!("Failed to parse content as JSON/JSONC. Using raw string as value.");
    serde_json::Value::String(content.to_string())
}
