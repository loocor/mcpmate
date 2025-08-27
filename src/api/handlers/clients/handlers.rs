// HTTP handlers for client management API

use super::config::{analyze_config_content, get_config_last_modified};
use super::database::{
    build_client_info, get_all_client_apps, get_client_config_path, get_config_type, parse_json_resilient,
    perform_client_detection, update_client_detection_status,
};

use super::import::import_servers_from_config;
use crate::api::models::clients::{
    ClientConfigData, ClientConfigMode, ClientConfigReq, ClientConfigResp, ClientConfigSelected,
    ClientConfigUpdateData, ClientConfigUpdateReq, ClientConfigUpdateResp, ClientsCheckData, ClientsCheckReq,
    ClientsCheckResp,
};
use crate::api::routes::AppState;

use crate::config::client::manager::ClientManager;
use crate::config::client::models::{ApplicationRequest, GenerationMode, GenerationRequest};
use crate::config::suit::basic::get_active_config_suits;
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
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
pub async fn list(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ClientsCheckReq>,
) -> Result<Json<ClientsCheckResp>, StatusCode> {
    let db_pool = get_db_pool!(app_state);

    let result = clients_check_core(&request, &db_pool).await?;

    Ok(Json(result))
}

/// Handler for GET /api/clients/{identifier}
/// Returns current configuration content
pub async fn details(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ClientConfigReq>,
) -> Result<Json<ClientConfigResp>, StatusCode> {
    let db_pool = get_db_pool!(app_state);

    let result = client_config_details_core(&request, &db_pool).await?;

    Ok(Json(result))
}

/// Handler for POST /api/clients/{identifier}
/// Generates and optionally applies configuration
pub async fn update(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientConfigUpdateReq>,
) -> Result<Json<ClientConfigUpdateResp>, StatusCode> {
    let db_pool = get_db_pool!(app_state);

    let result = client_config_update_core(&request, &db_pool).await?;

    Ok(Json(result))
}

// ==================== Core Business Functions ====================

