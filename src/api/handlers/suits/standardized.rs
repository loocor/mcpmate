// Following server module patterns with State + Request → Response signature

use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use std::sync::Arc;

use super::common::*;
use crate::api::models::suits::{
    ComponentAction, ConfigSuitApiResp, ConfigSuitServerResp, ConfigSuitToolResp, CreateConfigSuitReq, DeleteSuitReq,
    SuitAction, SuitBatchManageApiResp, SuitBatchManageReq, SuitBatchManageResp, SuitComponentListReq,
    SuitComponentManageApiResp, SuitComponentManageReq, SuitComponentManageResp, SuitDetailsApiResp, SuitDetailsReq,
    SuitDetailsResp, SuitManageApiResp, SuitManageReq, SuitManageResp, SuitServersListApiResp, SuitServersListResp,
    SuitToolsListApiResp, SuitToolsListResp, SuitsListApiResp, SuitsListReq, SuitsListResp,
};

// ==========================================
// BASIC CRUD OPERATIONS
// ==========================================

/// List all configuration suits with filtering
///
/// **Endpoint:** `GET /mcp/suits/list?filter_type={type}&suit_type={type}&limit={limit}&offset={offset}`
pub async fn suits_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitsListReq>,
) -> Result<Json<SuitsListApiResp>, ApiError> {
    let db = get_database(&state).await?;

    // Apply filters and pagination (simplified for now)
    let suits = crate::config::suit::get_all_config_suits(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suits: {e}")))?;

    // Apply filters
    let filtered_suits: Vec<_> = suits
        .into_iter()
        .filter(|suit| {
            if let Some(filter_type) = &request.filter_type {
                match filter_type.as_str() {
                    "active" => suit.is_active,
                    "inactive" => !suit.is_active,
                    "all" => true,
                    _ => true,
                }
            } else {
                true
            }
        })
        .filter(|suit| {
            if let Some(suit_type) = &request.suit_type {
                suit.suit_type.to_string() == *suit_type
            } else {
                true
            }
        })
        .collect();

    let total = filtered_suits.len();

    // Apply pagination
    let limit = request.limit.unwrap_or(50).min(100);
    let offset = request.offset.unwrap_or(0);
    let paginated_suits: Vec<_> = filtered_suits.into_iter().skip(offset).take(limit).collect();

    let suit_responses = paginated_suits.iter().map(suit_to_response).collect();

    let response = SuitsListResp {
        suits: suit_responses,
        total,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(SuitsListApiResp::success(response)))
}

/// Get details for a specific configuration suit
///
/// **Endpoint:** `GET /mcp/suits/details?id={suit_id}`
pub async fn suit_details(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitDetailsReq>,
) -> Result<Json<SuitDetailsApiResp>, ApiError> {
    let db = get_database(&state).await?;

    // Get the configuration suit
    let suit = crate::config::suit::get_config_suit(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                request.id
            )));
        }
    };

    // Get component counts
    let servers_count = crate::config::suit::get_config_suit_servers(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers count: {e}")))?
        .into_iter()
        .filter(|s| s.enabled)
        .count();

    let tools_count = crate::config::suit::get_config_suit_tools(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tools count: {e}")))?
        .into_iter()
        .filter(|t| t.enabled)
        .count();

    // For now, set resources and prompts counts to 0 (implement later)
    let resources_count = 0;
    let prompts_count = 0;

    let response = SuitDetailsResp {
        suit: suit_to_response(&suit),
        servers_count,
        tools_count,
        resources_count,
        prompts_count,
    };

    Ok(Json(SuitDetailsApiResp::success(response)))
}

/// Delete a configuration suit
///
/// **Endpoint:** `DELETE /mcp/suits/delete`
pub async fn delete_suit(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DeleteSuitReq>,
) -> Result<Json<SuitManageApiResp>, ApiError> {
    let db = get_database(&state).await?;

    // Get the existing suit to check if it exists and get its name
    let existing_suit = crate::config::suit::get_config_suit(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    let suit = match existing_suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                request.id
            )));
        }
    };

    // Check if it's the default suit (prevent deletion)
    if suit.is_default {
        return Err(ApiError::BadRequest(
            "Cannot delete the default configuration suit".to_string(),
        ));
    }

    // Delete the suit (cascade will handle related records)
    crate::config::suit::delete_config_suit(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete configuration suit: {e}")))?;

    let response = SuitManageResp {
        id: request.id,
        name: suit.name,
        result: "deleted".to_string(),
        status: "inactive".to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(SuitManageApiResp::success(response)))
}

