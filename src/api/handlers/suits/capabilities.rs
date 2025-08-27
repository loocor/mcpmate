// MCPMate Proxy API handlers for Config Suit capabilities management
// Contains handler functions for managing tools, resources, and prompts in Config Suits

use super::{common::*, helpers::get_suit_or_error};
use crate::api::models::suits::{
    ComponentOperationResult, SuitComponentAction, SuitComponentListReq, SuitComponentManageReq, SuitPromptData,
    SuitPromptsListData, SuitPromptsListResp, SuitResourceData, SuitResourcesListData, SuitResourcesListResp,
    SuitServerManageData, SuitServerManageResp, SuitToolData, SuitToolsListData, SuitToolsListResp,
};

// Component type enumeration for type-safe operations
#[derive(Debug, Clone, Copy)]
enum ComponentType {
    Tool,
    Resource,
    Prompt,
}

impl ComponentType {
    /// Detect component type from ID prefix
    fn from_id(id: &str) -> Result<Self, ApiError> {
        match id.to_uppercase().as_str() {
            s if s.starts_with("CSTOOL") => Ok(Self::Tool),
            s if s.starts_with("SRES") => Ok(Self::Resource),
            s if s.starts_with("SPMT") => Ok(Self::Prompt),
            _ => Err(ApiError::NotFound(format!("Unknown component type for ID: {}", id))),
        }
    }

    /// Get component type name as string
    fn as_str(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Resource => "resource",
            Self::Prompt => "prompt",
        }
    }
}

/// List prompts in a configuration suit (standardized version)
///
/// **Endpoint:** `GET /mcp/suits/prompts/list?suit_id={suit_id}&enabled_only={bool}`
pub async fn prompts_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitComponentListReq>,
) -> Result<Json<SuitPromptsListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify suit exists
    let suit = get_suit_or_error(&db, &request.suit_id).await?;

    // Get prompts in the suit
    let prompt_configs = crate::config::suit::get_prompts_for_config_suit(&db.pool, &request.suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get suit prompts: {e}")))?;

    // Convert to response format
    let mut prompts = Vec::new();
    for config in prompt_configs {
        let allowed_operations: Vec<String> = allowed_ops(config.enabled);

        prompts.push(SuitPromptData {
            id: config.id.unwrap_or_default(),
            server_id: config.server_id.clone(),
            server_name: config.server_name.clone(),
            prompt_name: config.prompt_name.clone(),
            enabled: config.enabled,
            allowed_operations,
        });
    }

    // Apply enabled filter if requested
    if request.enabled_only.unwrap_or(false) {
        prompts.retain(|p| p.enabled);
    }

    let total = prompts.len();
    let response = SuitPromptsListData {
        suit_id: request.suit_id,
        suit_name: suit.name,
        prompts,
        total,
    };

    Ok(Json(SuitPromptsListResp::success(response)))
}

/// List resources in a configuration suit (standardized version)
///
/// **Endpoint:** `GET /mcp/suits/resources/list?suit_id={suit_id}&enabled_only={bool}`
pub async fn resources_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitComponentListReq>,
) -> Result<Json<SuitResourcesListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify suit exists
    let suit = get_suit_or_error(&db, &request.suit_id).await?;

    // Get resources in the suit
    let resource_configs = crate::config::suit::get_resources_for_config_suit(&db.pool, &request.suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get suit resources: {e}")))?;

    // Convert to response format
    let mut resources = Vec::new();
    for config in resource_configs {
        let allowed_operations: Vec<String> = allowed_ops(config.enabled);

        resources.push(SuitResourceData {
            id: config.id.unwrap_or_default(),
            server_id: config.server_id.clone(),
            server_name: config.server_name.clone(),
            resource_uri: config.resource_uri.clone(),
            enabled: config.enabled,
            allowed_operations,
        });
    }

    // Apply enabled filter if requested
    if request.enabled_only.unwrap_or(false) {
        resources.retain(|r| r.enabled);
    }

    let total = resources.len();
    let response = SuitResourcesListData {
        suit_id: request.suit_id,
        suit_name: suit.name,
        resources,
        total,
    };

    Ok(Json(SuitResourcesListResp::success(response)))
}

