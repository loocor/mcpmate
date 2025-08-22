// MCPMate Proxy API handlers for Config Suit CRUD operations
// Contains handler functions for creating, updating, and deleting Config Suits

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use std::str::FromStr;
use std::sync::Arc;

use crate::api::{
    models::suits::{
        ConfigSuitApiResp, ConfigSuitResp, CreateConfigSuitReq, SuitOperationApiResp, SuitOperationResp,
        UpdateConfigSuitReq,
    },
    routes::AppState,
};
use crate::common::config::ConfigSuitType;
use crate::config::models::suit::{ConfigSuit, ConfigSuitServer};
use crate::config::suit::get_config_suit_by_name;
use crate::generate_id;

/// Get database pool from app state
async fn get_database(state: &Arc<AppState>) -> Result<Arc<crate::config::database::Database>, StatusCode> {
    match &state.database {
        Some(db) => Ok(db.clone()),
        None => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// Convert suit database model to response format
fn suit_to_response(suit: &ConfigSuit) -> ConfigSuitResp {
    ConfigSuitResp {
        id: suit.id.clone().unwrap_or_default(),
        name: suit.name.clone(),
        description: suit.description.clone(),
        suit_type: suit.suit_type.to_string(),
        multi_select: suit.multi_select,
        priority: suit.priority,
        is_active: suit.is_active,
        is_default: suit.is_default,
        allowed_operations: vec!["update".to_string(), "delete".to_string()],
    }
}

/// Create a new configuration suit
pub async fn create_suit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateConfigSuitReq>,
) -> Result<Json<ConfigSuitApiResp>, StatusCode> {
    // Get database reference
    let db = get_database(&state).await?;

    // Check if a suit with the same name already exists
    let existing_suit = get_config_suit_by_name(&db.pool, &payload.name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if existing_suit.is_some() {
        return Ok(Json(ConfigSuitApiResp::error(
            "CONFLICT",
            &format!("Configuration suit with name '{}' already exists", payload.name),
        )));
    }

    // Parse suit type
    let suit_type = match ConfigSuitType::from_str(&payload.suit_type) {
        Ok(t) => t,
        Err(_) => {
            return Ok(Json(ConfigSuitApiResp::error(
                "INVALID_TYPE",
                &format!(
                    "Invalid configuration suit type: {}. Must be one of: host_app, scenario, shared",
                    payload.suit_type
                ),
            )));
        }
    };

    // Create new configuration suit
    let mut new_suit = ConfigSuit::new_with_description(payload.name.clone(), payload.description.clone(), suit_type);

    // Set optional fields
    if let Some(multi_select) = payload.multi_select {
        new_suit.multi_select = multi_select;
    }
    if let Some(priority) = payload.priority {
        new_suit.priority = priority;
    }
    if let Some(is_active) = payload.is_active {
        new_suit.is_active = is_active;
    }
    if let Some(is_default) = payload.is_default {
        new_suit.is_default = is_default;
    }

    // Generate an id for the new suit
    let suit_id = generate_id!("suit");
    new_suit.id = Some(suit_id.clone());

    // If cloning from existing suit, copy server and tool associations
    if let Some(clone_from_id) = payload.clone_from_id {
        // Check if the source suit exists
        let source_suit = crate::config::suit::get_config_suit(&db.pool, &clone_from_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if source_suit.is_none() {
            return Ok(Json(ConfigSuitApiResp::error(
                "NOT_FOUND",
                &format!("Source configuration suit with ID '{clone_from_id}' not found"),
            )));
        }

        // Start a transaction
        let mut tx = db.pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Insert the new suit
        crate::config::suit::upsert_config_suit_tx(&mut tx, &new_suit)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Copy server associations
        let server_configs = crate::config::suit::get_config_suit_servers(&db.pool, &clone_from_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        for server_config in server_configs {
            let new_server_config = ConfigSuitServer {
                id: Some(generate_id!("ssrv")),
                config_suit_id: suit_id.clone(),
                server_id: server_config.server_id.clone(),
                enabled: server_config.enabled,
                created_at: None,
                updated_at: None,
            };

            sqlx::query(
                r#"
                INSERT INTO config_suit_server (id, config_suit_id, server_id, enabled)
                VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(new_server_config.id.as_ref().unwrap())
            .bind(&new_server_config.config_suit_id)
            .bind(&new_server_config.server_id)
            .bind(new_server_config.enabled)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }

        // Copy tool associations (using new architecture)
        let tool_configs = crate::config::suit::get_config_suit_tools(&db.pool, &clone_from_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        for tool_config in tool_configs {
            // Use the new architecture to add tools to the config suit
            crate::config::suit::add_tool_to_config_suit(
                &db.pool,
                &suit_id,
                &tool_config.server_id,
                &tool_config.tool_name,
                tool_config.enabled,
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }

        // Commit the transaction
        tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        // Insert the new suit without cloning
        crate::config::suit::upsert_config_suit(&db.pool, &new_suit)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Get the created suit
    let created_suit = crate::config::suit::get_config_suit(&db.pool, &suit_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap();

    // Convert to response format
    let response = suit_to_response(&created_suit);

    // Return response
    Ok(Json(ConfigSuitApiResp::success(response)))
}

/// Update an existing configuration suit
pub async fn update_suit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateConfigSuitReq>,
) -> Result<Json<ConfigSuitApiResp>, StatusCode> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the existing suit
    let existing_suit = crate::config::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Check if the suit exists
    let mut suit = match existing_suit {
        Some(s) => s,
        None => {
            return Ok(Json(ConfigSuitApiResp::error(
                "NOT_FOUND",
                &format!("Configuration suit with ID '{id}' not found"),
            )));
        }
    };

    // Update fields if provided
    if let Some(name) = payload.name {
        // Check if the new name is already used by another suit
        if name != suit.name {
            let existing_suit = crate::config::suit::get_config_suit_by_name(&db.pool, &name)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            if let Some(existing) = existing_suit {
                if existing.id != suit.id {
                    return Ok(Json(ConfigSuitApiResp::error(
                        "CONFLICT",
                        &format!("Configuration suit with name '{name}' already exists"),
                    )));
                }
            }
        }
        suit.name = name;
    }

    if let Some(description) = payload.description {
        suit.description = Some(description);
    }

    if let Some(suit_type) = payload.suit_type {
        // Parse suit type
        let parsed_type = match ConfigSuitType::from_str(&suit_type) {
            Ok(t) => t,
            Err(_) => {
                return Ok(Json(ConfigSuitApiResp::error(
                    "INVALID_TYPE",
                    &format!(
                        "Invalid configuration suit type: {suit_type}. Must be one of: host_app, scenario, shared"
                    ),
                )));
            }
        };
        suit.suit_type = parsed_type;
    }

    if let Some(multi_select) = payload.multi_select {
        suit.multi_select = multi_select;
    }

    if let Some(priority) = payload.priority {
        suit.priority = priority;
    }

    if let Some(is_active) = payload.is_active {
        suit.is_active = is_active;
    }

    if let Some(is_default) = payload.is_default {
        suit.is_default = is_default;
    }

    // Update the suit
    crate::config::suit::upsert_config_suit(&db.pool, &suit)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get the updated suit
    let updated_suit = crate::config::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap();

    // Convert to response format
    let response = suit_to_response(&updated_suit);

    // Return response
    Ok(Json(ConfigSuitApiResp::success(response)))
}

/// Delete a configuration suit
pub async fn delete_suit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SuitOperationApiResp>, StatusCode> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = crate::config::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Check if the suit exists
    let suit = match suit {
        Some(s) => s,
        None => {
            return Ok(Json(SuitOperationApiResp::error(
                "NOT_FOUND",
                &format!("Configuration suit with ID '{id}' not found"),
            )));
        }
    };

    // Check if this is the default suit
    if suit.is_default {
        return Ok(Json(SuitOperationApiResp::error(
            "BAD_REQUEST",
            "Cannot delete the default configuration suit",
        )));
    }

    // Delete the suit
    let deleted = crate::config::suit::delete_config_suit(&db.pool, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !deleted {
        return Ok(Json(SuitOperationApiResp::error(
            "INTERNAL_ERROR",
            &format!("Failed to delete configuration suit with ID '{id}'"),
        )));
    }

    // Return success response
    Ok(Json(SuitOperationApiResp::success(SuitOperationResp {
        id,
        name: suit.name,
        result: "Successfully deleted configuration suit".to_string(),
        status: "Deleted".to_string(),
        allowed_operations: Vec::new(),
    })))
}
