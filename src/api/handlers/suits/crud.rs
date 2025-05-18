// MCPMate Proxy API handlers for Config Suit CRUD operations
// Contains handler functions for creating, updating, and deleting Config Suits

use std::str::FromStr;

use uuid::Uuid;

use super::common::*;

/// Create a new configuration suit
pub async fn create_suit(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateConfigSuitRequest>,
) -> Result<Json<ConfigSuitResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Check if a suit with the same name already exists
    let existing_suit =
        crate::conf::operations::suit::get_config_suit_by_name(&db.pool, &payload.name)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to check configuration suit: {e}"))
            })?;

    if existing_suit.is_some() {
        return Err(ApiError::Conflict(format!(
            "Configuration suit with name '{}' already exists",
            payload.name
        )));
    }

    // Parse suit type
    let suit_type = match ConfigSuitType::from_str(&payload.suit_type) {
        Ok(t) => t,
        Err(_) => {
            return Err(ApiError::BadRequest(format!(
                "Invalid configuration suit type: {}. Must be one of: host_app, scenario, shared",
                payload.suit_type
            )));
        }
    };

    // Create new configuration suit
    let mut new_suit = ConfigSuit::new_with_description(
        payload.name.clone(),
        payload.description.clone(),
        suit_type,
    );

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

    // Generate UUID for the new suit
    let suit_id = Uuid::new_v4().to_string();
    new_suit.id = Some(suit_id.clone());

    // If cloning from existing suit, copy server and tool associations
    if let Some(clone_from_id) = payload.clone_from_id {
        // Check if the source suit exists
        let source_suit = crate::conf::operations::suit::get_config_suit(&db.pool, &clone_from_id)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to get source configuration suit: {e}"))
            })?;

        if source_suit.is_none() {
            return Err(ApiError::NotFound(format!(
                "Source configuration suit with ID '{clone_from_id}' not found"
            )));
        }

        // Start a transaction
        let mut tx =
            db.pool.begin().await.map_err(|e| {
                ApiError::InternalError(format!("Failed to begin transaction: {e}"))
            })?;

        // Insert the new suit
        crate::conf::operations::suit::upsert_config_suit_tx(&mut tx, &new_suit)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to create configuration suit: {e}"))
            })?;

        // Copy server associations
        let server_configs =
            crate::conf::operations::get_config_suit_servers(&db.pool, &clone_from_id)
                .await
                .map_err(|e| {
                    ApiError::InternalError(format!("Failed to get server configurations: {e}"))
                })?;

        for server_config in server_configs {
            let new_server_config = ConfigSuitServer {
                id: Some(Uuid::new_v4().to_string()),
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
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to create server association: {e}"))
            })?;
        }

        // Copy tool associations
        let tool_configs =
            crate::conf::operations::tool::get_tools_by_suit_id(&db.pool, &clone_from_id)
                .await
                .map_err(|e| {
                    ApiError::InternalError(format!("Failed to get tool configurations: {e}"))
                })?;

        for tool_config in tool_configs {
            let new_tool_config = ConfigSuitTool {
                id: Some(Uuid::new_v4().to_string()),
                config_suit_id: suit_id.clone(),
                server_id: tool_config.server_id.clone(),
                server_name: tool_config.server_name.clone(),
                tool_name: tool_config.tool_name.clone(),
                prefixed_name: tool_config.prefixed_name.clone(),
                enabled: tool_config.enabled,
                created_at: None,
                updated_at: None,
            };

            sqlx::query(
                r#"
                INSERT INTO config_suit_tool (id, config_suit_id, server_id, server_name, tool_name, prefixed_name, enabled)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(new_tool_config.id.as_ref().unwrap())
            .bind(&new_tool_config.config_suit_id)
            .bind(&new_tool_config.server_id)
            .bind(&new_tool_config.server_name)
            .bind(&new_tool_config.tool_name)
            .bind(&new_tool_config.prefixed_name)
            .bind(new_tool_config.enabled)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to create tool association: {e}"))
            })?;
        }

        // Commit the transaction
        tx.commit()
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to commit transaction: {e}")))?;
    } else {
        // Insert the new suit without cloning
        crate::conf::operations::suit::upsert_config_suit(&db.pool, &new_suit)
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to create configuration suit: {e}"))
            })?;
    }

    // Get the created suit
    let created_suit = crate::conf::operations::suit::get_config_suit(&db.pool, &suit_id)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to get created configuration suit: {e}"))
        })?
        .unwrap();

    // Convert to response format
    let response = suit_to_response(&created_suit);

    // Return response
    Ok(Json(response))
}

/// Update an existing configuration suit
pub async fn update_suit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateConfigSuitRequest>,
) -> Result<Json<ConfigSuitResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the existing suit
    let existing_suit = crate::conf::operations::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    // Check if the suit exists
    let mut suit = match existing_suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{id}' not found"
            )));
        }
    };

    // Update fields if provided
    if let Some(name) = payload.name {
        // Check if the new name is already used by another suit
        if name != suit.name {
            let existing_suit =
                crate::conf::operations::suit::get_config_suit_by_name(&db.pool, &name)
                    .await
                    .map_err(|e| {
                        ApiError::InternalError(format!(
                            "Failed to check configuration suit: {e}"
                        ))
                    })?;

            if let Some(existing) = existing_suit {
                if existing.id != suit.id {
                    return Err(ApiError::Conflict(format!(
                        "Configuration suit with name '{name}' already exists"
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
                return Err(ApiError::BadRequest(format!(
                    "Invalid configuration suit type: {suit_type}. Must be one of: host_app, scenario, shared"
                )));
            }
        };
        suit.suit_type = parsed_type.as_str().to_string();
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
    crate::conf::operations::suit::upsert_config_suit(&db.pool, &suit)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to update configuration suit: {e}"))
        })?;

    // Get the updated suit
    let updated_suit = crate::conf::operations::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to get updated configuration suit: {e}"))
        })?
        .unwrap();

    // Convert to response format
    let response = suit_to_response(&updated_suit);

    // Return response
    Ok(Json(response))
}

/// Delete a configuration suit
pub async fn delete_suit(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = crate::conf::operations::suit::get_config_suit(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get configuration suit: {e}")))?;

    // Check if the suit exists
    let suit = match suit {
        Some(s) => s,
        None => {
            return Err(ApiError::NotFound(format!(
                "Configuration suit with ID '{id}' not found"
            )));
        }
    };

    // Check if this is the default suit
    if suit.is_default {
        return Err(ApiError::BadRequest(
            "Cannot delete the default configuration suit".to_string(),
        ));
    }

    // Delete the suit
    let deleted = crate::conf::operations::suit::delete_config_suit(&db.pool, &id)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to delete configuration suit: {e}"))
        })?;

    if !deleted {
        return Err(ApiError::InternalError(format!(
            "Failed to delete configuration suit with ID '{id}'"
        )));
    }

    // Return success response
    Ok(Json(SuitOperationResponse {
        id,
        name: suit.name,
        result: "Successfully deleted configuration suit".to_string(),
        status: "Deleted".to_string(),
        allowed_operations: Vec::new(),
    }))
}
