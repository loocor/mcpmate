// MCPMate Proxy API handlers for Config Suit prompt management
// Contains handler functions for Config Suit prompt endpoints

use super::common::*;
use super::helpers::get_suit_or_error;

/// List all prompts in a configuration suit
pub async fn list_prompts(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
) -> Result<Json<ConfigSuitPromptsResponse>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Verify the configuration suit exists
    let suit = get_suit_or_error(db, &suit_id).await?;

    // Get all prompts for this configuration suit
    let prompts = crate::config::suit::get_prompts_for_config_suit(&db.pool, &suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get prompts: {e}")))?;

    // Convert to response format
    let prompt_responses: Vec<ConfigSuitPromptResponse> = prompts
        .into_iter()
        .map(|prompt| ConfigSuitPromptResponse {
            id: prompt.id.unwrap_or_default(),
            server_id: prompt.server_id,
            server_name: prompt.server_name,
            prompt_name: prompt.prompt_name,
            enabled: prompt.enabled,
            allowed_operations: vec!["enable".to_string(), "disable".to_string()],
        })
        .collect();

    Ok(Json(ConfigSuitPromptsResponse {
        suit_id: suit_id.clone(),
        suit_name: suit.name,
        prompts: prompt_responses,
    }))
}

/// Enable a prompt in a configuration suit
pub async fn enable_prompt(
    State(state): State<Arc<AppState>>,
    Path((suit_id, prompt_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Verify the configuration suit exists
    let _suit = get_suit_or_error(db, &suit_id).await?;

    // Enable the prompt
    crate::config::suit::update_prompt_enabled_status(&db.pool, &prompt_id, true)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to enable prompt: {e}")))?;

    Ok(Json(SuitOperationResponse {
        id: prompt_id.clone(),
        name: format!("Prompt {}", prompt_id),
        result: "enabled".to_string(),
        status: "success".to_string(),
        allowed_operations: vec!["disable".to_string()],
    }))
}

/// Disable a prompt in a configuration suit
pub async fn disable_prompt(
    State(state): State<Arc<AppState>>,
    Path((suit_id, prompt_id)): Path<(String, String)>,
) -> Result<Json<SuitOperationResponse>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Verify the configuration suit exists
    let _suit = get_suit_or_error(db, &suit_id).await?;

    // Disable the prompt
    crate::config::suit::update_prompt_enabled_status(&db.pool, &prompt_id, false)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to disable prompt: {e}")))?;

    Ok(Json(SuitOperationResponse {
        id: prompt_id.clone(),
        name: format!("Prompt {}", prompt_id),
        result: "disabled".to_string(),
        status: "success".to_string(),
        allowed_operations: vec!["enable".to_string()],
    }))
}

/// Batch enable prompts in a configuration suit
pub async fn batch_enable_prompts(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(request): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Verify the configuration suit exists
    let _suit = get_suit_or_error(db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = std::collections::HashMap::new();

    for prompt_id in request.ids {
        match crate::config::suit::update_prompt_enabled_status(&db.pool, &prompt_id, true).await {
            Ok(_) => {
                successful_ids.push(prompt_id);
            }
            Err(e) => {
                failed_ids.insert(prompt_id, format!("Failed to enable prompt: {e}"));
            }
        }
    }

    Ok(Json(BatchOperationResponse {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}

/// Batch disable prompts in a configuration suit
pub async fn batch_disable_prompts(
    State(state): State<Arc<AppState>>,
    Path(suit_id): Path<String>,
    Json(request): Json<BatchOperationRequest>,
) -> Result<Json<BatchOperationResponse>, ApiError> {
    let db = state
        .database
        .as_ref()
        .ok_or_else(|| ApiError::InternalError("Database not available".to_string()))?;

    // Verify the configuration suit exists
    let _suit = get_suit_or_error(db, &suit_id).await?;

    let mut successful_ids = Vec::new();
    let mut failed_ids = std::collections::HashMap::new();

    for prompt_id in request.ids {
        match crate::config::suit::update_prompt_enabled_status(&db.pool, &prompt_id, false).await {
            Ok(_) => {
                successful_ids.push(prompt_id);
            }
            Err(e) => {
                failed_ids.insert(prompt_id, format!("Failed to disable prompt: {e}"));
            }
        }
    }

    Ok(Json(BatchOperationResponse {
        success_count: successful_ids.len(),
        successful_ids,
        failed_ids,
    }))
}
