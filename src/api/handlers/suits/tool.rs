// MCPMate Proxy API handlers for Config Suit tool management
// Contains handler functions for managing tools in Config Suits

use std::collections::HashMap;

use super::{
    check_tool_belongs_to_suit, common::*, get_suit_or_error, get_tool_or_error, get_tool_with_details_or_error,
};

/// List tools in a configuration suit
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SuitToolsResp>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = get_suit_or_error(&db, &id).await?;

    // Get all tools in the suit (using new architecture)
    let tool_configs = crate::config::suit::get_config_suit_tools(&db.pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get tool configurations: {e}")))?;

    // Convert to response format
    let mut tool_responses = Vec::new();
    for config in tool_configs {
        let mut allowed_operations = Vec::new();
        if config.enabled {
            allowed_operations.push("disable".to_string());
        } else {
            allowed_operations.push("enable".to_string());
        }

        tool_responses.push(SuitToolData {
            id: config.id,
            server_id: config.server_id.clone(),
            server_name: config.server_name.clone(),
            tool_name: config.tool_name.clone(),
            unique_name: Some(config.unique_name.clone()),
            enabled: config.enabled,
            allowed_operations,
        });
    }

    // Return response
    Ok(Json(SuitToolsResp {
        suit_id: id,
        suit_name: suit.name,
        tools: tool_responses,
    }))
}

/// Enable a tool in a configuration suit (ID-only)
pub async fn enable_tool(
    State(state): State<Arc<AppState>>,
    Path((suit_id, tool_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationData>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    // ID-only resolution
    let tool = get_tool_or_error(&db, &tool_id).await?;

    // Check if the tool belongs to the specified suit
    check_tool_belongs_to_suit(&tool, &suit_id)?;

    // Check if the tool is already enabled
    if tool.enabled {
        // Get tool details for response
        let tool_details = get_tool_with_details_or_error(&db, &tool.id).await?;
        return Ok(Json(SuitOperationData {
            id: tool.id.clone(),
            name: format!("{}/{}", tool_details.server_name, tool_details.tool_name),
            result: "Tool is already enabled in this configuration suit".to_string(),
            status: "Enabled".to_string(),
            allowed_operations: vec!["disable".to_string()],
        }));
    }

    // Enable the tool
    sqlx::query("UPDATE config_suit_tool SET enabled = true WHERE id = ?")
        .bind(&tool.id)
        .execute(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to enable tool in configuration suit: {e}")))?;

    // Get tool details for response
    let tool_details = get_tool_with_details_or_error(&db, &tool.id).await?;

    // Return success response
    Ok(Json(SuitOperationData {
        id: tool.id,
        name: format!("{}/{}", tool_details.server_name, tool_details.tool_name),
        result: "Successfully enabled tool in configuration suit".to_string(),
        status: "Enabled".to_string(),
        allowed_operations: vec!["disable".to_string()],
    }))
}

/// Disable a tool in a configuration suit (ID-only)
pub async fn disable_tool(
    State(state): State<Arc<AppState>>,
    Path((suit_id, tool_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationData>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    // ID-only resolution
    let tool = get_tool_or_error(&db, &tool_id).await?;

    // Check if the tool belongs to the specified suit
    check_tool_belongs_to_suit(&tool, &suit_id)?;

    // Check if the tool is already disabled
    if !tool.enabled {
        // Get tool details for response
        let tool_details = get_tool_with_details_or_error(&db, &tool.id).await?;
        return Ok(Json(SuitOperationData {
            id: tool.id.clone(),
            name: format!("{}/{}", tool_details.server_name, tool_details.tool_name),
            result: "Tool is already disabled in this configuration suit".to_string(),
            status: "Disabled".to_string(),
            allowed_operations: vec!["enable".to_string()],
        }));
    }

    // Disable the tool
    sqlx::query("UPDATE config_suit_tool SET enabled = false WHERE id = ?")
        .bind(&tool.id)
        .execute(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to disable tool in configuration suit: {e}")))?;

    // Get tool details for response
    let tool_details = get_tool_with_details_or_error(&db, &tool.id).await?;

    // Return success response
    Ok(Json(SuitOperationData {
        id: tool.id,
        name: format!("{}/{}", tool_details.server_name, tool_details.tool_name),
        result: "Successfully disabled tool in configuration suit".to_string(),
        status: "Disabled".to_string(),
        allowed_operations: vec!["enable".to_string()],
    }))
}

/// Batch enable tools in a configuration suit
pub async fn batch_enable_tools(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<SuitBatchOperationReq>,
) -> Result<Json<SuitBatchOperationResp>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each tool id (ID-only)
    for tool_id in payload.ids {
        // Get by ID and validate suit ownership
        match get_tool_or_error(&db, &tool_id).await {
            Ok(tool) => {
                // Skip if already enabled
                if tool.enabled {
                    continue;
                }

                // Enable the tool
                match sqlx::query("UPDATE config_suit_tool SET enabled = true WHERE id = ?")
                    .bind(&tool.id)
                    .execute(&db.pool)
                    .await
                {
                    Ok(_) => {
                        successful_ids.push(tool_id.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(tool_id.clone(), format!("Failed to enable tool: {e}"));
                    }
                }
            }
            Err(e) => {
                failed_ids.insert(tool_id.clone(), e.to_string());
            }
        }
    }

    // Return response
    Ok(Json(SuitBatchOperationResp {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}

/// Batch disable tools in a configuration suit
pub async fn batch_disable_tools(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<SuitBatchOperationReq>,
) -> Result<Json<SuitBatchOperationResp>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each tool id (ID-only)
    for tool_id in payload.ids {
        // Get by ID and validate suit ownership
        match get_tool_or_error(&db, &tool_id).await {
            Ok(tool) => {
                // Skip if already disabled
                if !tool.enabled {
                    continue;
                }

                // Disable the tool
                match sqlx::query("UPDATE config_suit_tool SET enabled = false WHERE id = ?")
                    .bind(&tool.id)
                    .execute(&db.pool)
                    .await
                {
                    Ok(_) => {
                        successful_ids.push(tool_id.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(tool_id.clone(), format!("Failed to disable tool: {e}"));
                    }
                }
            }
            Err(e) => {
                failed_ids.insert(tool_id.clone(), e.to_string());
            }
        }
    }

    // Return response
    Ok(Json(SuitBatchOperationResp {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}