/// Create a new configuration suit
///
/// **Endpoint:** `POST /mcp/suits/create`
pub async fn create_suit(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateConfigSuitReq>,
) -> Result<Json<ConfigSuitApiResp>, ApiError> {
    // Call the legacy handler and wrap the response
    match super::crud::create_suit(State(state), Json(request)).await {
        Ok(Json(legacy_response)) => Ok(Json(legacy_response)),
        Err(e) => match e {
            StatusCode::NOT_FOUND => Err(ApiError::NotFound("Configuration suit not found".to_string())),
            StatusCode::BAD_REQUEST => Err(ApiError::BadRequest("Invalid request parameters".to_string())),
            StatusCode::INTERNAL_SERVER_ERROR => Err(ApiError::InternalError("Internal server error".to_string())),
            _ => Err(ApiError::InternalError(format!("Unexpected error: {}", e.as_u16()))),
        },
    }
}

/// Update an existing configuration suit
///
/// **Endpoint:** `POST /mcp/suits/update`
pub async fn update_suit(
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<UpdateConfigSuitReq>,
) -> Result<Json<ConfigSuitApiResp>, ApiError> {
    // For now, we need to extract an ID from the request
    // In a full refactor, we'd add the ID to the UpdateConfigSuitReq
    // For now, return an error asking for the ID to be provided differently
    Err(ApiError::BadRequest(
        "Update operation not yet implemented in standardized form. Use legacy endpoint.".to_string(),
    ))
}

// ==========================================
// SUIT MANAGEMENT OPERATIONS
// ==========================================

/// Manage suit operations (activate/deactivate)
///
/// **Endpoint:** `POST /mcp/suits/manage`
pub async fn manage_suit(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuitManageReq>,
) -> Result<Json<SuitManageApiResp>, ApiError> {
    let db = get_database(&state).await?;

    // Get the existing suit
    let existing_suit = crate::config::suit::get_config_suit(&db.pool, &request.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    let mut suit = match existing_suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                request.id
            )));
        }
    };

    // Perform the action
    let (result, new_status) = match request.action {
        SuitAction::Activate => {
            suit.is_active = true;
            ("activated", "active")
        }
        SuitAction::Deactivate => {
            // Prevent deactivation of default suit
            if suit.is_default {
                return Err(ApiError::BadRequest(
                    "Cannot deactivate the default configuration suit".to_string(),
                ));
            }
            suit.is_active = false;
            ("deactivated", "inactive")
        }
    };

    // Update the suit in database
    crate::config::suit::upsert_config_suit(&db.pool, &suit)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to update configuration suit: {e}")))?;

    let response = SuitManageResp {
        id: request.id,
        name: suit.name,
        result: result.to_string(),
        status: new_status.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(SuitManageApiResp::success(response)))
}

/// Batch manage suit operations
///
/// **Endpoint:** `POST /mcp/suits/manage/batch`
pub async fn manage_suits_batch(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuitBatchManageReq>,
) -> Result<Json<SuitBatchManageApiResp>, ApiError> {
    let _db = get_database(&state).await?;

    let mut successful_ids = Vec::new();
    let mut failed_operations = std::collections::HashMap::new();

    for suit_id in &request.ids {
        let individual_request = SuitManageReq {
            id: suit_id.clone(),
            action: request.action.clone(),
        };

        match manage_suit(State(state.clone()), Json(individual_request)).await {
            Ok(_) => successful_ids.push(suit_id.clone()),
            Err(e) => {
                failed_operations.insert(suit_id.clone(), e.to_string());
            }
        }
    }

    let response = SuitBatchManageResp {
        success_count: successful_ids.len(),
        successful_ids,
        failed_operations,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(SuitBatchManageApiResp::success(response)))
}

// ==========================================
// COMPONENT LIST OPERATIONS
// ==========================================

