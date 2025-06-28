// MCPMate Proxy API handlers for Config Suit tool management
// Contains handler functions for managing tools in Config Suits

use std::collections::HashMap;

use super::{
    check_tool_belongs_to_suit, common::*, get_suit_or_error, get_tool_or_error,
    get_tool_with_details_or_error, get_or_create_tool_by_name, resolve_tool_for_batch_operation,
};

/// List tools in a configuration suit
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConfigSuitToolsResponse>, ApiError> {
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

        tool_responses.push(ConfigSuitToolResponse {
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
    Ok(Json(ConfigSuitToolsResponse {
        suit_id: id,
        suit_name: suit.name,
        tools: tool_responses,
    }))
}

/// Enable a tool in a configuration suit
pub async fn enable_tool(
    State(state): State<Arc<AppState>>,
    Path((suit_id, tool_identifier)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    // Smart tool resolution: try ID first, then tool name
    let tool = match get_tool_or_error(&db, &tool_identifier).await {
        Ok(tool) => tool,
        Err(_) => {
            // If tool ID lookup failed, try to find or create by tool name
            get_or_create_tool_by_name(&db, &suit_id, &tool_identifier).await?
        }
    };

    // Check if the tool belongs to the specified suit
    check_tool_belongs_to_suit(&tool, &suit_id)?;

    // Check if the tool is already enabled
    if tool.enabled {
        // Get tool details for response
        let tool_details = get_tool_with_details_or_error(&db, &tool.id).await?;
        return Ok(Json(SuitOperationResponse {
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
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to enable tool in configuration suit: {e}"))
        })?;

    // Get tool details for response
    let tool_details = get_tool_with_details_or_error(&db, &tool.id).await?;

    // Return success response
    Ok(Json(SuitOperationResponse {
        id: tool.id,
        name: format!("{}/{}", tool_details.server_name, tool_details.tool_name),
        result: "Successfully enabled tool in configuration suit".to_string(),
        status: "Enabled".to_string(),
        allowed_operations: vec!["disable".to_string()],
    }))
}

/// Disable a tool in a configuration suit
pub async fn disable_tool(
    State(state): State<Arc<AppState>>,
    Path((suit_id, tool_identifier)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    // Smart tool resolution: try ID first, then tool name
    let tool = match get_tool_or_error(&db, &tool_identifier).await {
        Ok(tool) => tool,
        Err(_) => {
            // If tool ID lookup failed, try to find or create by tool name
            get_or_create_tool_by_name(&db, &suit_id, &tool_identifier).await?
        }
    };

    // Check if the tool belongs to the specified suit
    check_tool_belongs_to_suit(&tool, &suit_id)?;

    // Check if the tool is already disabled
    if !tool.enabled {
        // Get tool details for response
        let tool_details = get_tool_with_details_or_error(&db, &tool.id).await?;
        return Ok(Json(SuitOperationResponse {
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
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to disable tool in configuration suit: {e}"))
        })?;

    // Get tool details for response
    let tool_details = get_tool_with_details_or_error(&db, &tool.id).await?;

    // Return success response
    Ok(Json(SuitOperationResponse {
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
    Json(payload): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    // Get database reference
    let db = get_database(&state).await?;

    // Get the suit to check if it exists
    let _suit = get_suit_or_error(&db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = HashMap::new();

    // Process each tool identifier (ID or name)
    for tool_identifier in payload.ids {
        // Resolve tool identifier to ConfigSuitTool
        match resolve_tool_for_batch_operation(&db, &suit_id, &tool_identifier).await {
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
                        successful_ids.push(tool_identifier.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(tool_identifier.clone(), format!("Failed to enable tool: {e}"));
                    }
                }
            }
            Err(e) => {
                failed_ids.insert(tool_identifier.clone(), e);
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

    // Process each tool identifier (ID or name)
    for tool_identifier in payload.ids {
        // Resolve tool identifier to ConfigSuitTool
        match resolve_tool_for_batch_operation(&db, &suit_id, &tool_identifier).await {
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
                        successful_ids.push(tool_identifier.clone());
                    }
                    Err(e) => {
                        failed_ids.insert(tool_identifier.clone(), format!("Failed to disable tool: {e}"));
                    }
                }
            }
            Err(e) => {
                failed_ids.insert(tool_identifier.clone(), e);
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
