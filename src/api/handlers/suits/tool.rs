// MCPMate Proxy API handlers for Config Suit tool management
// Contains handler functions for managing tools in Config Suits

use std::collections::HashMap;

use super::{check_tool_belongs_to_suit, common::*, get_suit_or_error, get_tool_or_error};

/// List tools in a configuration suit
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigSuitToolsResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists and get its name
    let suit = get_suit_or_error(&db, &id).await?;

    // Get all tools in the suit
    let tool_configs = crate::conf::operations::tool::get_tools_by_suit_id(&db.pool, &id)
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

        tool_responses.push(ConfigSuitToolResponse {
            id: config.id.unwrap_or_default(),
            server_id: config.server_id.clone(),
            server_name: config.server_name.clone(),
            tool_name: config.tool_name.clone(),
            unique_name: config.unique_name.clone(),
            enabled: config.enabled,
            allowed_operations,
        });
    }

    // Return response
    Ok(Json(ConfigSuitToolsResponse {
        suit_id: id,
        suit_name: suit.name,
        tools: tool_responses,
    }))
}

/// Enable a tool in a configuration suit
pub async fn enable_tool(
    State(state): State<Arc<AppState>>,
    Path((suit_id, tool_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit and tool
    let _suit = get_suit_or_error(&db, &suit_id).await?;
    let tool = get_tool_or_error(&db, &tool_id).await?;

    // Check if the tool belongs to the specified suit
    check_tool_belongs_to_suit(&tool, &suit_id)?;

    // Check if the tool is already enabled
    if tool.enabled {
        return Ok(Json(SuitOperationResponse {
            id: tool_id,
            name: format!("{}/{}", tool.server_id, tool.tool_name),
            result: "Tool is already enabled in this configuration suit".to_string(),
            status: "Enabled".to_string(),
            allowed_operations: vec!["disable".to_string()],
        }));
    }

    // Enable the tool
    crate::conf::operations::tool::set_tool_enabled_by_id(&db.pool, &tool_id, true)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to enable tool in configuration suit: {e}"))
        })?;

    // Return success response
    Ok(Json(SuitOperationResponse {
        id: tool_id,
        name: format!("{}/{}", tool.server_id, tool.tool_name),
        result: "Successfully enabled tool in configuration suit".to_string(),
        status: "Enabled".to_string(),
        allowed_operations: vec!["disable".to_string()],
    }))
}

/// Disable a tool in a configuration suit
pub async fn disable_tool(
    State(state): State<Arc<AppState>>,
    Path((suit_id, tool_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit and tool
    let _suit = get_suit_or_error(&db, &suit_id).await?;
    let tool = get_tool_or_error(&db, &tool_id).await?;

    // Check if the tool belongs to the specified suit
    check_tool_belongs_to_suit(&tool, &suit_id)?;

    // Check if the tool is already disabled
    if !tool.enabled {
        return Ok(Json(SuitOperationResponse {
            id: tool_id,
            name: format!("{}/{}", tool.server_id, tool.tool_name),
            result: "Tool is already disabled in this configuration suit".to_string(),
            status: "Disabled".to_string(),
            allowed_operations: vec!["enable".to_string()],
        }));
    }

    // Disable the tool
    crate::conf::operations::tool::set_tool_enabled_by_id(&db.pool, &tool_id, false)
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to disable tool in configuration suit: {e}"))
        })?;

    // Return success response
    Ok(Json(SuitOperationResponse {
        id: tool_id,
        name: format!("{}/{}", tool.server_id, tool.tool_name),
        result: "Successfully disabled tool in configuration suit".to_string(),
        status: "Disabled".to_string(),
        allowed_operations: vec!["enable".to_string()],
    }))
}

/// Batch enable tools in a configuration suit
pub async fn batch_enable_tools(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each tool ID
    for tool_id in payload.ids {
        // Get the tool to check if it exists
        let tool = crate::conf::operations::tool::get_config_suit_tool_by_id(&db.pool, &tool_id)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to get tool: {e}")))?;

        // Check if the tool exists and belongs to the specified suit
        match tool {
            Some(t) => {
                if t.config_suit_id != suit_id {
                    failed_ids.insert(
                        tool_id.clone(),
                        "Tool does not belong to the specified configuration suit".to_string(),
                    );
                    continue;
                }

                // Skip if already enabled
                if t.enabled {
                    continue;
                }

                // Enable the tool
                match crate::conf::operations::tool::set_tool_enabled_by_id(
                    &db.pool, &tool_id, true,
                )
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
            None => {
                failed_ids.insert(tool_id.clone(), "Tool not found".to_string());
            }
        }
    }

    // Return response
    Ok(Json(BatchOperationResponse {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}

/// Batch disable tools in a configuration suit
pub async fn batch_disable_tools(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(payload): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each tool ID
    for tool_id in payload.ids {
        // Get the tool to check if it exists
        let tool = crate::conf::operations::tool::get_config_suit_tool_by_id(&db.pool, &tool_id)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to get tool: {e}")))?;

        // Check if the tool exists and belongs to the specified suit
        match tool {
            Some(t) => {
                if t.config_suit_id != suit_id {
                    failed_ids.insert(
                        tool_id.clone(),
                        "Tool does not belong to the specified configuration suit".to_string(),
                    );
                    continue;
                }

                // Skip if already disabled
                if !t.enabled {
                    continue;
                }

                // Disable the tool
                match crate::conf::operations::tool::set_tool_enabled_by_id(
                    &db.pool, &tool_id, false,
                )
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
            None => {
                failed_ids.insert(tool_id.clone(), "Tool not found".to_string());
            }
        }
    }

    // Return response
    Ok(Json(BatchOperationResponse {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}