/// List servers in a configuration suit
///
/// **Endpoint:** `GET /mcp/suits/servers/list?suit_id={suit_id}&enabled_only={bool}`
pub async fn suit_servers_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitComponentListReq>,
) -> Result<Json<SuitServersListApiResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify suit exists
    let suit = crate::config::suit::get_config_suit(&db.pool, &request.suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                request.suit_id
            )));
        }
    };

    // Get servers in the suit
    let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &request.suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get suit servers: {e}")))?;

    // Convert to response format (simplified for now)
    let mut servers = Vec::new();
    for server_config in server_configs {
        // Get server details from server_config table
        if let Ok(Some(server)) = crate::config::server::get_server_by_id(&db.pool, &server_config.server_id).await {
            servers.push(ConfigSuitServerResp {
                id: server_config.server_id.clone(),
                name: server.name,
                enabled: server_config.enabled,
                allowed_operations: vec!["enable".to_string(), "disable".to_string()],
            });
        }
    }

    // Apply enabled filter if requested
    if request.enabled_only.unwrap_or(false) {
        servers.retain(|s| s.enabled);
    }

    let total = servers.len();
    let response = SuitServersListResp {
        suit_id: request.suit_id,
        suit_name: suit.name,
        servers,
        total,
    };

    Ok(Json(SuitServersListApiResp::success(response)))
}

/// List tools in a configuration suit
///
/// **Endpoint:** `GET /mcp/suits/tools/list?suit_id={suit_id}&enabled_only={bool}`
pub async fn suit_tools_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitComponentListReq>,
) -> Result<Json<SuitToolsListApiResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify suit exists
    let suit = crate::config::suit::get_config_suit(&db.pool, &request.suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{}' not found",
                request.suit_id
            )));
        }
    };

    // Get tools in the suit
    let tool_configs = crate::config::suit::get_config_suit_tools(&db.pool, &request.suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get suit tools: {e}")))?;

    // Convert to response format (simplified for now)
    let mut tools = Vec::new();
    for tool_config in tool_configs {
        // Get server details to include server name
        if let Ok(Some(server)) = crate::config::server::get_server_by_id(&db.pool, &tool_config.server_id).await {
            tools.push(ConfigSuitToolResp {
                id: tool_config.id.clone(),
                server_id: tool_config.server_id.clone(),
                server_name: server.name,
                tool_name: tool_config.tool_name.clone(),
                unique_name: Some(tool_config.unique_name.clone()),
                enabled: tool_config.enabled,
                allowed_operations: vec!["enable".to_string(), "disable".to_string()],
            });
        }
    }

    // Apply enabled filter if requested
    if request.enabled_only.unwrap_or(false) {
        tools.retain(|t| t.enabled);
    }

    let total = tools.len();
    let response = SuitToolsListResp {
        suit_id: request.suit_id,
        suit_name: suit.name,
        tools,
        total,
    };

    Ok(Json(SuitToolsListApiResp::success(response)))
}

// ==========================================
// COMPONENT MANAGEMENT OPERATIONS
// ==========================================

/// Manage component operations (enable/disable servers, tools, etc.)
///
/// **Endpoint:** `POST /mcp/suits/servers/manage` or `POST /mcp/suits/tools/manage`
pub async fn manage_suit_component(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuitComponentManageReq>,
) -> Result<Json<SuitComponentManageApiResp>, ApiError> {
    let db = get_database(&state).await?;

    // For now, implement server management (most common case)
    let (result, status) = match request.action {
        ComponentAction::Enable => {
            // Add server to suit (this enables it)
            crate::config::suit::add_server_to_config_suit(&db.pool, &request.suit_id, &request.component_id, true)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to enable server: {e}")))?;
            ("enabled", "active")
        }
        ComponentAction::Disable => {
            // Remove server from suit (this disables it)
            crate::config::suit::remove_server_from_config_suit(&db.pool, &request.suit_id, &request.component_id)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to disable server: {e}")))?;
            ("disabled", "inactive")
        }
    };

    let response = SuitComponentManageResp {
        suit_id: request.suit_id,
        component_id: request.component_id,
        component_type: "server".to_string(), // TODO: Auto-detect
        result: result.to_string(),
        status: status.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(Json(SuitComponentManageApiResp::success(response)))
}
