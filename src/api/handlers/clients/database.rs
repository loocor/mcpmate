// Database operations for client handlers

use super::models::{ClientAppRow, SimpleDetectedApp};
use crate::system::detection::detector::AppDetector;
use anyhow::Result;
use std::sync::Arc;

/// Helper function to get all client apps from database
pub async fn get_all_client_apps(
    db_pool: &sqlx::SqlitePool
) -> Result<Vec<ClientAppRow>, sqlx::Error> {
    sqlx::query_as::<_, ClientAppRow>(
        r#"
        SELECT id, identifier, display_name, description, enabled, detected,
               last_detected_at, install_path, config_path, version, detection_method,
               created_at, updated_at
        FROM client_apps
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
        WHERE client_app_id = (
            SELECT id FROM client_apps WHERE identifier = ?
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

/// Helper function to get supported runtimes for a client from database
pub async fn get_supported_runtimes(
    client_id: &str,
    db_pool: &sqlx::SqlitePool,
) -> Vec<String> {
    // Query the database for supported runtimes (stored as JSON by platform)
    let query = "
        SELECT supported_runtimes
        FROM client_config_rules
        WHERE client_app_id = (
            SELECT id FROM client_apps WHERE identifier = ?
        )
        LIMIT 1
    ";

    match sqlx::query_scalar::<_, String>(query)
        .bind(client_id)
        .fetch_optional(db_pool)
        .await
    {
        Ok(Some(json_str)) => {
            // Parse JSON object with platform-specific runtimes
            match serde_json::from_str::<serde_json::Value>(&json_str) {
                Ok(platforms) => {
                    // Get current platform
                    let current_platform = if cfg!(target_os = "macos") {
                        "macos"
                    } else if cfg!(target_os = "linux") {
                        "linux"
                    } else if cfg!(target_os = "windows") {
                        "windows"
                    } else {
                        "linux"
                    };

                    // Extract runtimes for current platform
                    if let Some(platform_runtimes) = platforms.get(current_platform) {
                        if let Some(runtimes_array) = platform_runtimes.as_array() {
                            return runtimes_array
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                        }
                    }

                    // Fallback: try to get any platform's runtimes
                    for (_, platform_runtimes) in
                        platforms.as_object().unwrap_or(&serde_json::Map::new())
                    {
                        if let Some(runtimes_array) = platform_runtimes.as_array() {
                            return runtimes_array
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();
                        }
                    }

                    vec!["npx".to_string()]
                }
                Err(_) => vec!["npx".to_string()],
            }
        }
        _ => vec!["npx".to_string()],
    }
}

/// Helper function to get the actual config path for a client
pub async fn get_client_config_path(
    client_id: &str,
    db_pool: &sqlx::SqlitePool,
) -> String {
    // Query the database for the config path from detection rules
    let query = "
        SELECT config_path
        FROM client_detection_rules
        WHERE client_app_id = (
            SELECT id FROM client_apps WHERE identifier = ?
        )
        LIMIT 1
    ";

    match sqlx::query_scalar::<_, String>(query)
        .bind(client_id)
        .fetch_optional(db_pool)
        .await
    {
        Ok(Some(config_path)) => {
            // Use PathMapper to resolve template variables consistently
            use crate::system::paths::PathMapper;

            let path_mapper = PathMapper::new().unwrap_or_default();
            match path_mapper.resolve_template(&config_path) {
                Ok(resolved_path) => resolved_path.to_string_lossy().to_string(),
                Err(_) => {
                    // Fallback: try old {HOME} format
                    config_path.replace("{HOME}", &std::env::var("HOME").unwrap_or_default())
                }
            }
        }
        _ => format!("~/.config/{}/config.json", client_id),
    }
}

/// Perform detection for a specific client
pub async fn perform_client_detection(
    client_id: &str,
    db_pool: &sqlx::SqlitePool,
) -> Result<Option<SimpleDetectedApp>, anyhow::Error> {
    // Create app detector
    let detector = AppDetector::new(Arc::new(db_pool.clone())).await?;

    // Detect the specific client
    match detector.detect_by_identifier(client_id).await? {
        Some(detected_app) => Ok(Some(SimpleDetectedApp {
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
        UPDATE client_apps
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
