// MCPMate Proxy API handlers for Profile capabilities management
// Contains handler functions for managing tools, resources, and prompts in Profile

use super::{common::*, helpers::get_profile_or_error};
use crate::api::models::profile::{
    ComponentOperationResult, ProfileComponentAction, ProfileComponentListReq, ProfileComponentManageReq,
    ProfilePromptData, ProfilePromptsListData, ProfilePromptsListResp, ProfileResourceData, ProfileResourcesListData,
    ProfileResourcesListResp, ProfileServerManageData, ProfileServerManageResp, ProfileToolData, ProfileToolsListData,
    ProfileToolsListResp,
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

/// List prompts in a profile (standardized version)
///
/// **Endpoint:** `GET /mcp/profile/prompts/list?profile_id={profile_id}&enabled_only={bool}`
pub async fn prompts_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileComponentListReq>,
) -> Result<Json<ProfilePromptsListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify profile exists
    let profile = get_profile_or_error(&db, &request.profile_id).await?;

    // Get prompts in the profile
    let prompt_configs = crate::config::profile::get_prompts_for_profile(&db.pool, &request.profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile prompts: {e}")))?;

    // Convert to response format
    let mut prompts = Vec::new();
    for config in prompt_configs {
        let allowed_operations: Vec<String> = allowed_ops(config.enabled);

        prompts.push(ProfilePromptData {
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
    let response = ProfilePromptsListData {
        profile_id: request.profile_id,
        profile_name: profile.name,
        prompts,
        total,
    };

    Ok(Json(ProfilePromptsListResp::success(response)))
}

/// List resources in a profile (standardized version)
///
/// **Endpoint:** `GET /mcp/profile/resources/list?profile_id={profile_id}&enabled_only={bool}`
pub async fn resources_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileComponentListReq>,
) -> Result<Json<ProfileResourcesListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify profile exists
    let profile = get_profile_or_error(&db, &request.profile_id).await?;

    // Get resources in the profile
    let resource_configs = crate::config::profile::get_resources_for_profile(&db.pool, &request.profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile resources: {e}")))?;

    // Convert to response format
    let mut resources = Vec::new();
    for config in resource_configs {
        let allowed_operations: Vec<String> = allowed_ops(config.enabled);

        resources.push(ProfileResourceData {
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
    let response = ProfileResourcesListData {
        profile_id: request.profile_id,
        profile_name: profile.name,
        resources,
        total,
    };

    Ok(Json(ProfileResourcesListResp::success(response)))
}

/// List tools in a profile (standardized version)
///
/// **Endpoint:** `GET /mcp/profile/tools/list?profile_id={profile_id}&enabled_only={bool}`
pub async fn tools_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileComponentListReq>,
) -> Result<Json<ProfileToolsListResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify profile exists
    let profile = get_profile_or_error(&db, &request.profile_id).await?;

    // Get tools in the profile
    let tool_configs = crate::config::profile::get_profile_tools(&db.pool, &request.profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile tools: {e}")))?;

    // Convert to response format
    let mut tools = Vec::new();
    for tool_config in tool_configs {
        // Get server details to include server name
        if let Ok(Some(server)) = crate::config::server::get_server_by_id(&db.pool, &tool_config.server_id).await {
            tools.push(ProfileToolData {
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
    let response = ProfileToolsListData {
        profile_id: request.profile_id,
        profile_name: profile.name,
        tools,
        total,
    };

    Ok(Json(ProfileToolsListResp::success(response)))
}

/// Manage capability operations (enable/disable tools, resources, prompts)
/// Supports both single and batch operations for enhanced performance
///
/// **Endpoint:** `POST /mcp/profile/tools/manage`, `POST /mcp/profile/resources/manage`, `POST /mcp/profile/prompts/manage`
pub async fn component_manage(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileComponentManageReq>,
) -> Result<Json<ProfileServerManageResp>, ApiError> {
    let db = get_database(&state).await?;

    // Verify profile exists
    let _profile = get_profile_or_error(&db, &request.profile_id).await?;

    // Validate component IDs
    validate_component_ids(&request)?;
    let enabled = matches!(request.action, ProfileComponentAction::Enable);

    // Execute unified operations (single or batch)
    execute_unified_operations(&state, &request, enabled).await
}

/// Validate component IDs from request
fn validate_component_ids(request: &ProfileComponentManageReq) -> Result<(), ApiError> {
    if request.component_ids.is_empty() {
        Err(ApiError::BadRequest("component_ids cannot be empty".to_string()))
    } else {
        Ok(())
    }
}

/// Execute unified operations (single or batch) with transaction support
async fn execute_unified_operations(
    state: &Arc<AppState>,
    request: &ProfileComponentManageReq,
    enabled: bool,
) -> Result<Json<ProfileServerManageResp>, ApiError> {
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
        let result = process_single_component_in_tx(&mut tx, &request.profile_id, component_id, enabled).await;

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
        invalidate_profile_cache(state).await;
    } else {
        // Rollback if all operations failed
        tx.rollback()
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to rollback transaction: {e}")))?;
    }

    // Build unified response
    let response = ProfileServerManageData {
        profile_id: request.profile_id.clone(),
        results,
        summary: format!("{} succeeded, {} failed", success_count, failed_count),
        status: if failed_count == 0 { "completed" } else { "partial" }.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(ProfileServerManageResp::success(response)))
}

/// Process a single component within a transaction
async fn process_single_component_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    profile_id: &str,
    component_id: &str,
    enabled: bool,
) -> Result<ComponentType, ApiError> {
    let component_type = ComponentType::from_id(component_id)?;

    // Execute the appropriate management action within the transaction
    match component_type {
        ComponentType::Tool => {
            sqlx::query("UPDATE profile_tool SET enabled = ? WHERE id = ?")
                .bind(enabled)
                .bind(component_id)
                .execute(&mut **tx)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to update tool status: {e}")))?;

            // Publish MCP-facing event so downstream clients receive tools/list_changed
            if let Ok(Some(tool_name)) = sqlx::query_scalar::<_, String>(
                r#"
                SELECT st.tool_name
                FROM profile_tool cst
                JOIN server_tools st ON cst.server_tool_id = st.id
                WHERE cst.id = ?
                "#,
            )
            .bind(component_id)
            .fetch_optional(&mut **tx)
            .await
            {
                crate::core::events::EventBus::global().publish(
                    crate::core::events::Event::ToolEnabledInProfileChanged {
                        tool_id: component_id.to_string(),
                        tool_name,
                        profile_id: profile_id.to_string(),
                        enabled,
                    },
                );
            }
        }
        ComponentType::Resource => {
            // For resources, we need to get details first
            let resource = sqlx::query_as::<_, (String, String, String)>(
                "SELECT server_id, resource_uri, id FROM profile_resource WHERE id = ?",
            )
            .bind(component_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to get resource: {e}")))?
            .ok_or_else(|| ApiError::NotFound("Resource not found".to_string()))?;

            sqlx::query(
                "UPDATE profile_resource SET enabled = ? WHERE profile_id = ? AND server_id = ? AND resource_uri = ?",
            )
            .bind(enabled)
            .bind(profile_id)
            .bind(&resource.0)
            .bind(&resource.1)
            .execute(&mut **tx)
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to update resource status: {e}")))?;

            // Publish MCP-facing event so downstream clients receive resources/list_changed
            crate::core::events::EventBus::global().publish(
                crate::core::events::Event::ResourceEnabledInProfileChanged {
                    resource_id: component_id.to_string(),
                    resource_uri: resource.1.clone(),
                    profile_id: profile_id.to_string(),
                    enabled,
                },
            );
        }
        ComponentType::Prompt => {
            sqlx::query("UPDATE profile_prompt SET enabled = ? WHERE id = ?")
                .bind(enabled)
                .bind(component_id)
                .execute(&mut **tx)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to update prompt status: {e}")))?;

            // Publish MCP-facing event so downstream clients receive prompts/list_changed
            if let Ok(Some(prompt_name)) = sqlx::query_scalar::<_, String>(
                "SELECT prompt_name FROM profile_prompt WHERE id = ?",
            )
            .bind(component_id)
            .fetch_optional(&mut **tx)
            .await
            {
                crate::core::events::EventBus::global().publish(
                    crate::core::events::Event::PromptEnabledInProfileChanged {
                        prompt_id: component_id.to_string(),
                        prompt_name,
                        profile_id: profile_id.to_string(),
                        enabled,
                    },
                );
            }
        }
    }

    Ok(component_type)
}

/// Invalidate profile cache if merge service is available
async fn invalidate_profile_cache(state: &Arc<AppState>) {
    if let Some(merge_service) = &state.profile_merge_service {
        merge_service.invalidate_cache().await;
        tracing::debug!("Invalidated profile service cache to sync capability changes");
    }
}

// Small helpers to reduce duplication
fn allowed_ops(enabled: bool) -> Vec<String> {
    vec![if enabled { "disable" } else { "enable" }.to_string()]
}