/// Core business logic for clients check operation
async fn clients_check_core(
    request: &ClientsCheckReq,
    db_pool: &sqlx::SqlitePool,
) -> Result<ClientsCheckResp, StatusCode> {
    // Get all client apps from database (not just enabled ones)
    let mut all_clients = match get_all_client_apps(db_pool).await {
        Ok(clients) => clients,
        Err(e) => {
            tracing::error!("Failed to get client apps from database: {e}");
            return Ok(ClientsCheckResp::error_simple(
                "DATABASE_ERROR",
                &format!("Failed to get client apps: {e}"),
            ));
        }
    };

    // If refresh is requested, perform actual detection
    let mut detected_apps_map = std::collections::HashMap::new();
    if request.refresh {
        tracing::info!(
            "Force refresh requested, performing detection for {} clients",
            all_clients.len()
        );

        // Perform detection for all clients in the database
        for client in &all_clients {
            tracing::debug!("Attempting to detect client: {}", client.identifier);

            match perform_client_detection(&client.identifier, db_pool).await {
                Ok(Some(detected_app)) => {
                    tracing::info!(
                        "Successfully detected client: {} at {}",
                        client.identifier,
                        detected_app.install_path.display()
                    );

                    // Determine if this is a real application installation path or just a config file
                    let install_path_str = detected_app.install_path.to_string_lossy();
                    let is_real_app_path = install_path_str.contains("/Applications/")
                        || install_path_str.ends_with(".app")
                        || install_path_str.ends_with(".exe")
                        || (!install_path_str.contains(".json")
                            && !install_path_str.contains(".config")
                            && !install_path_str.contains("settings")
                            && !install_path_str.contains("globalStorage")
                            && !install_path_str.contains("Application Support")
                            && !install_path_str.contains("AppData"));

                    // Only update install_path if it's a real application path
                    let install_path_to_store = if is_real_app_path {
                        Some(install_path_str.as_ref())
                    } else {
                        None
                    };

                    // Update database with detection results
                    if let Err(e) =
                        update_client_detection_status(&client.identifier, true, install_path_to_store, db_pool).await
                    {
                        tracing::warn!("Failed to update detection status for {}: {}", client.identifier, e);
                    }

                    detected_apps_map.insert(client.identifier.clone(), detected_app);
                }
                Ok(None) => {
                    tracing::debug!("Client not detected: {}", client.identifier);

                    // Update database to mark as not detected
                    if let Err(e) = update_client_detection_status(&client.identifier, false, None, db_pool).await {
                        tracing::warn!("Failed to update detection status for {}: {}", client.identifier, e);
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
        all_clients = match get_all_client_apps(db_pool).await {
            Ok(clients) => clients,
            Err(e) => {
                tracing::error!("Failed to re-fetch client apps after detection: {e}");
                return Ok(ClientsCheckResp::error_simple(
                    "DATABASE_ERROR",
                    "Failed to retrieve updated client information",
                ));
            }
        };
    }

    // Convert all client apps to ClientInfo using the shared function
    let mut client_infos = Vec::new();
    for client in all_clients {
        let detected_app = detected_apps_map.get(&client.identifier);
        let info = build_client_info(&client, detected_app, db_pool).await;
        client_infos.push(info);
    }

    let response = ClientsCheckData {
        total: client_infos.len(),
        clients: client_infos,
        last_updated: chrono::Utc::now().to_rfc3339(),
    };

    Ok(ClientsCheckResp::success(response))
}

/// Core business logic for client config details operation
async fn client_config_details_core(
    request: &ClientConfigReq,
    db_pool: &sqlx::SqlitePool,
) -> Result<ClientConfigResp, StatusCode> {
    let identifier = &request.identifier;
    let mut client_manager = ClientManager::new(Arc::new(db_pool.clone()));

    // Get current configuration
    match client_manager.get_current_config(identifier).await {
        Ok(content) => {
            // Get actual config path from client manager
            let config_path = get_client_config_path(identifier, db_pool).await;
            let config_exists = !content.is_empty();

            // Parse and analyze the configuration first (while we still have content)
            let (has_mcp_config, mcp_servers_count) = analyze_config_content(&content, identifier, db_pool).await;

            // Parse configuration content to JSON object using resilient parser
            let json_content = parse_json_resilient(&content);

            // Get file modification time
            let last_modified = get_config_last_modified(&config_path);

            // Get config type for this client
            let config_type = get_config_type(identifier, db_pool).await;

            // Check if import is requested and client is in transparent mode
            let imported_servers = if request.import {
                // Check if client is in transparent mode (or has no config_mode set)
                let is_transparent = sqlx::query_scalar::<_, bool>(
                    "SELECT (config_mode = 'transparent' OR config_mode IS NULL) FROM client_apps WHERE identifier = ?",
                )
                .bind(identifier)
                .fetch_optional(db_pool)
                .await
                .unwrap_or(Some(true)) // Default to true if client not found
                .unwrap_or(true);

                if is_transparent {
                    match import_servers_from_config(&json_content, db_pool).await {
                        Ok(servers) => Some(servers),
                        Err(e) => {
                            tracing::warn!("Failed to import servers from config: {}", e);
                            None
                        }
                    }
                } else {
                    tracing::info!("Skipping import for client {} in hosted mode", identifier);
                    None
                }
            } else {
                None
            };

            let response = ClientConfigData {
                config_path,
                config_exists,
                content: json_content,
                has_mcp_config,
                mcp_servers_count,
                last_modified,
                config_type,
                imported_servers,
            };

            Ok(ClientConfigResp::success(response))
        }
        Err(e) => {
            tracing::error!("Failed to get config for {}: {}", identifier, e);
            Ok(ClientConfigResp::error_simple(
                "CONFIG_READ_FAILED",
                &format!("Failed to read configuration: {e}"),
            ))
        }
    }
}

/// Core business logic for client config update operation
async fn client_config_update_core(
    request: &ClientConfigUpdateReq,
    db_pool: &sqlx::SqlitePool,
) -> Result<ClientConfigUpdateResp, StatusCode> {
    let identifier = &request.identifier;
    let mut client_manager = ClientManager::new(Arc::new(db_pool.clone()));

    // Convert API request to internal request format
    let config_suit_id = match &request.selected_config {
        ClientConfigSelected::Suit { config_suit_id } => Some(config_suit_id.clone()),
        ClientConfigSelected::Default => {
            // For Default mode, get the currently active config suit ID
            match get_active_config_suits(db_pool).await {
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
        identifier: identifier.to_string(),
        mode: match request.mode {
            ClientConfigMode::Hosted => GenerationMode::Hosted,
            ClientConfigMode::Transparent => GenerationMode::Transparent,
        },
        config_suit_id,
        servers: match &request.selected_config {
            ClientConfigSelected::Servers { server_ids } => {
                // For hosted mode, ignore servers parameter
                if matches!(request.mode, ClientConfigMode::Hosted) {
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
            tracing::error!("Failed to generate config for {}: {}", identifier, e);
            return Ok(ClientConfigUpdateResp::error_simple(
                "GENERATION_FAILED",
                &format!("Failed to generate configuration: {}", e),
            ));
        }
    };

    // Parse the generated config content to JSON using resilient parser
    let preview_json = parse_json_resilient(&generated_config.config_content);

    let mut response = ClientConfigUpdateData {
        success: true,
        preview: preview_json,
        applied: false,
        backup_path: None,
        warnings: vec![],
    };

    // Apply configuration if not preview only
    if !request.preview {
        let application_request = ApplicationRequest {
            identifier: identifier.to_string(),
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
                        ClientConfigMode::Transparent => "transparent",
                        ClientConfigMode::Hosted => "hosted",
                    };

                    if let Err(e) = client_manager.update_client_config_mode(identifier, config_mode).await {
                        tracing::warn!("Failed to update config_mode for client {}: {}", identifier, e);
                    }
                }

                // If application failed, add error message to warnings
                if !result.success {
                    response.warnings.push(result.error_message.unwrap_or_default());
                }
            }
            Err(e) => {
                tracing::error!("Failed to apply config for {}: {}", identifier, e);
                return Ok(ClientConfigUpdateResp::error_simple(
                    "APPLICATION_FAILED",
                    &format!("Failed to apply configuration: {}", e),
                ));
            }
        }
    }

    Ok(ClientConfigUpdateResp::success(response))
}