/// List tools in a configuration suit (standardized version)
///
/// **Endpoint:** `GET /mcp/suits/tools/list?suit_id={suit_id}&enabled_only={bool}`
pub async fn tools_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<SuitComponentListReq>,
) -> Result<Json<SuitToolsListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify suit exists
    let suit = get_suit_or_error(&db, &request.suit_id).await?;

    // Get tools in the suit
    let tool_configs = crate::config::suit::get_config_suit_tools(&db.pool, &request.suit_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get suit tools: {e}")))?;

    // Convert to response format
    let mut tools = Vec::new();
    for tool_config in tool_configs {
        // Get server details to include server name
        if let Ok(Some(server)) = crate::config::server::get_server_by_id(&db.pool, &tool_config.server_id).await {
            tools.push(SuitToolData {
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
    let response = SuitToolsListData {
        suit_id: request.suit_id,
        suit_name: suit.name,
        tools,
        total,
    };

    Ok(Json(SuitToolsListResp::success(response)))
}

/// Manage capability operations (enable/disable tools, resources, prompts)
/// Supports both single and batch operations for enhanced performance
///
/// **Endpoint:** `POST /mcp/suits/tools/manage`, `POST /mcp/suits/resources/manage`, `POST /mcp/suits/prompts/manage`
pub async fn component_manage(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SuitComponentManageReq>,
) -> Result<Json<SuitServerManageResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify suit exists
    let _suit = get_suit_or_error(&db, &request.suit_id).await?;

    // Validate component IDs
    validate_component_ids(&request)?;
    let enabled = matches!(request.action, SuitComponentAction::Enable);

    // Execute unified operations (single or batch)
    execute_unified_operations(&state, &request, enabled).await
}

/// Validate component IDs from request
fn validate_component_ids(request: &SuitComponentManageReq) -> Result<(), ApiError> {
    if request.component_ids.is_empty() {
        Err(ApiError::BadRequest("component_ids cannot be empty".to_string()))
    } else {
        Ok(())
    }
}

/// Execute unified operations (single or batch) with transaction support
async fn execute_unified_operations(
    state: &Arc<AppState>,
    request: &SuitComponentManageReq,
    enabled: bool,
) -> Result<Json<SuitServerManageResp>, ApiError> {
    let db = get_database(state).await?;
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failed_count = 0;

    // Start transaction for all operations
    let mut tx = db
        .pool
        .begin()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to begin transaction: {e}")))?;

    // Process each component
    for component_id in &request.component_ids {
        let result = process_single_component_in_tx(&mut tx, &request.suit_id, component_id, enabled).await;

        match result {
            Ok(component_type) => {
                success_count += 1;
                results.push(ComponentOperationResult {
                    component_id: component_id.clone(),
                    component_type: component_type.as_str().to_string(),
                    success: true,
                    result: if enabled { "enabled" } else { "disabled" }.to_string(),
                    error: None,
                });
            }
            Err(e) => {
                failed_count += 1;
                results.push(ComponentOperationResult {
                    component_id: component_id.clone(),
                    component_type: "unknown".to_string(),
                    success: false,
                    result: "failed".to_string(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Commit transaction if any operations succeeded
    if success_count > 0 {
        tx.commit()
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to commit transaction: {e}")))?;

        // Invalidate cache after successful operations
        invalidate_suit_cache(state).await;
    } else {
        // Rollback if all operations failed
        tx.rollback()
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to rollback transaction: {e}")))?;
    }

    // Build unified response
    let response = SuitServerManageData {
        suit_id: request.suit_id.clone(),
        results,
        summary: format!("{} succeeded, {} failed", success_count, failed_count),
        status: if failed_count == 0 { "completed" } else { "partial" }.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(SuitServerManageResp::success(response)))
}

/// Process a single component within a transaction
async fn process_single_component_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    suit_id: &str,
    component_id: &str,
    enabled: bool,
) -> Result<ComponentType, ApiError> {
    let component_type = ComponentType::from_id(component_id)?;

    // Execute the appropriate management action within the transaction
    match component_type {
        ComponentType::Tool => {
            sqlx::query("UPDATE config_suit_tool SET enabled = ? WHERE id = ?")
                .bind(enabled)
                .bind(component_id)
                .execute(&mut **tx)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to update tool status: {e}")))?;
        }
        ComponentType::Resource => {
            // For resources, we need to get details first
            let resource = sqlx::query_as::<_, (String, String, String)>(
                "SELECT server_id, resource_uri, id FROM config_suit_resource WHERE id = ?",
            )
            .bind(component_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to get resource: {e}")))?
            .ok_or_else(|| ApiError::NotFound("Resource not found".to_string()))?;

            sqlx::query("UPDATE config_suit_resource SET enabled = ? WHERE config_suit_id = ? AND server_id = ? AND resource_uri = ?")
                .bind(enabled)
                .bind(suit_id)
                .bind(&resource.0)
                .bind(&resource.1)
                .execute(&mut **tx)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to update resource status: {e}")))?;
        }
        ComponentType::Prompt => {
            sqlx::query("UPDATE config_suit_prompt SET enabled = ? WHERE id = ?")
                .bind(enabled)
                .bind(component_id)
                .execute(&mut **tx)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to update prompt status: {e}")))?;
        }
    }

    Ok(component_type)
}

/// Invalidate suit cache if merge service is available
async fn invalidate_suit_cache(state: &Arc<AppState>) {
    if let Some(merge_service) = &state.suit_merge_service {
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated suit service cache to sync capability changes");
    }
}

// Small helpers to reduce duplication
fn allowed_ops(enabled: bool) -> Vec<String> {
    vec![if enabled { "disable" } else { "enable" }.to_string()]
}
