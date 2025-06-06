// HTTP handlers for client management API

use super::config::{analyze_config_content, check_mcp_config_exists, get_config_last_modified};
use super::database::{
    get_all_client_apps, get_client_config_path, get_supported_runtimes, get_supported_transports,
    perform_client_detection, update_client_detection_status,
};

use super::import::import_servers_from_config;
use super::models::ClientsQuery;
use crate::api::models::clients::*;
use crate::api::routes::AppState;
use crate::common::json::strip_comments;
use crate::config::client::manager::ClientManager;
use crate::config::client::models::{ApplicationRequest, GenerationMode, GenerationRequest};
use crate::config::suit::basic::get_active_config_suits;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

/// Macro to extract database pool from app state with early return on error
macro_rules! get_db_pool {
    ($app_state:expr) => {
        match &$app_state.database {
            Some(db) => db.pool.clone(),
            None => return Err(StatusCode::SERVICE_UNAVAILABLE),
        }
    };
}

/// Handler for GET /api/clients
/// Detects and returns all clients, with optional force refresh
pub async fn get_clients(
    Query(params): Query<ClientsQuery>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<ClientInfo>>>, StatusCode> {
    let db_pool = get_db_pool!(app_state);

    // Get all client apps from database (not just enabled ones)
    let mut all_clients = match get_all_client_apps(&db_pool).await {
        Ok(clients) => clients,
        Err(e) => {
            tracing::error!("Failed to get client apps from database: {e}");
            return Ok(Json(ApiResponse::error(
                "DATABASE_ERROR",
                &format!("Failed to get client apps: {e}"),
            )));
        }
    };

    // If force_refresh is requested, perform actual detection
    let mut detected_apps_map = std::collections::HashMap::new();
    if params.force_refresh {
        tracing::info!(
            "Force refresh requested, performing detection for {} clients",
            all_clients.len()
        );

        // Perform detection for all clients in the database
        for client in &all_clients {
            tracing::debug!("Attempting to detect client: {}", client.identifier);

            match perform_client_detection(&client.identifier, &db_pool).await {
                Ok(Some(detected_app)) => {
                    tracing::info!(
                        "Successfully detected client: {} at {}",
                        client.identifier,
                        detected_app.install_path.display()
                    );

                    // Update database with detection results
                    if let Err(e) = update_client_detection_status(
                        &client.identifier,
                        true,
                        Some(&detected_app.install_path.to_string_lossy()),
                        &db_pool,
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to update detection status for {}: {}",
                            client.identifier,
                            e
                        );
                    }

                    detected_apps_map.insert(client.identifier.clone(), detected_app);
                }
                Ok(None) => {
                    tracing::debug!("Client not detected: {}", client.identifier);

                    // Update database to mark as not detected
                    if let Err(e) =
                        update_client_detection_status(&client.identifier, false, None, &db_pool)
                            .await
                    {
                        tracing::warn!(
                            "Failed to update detection status for {}: {}",
                            client.identifier,
                            e
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to detect client {}: {}", client.identifier, e);
                }
            }
        }

        tracing::info!(
            "Detection completed. Found {} detected clients",
            detected_apps_map.len()
        );

        // Re-fetch client data from database to get updated detection status
        all_clients = match get_all_client_apps(&db_pool).await {
            Ok(clients) => clients,
            Err(e) => {
                tracing::error!("Failed to re-fetch client apps after detection: {e}");
                return Ok(Json(ApiResponse::error(
                    "DATABASE_ERROR",
                    "Failed to retrieve updated client information",
                )));
            }
        };
    }

    // Convert all client apps to ClientInfo
    let mut client_infos = Vec::new();
    for client in all_clients {
        let client_id = &client.identifier;

        // Get supported transports and runtimes from database
        let supported_transports = get_supported_transports(client_id, &db_pool).await;
        let supported_runtimes = get_supported_runtimes(client_id, &db_pool).await;

        // Check if this client was detected (if force_refresh was used)
        let (detected, install_path, config_path, config_exists, has_mcp_config) =
            if let Some(detected_app) = detected_apps_map.get(client_id) {
                (
                    true,
                    Some(detected_app.install_path.to_string_lossy().to_string()),
                    detected_app.config_path.to_string_lossy().to_string(),
                    detected_app.config_path.exists(),
                    check_mcp_config_exists(&detected_app.config_path, client_id, &db_pool).await,
                )
            } else {
                // Use database values or get config path from detection rules
                let config_path = get_client_config_path(client_id, &db_pool).await;
                let config_path_buf = std::path::PathBuf::from(&config_path);
                (
                    client.detected,
                    client.install_path,
                    config_path,
                    config_path_buf.exists(),
                    check_mcp_config_exists(&config_path_buf, client_id, &db_pool).await,
                )
            };

        client_infos.push(ClientInfo {
            identifier: client.identifier,
            display_name: client.display_name,
            detected,
            install_path,
            config_path,
            config_exists,
            has_mcp_config,
            supported_transports,
            supported_runtimes,
            last_detected_at: client.last_detected_at.map(|dt| dt.to_rfc3339()),
        });
    }

    Ok(Json(ApiResponse::success(client_infos)))
}

/// Handler for GET /api/clients/{client_identifier}/config
/// Returns current configuration content
pub async fn get_config(
    Path(client_identifier): Path<String>,
    Query(query): Query<std::collections::HashMap<String, String>>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<ConfigViewResponse>>, StatusCode> {
    let db_pool = get_db_pool!(app_state);
    let mut client_manager = ClientManager::new(Arc::new(db_pool.clone()));

    // Get current configuration
    match client_manager.get_current_config(&client_identifier).await {
        Ok(content) => {
            // Get actual config path from client manager
            let config_path = get_client_config_path(&client_identifier, &db_pool).await;
            let config_exists = !content.is_empty();

            // Parse and analyze the configuration first (while we still have content)
            let (has_mcp_config, mcp_servers_count) =
                analyze_config_content(&content, &client_identifier, &db_pool).await;

            // Parse configuration content to JSON object
            let json_content = if content.is_empty() {
                serde_json::Value::Object(serde_json::Map::new())
            } else {
                // Try to parse as standard JSON first
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => json,
                    Err(_) => {
                        // If standard JSON parsing fails, try to strip JSONC comments and parse again
                        let cleaned_content = strip_comments(&content);
                        match serde_json::from_str::<serde_json::Value>(&cleaned_content) {
                            Ok(json) => {
                                tracing::debug!(
                                    "Successfully parsed JSONC content for {}",
                                    client_identifier
                                );
                                json
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to parse config content as JSON/JSONC for {}: {}. Using raw string as value.",
                                    client_identifier,
                                    e
                                );
                                // If both attempts fail, wrap the raw content as a string value
                                serde_json::Value::String(content)
                            }
                        }
                    }
                }
            };

            // Get file modification time
            let last_modified = get_config_last_modified(&config_path);

            // Check if import is requested and client is in transparent mode
            let imported_servers = if query.get("import").map(|v| v == "true").unwrap_or(false) {
                // Check if client is in transparent mode (or has no config_mode set)
                let is_transparent = sqlx::query_scalar::<_, bool>(
                    "SELECT (config_mode = 'transparent' OR config_mode IS NULL) FROM client_apps WHERE identifier = ?"
                )
                .bind(&client_identifier)
                .fetch_optional(&db_pool)
                .await
                .unwrap_or(Some(true)) // Default to true if client not found
                .unwrap_or(true);

                if is_transparent {
                    match import_servers_from_config(&json_content, &db_pool).await {
                        Ok(servers) => Some(servers),
                        Err(e) => {
                            tracing::warn!("Failed to import servers from config: {}", e);
                            None
                        }
                    }
                } else {
                    tracing::info!(
                        "Skipping import for client {} in hosted mode",
                        client_identifier
                    );
                    None
                }
            } else {
                None
            };

            let response = ConfigViewResponse {
                config_path,
                config_exists,
                content: json_content,
                has_mcp_config,
                mcp_servers_count,
                last_modified,
                imported_servers,
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Failed to get config for {}: {}", client_identifier, e);
            Ok(Json(ApiResponse::error(
                "CONFIG_READ_FAILED",
                &format!("Failed to read configuration: {e}"),
            )))
        }
    }
}

/// Handler for POST /api/clients/{client_identifier}/config
/// Generates and optionally applies configuration
pub async fn manage_config(
    Path(client_identifier): Path<String>,
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ConfigRequest>,
) -> Result<Json<ApiResponse<ConfigResponse>>, StatusCode> {
    let db_pool = get_db_pool!(app_state);
    let mut client_manager = ClientManager::new(Arc::new(db_pool.clone()));

    // Convert API request to internal request format
    let config_suit_id = match &request.selected_config {
        SelectedConfig::Suit { config_suit_id } => Some(config_suit_id.clone()),
        SelectedConfig::Default => {
            // For Default mode, get the currently active config suit ID
            match get_active_config_suits(&db_pool).await {
                Ok(active_suits) => {
                    if let Some(suit) = active_suits.first() {
                        suit.id.clone()
                    } else {
                        tracing::warn!("No active config suits found for default mode");
                        None
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get active config suits: {}", e);
                    None
                }
            }
        }
        _ => None,
    };

    let generation_request = GenerationRequest {
        client_identifier: client_identifier.clone(),
        mode: match request.mode {
            ConfigMode::Hosted => GenerationMode::Hosted,
            ConfigMode::Transparent => GenerationMode::Transparent,
        },
        config_suit_id,
        servers: match &request.selected_config {
            SelectedConfig::Servers { server_ids } => {
                // For hosted mode, ignore servers parameter
                if matches!(request.mode, ConfigMode::Hosted) {
                    None
                } else {
                    Some(server_ids.clone())
                }
            }
            _ => None,
        },
    };

    // Generate configuration
    let generated_config = match client_manager.generate_config(&generation_request).await {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("Failed to generate config for {}: {}", client_identifier, e);
            return Ok(Json(ApiResponse::error(
                "GENERATION_FAILED",
                &format!("Failed to generate configuration: {}", e),
            )));
        }
    };

    // Parse the generated config content to JSON
    let preview_json =
        match serde_json::from_str::<serde_json::Value>(&generated_config.config_content) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to parse generated config as JSON: {}", e);
                return Ok(Json(ApiResponse::error(
                    "CONFIG_PARSE_FAILED",
                    &format!("Failed to parse generated configuration as JSON: {}", e),
                )));
            }
        };

    let mut response = ConfigResponse {
        success: true,
        preview: preview_json,
        applied: false,
        backup_path: None,
        warnings: vec![],
    };

    // Apply configuration if not preview-only
    if !request.preview_only {
        let application_request = ApplicationRequest {
            client_identifier: client_identifier.clone(),
            config: generated_config,
            create_backup: true,
            dry_run: false,
        };

        match client_manager.apply_config(&application_request).await {
            Ok(result) => {
                response.applied = result.success;
                response.backup_path = result.backup_path;

                // If application succeeded, update the client's config_mode
                if result.success {
                    let config_mode = match request.mode {
                        ConfigMode::Transparent => "transparent",
                        ConfigMode::Hosted => "hosted",
                    };

                    if let Err(e) = client_manager
                        .update_client_config_mode(&client_identifier, config_mode)
                        .await
                    {
                        tracing::warn!(
                            "Failed to update config_mode for client {}: {}",
                            client_identifier,
                            e
                        );
                    }
                }

                // If application failed, add error message to warnings
                if !result.success {
                    response
                        .warnings
                        .push(result.error_message.unwrap_or_default());
                }
            }
            Err(e) => {
                tracing::error!("Failed to apply config for {}: {}", client_identifier, e);
                return Ok(Json(ApiResponse::error(
                    "APPLICATION_FAILED",
                    &format!("Failed to apply configuration: {}", e),
                )));
            }
        }
    }

    Ok(Json(ApiResponse::success(response)))
}
