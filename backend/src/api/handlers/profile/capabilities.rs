// MCPMate Proxy API handlers for Profile capabilities management
// Contains handler functions for managing tools, resources, and prompts in Profile

use super::{common::*, helpers::get_profile_or_error};
use crate::api::models::profile::{
    ComponentOperationResult, ProfileComponentAction, ProfileComponentListReq, ProfileComponentManageReq,
    ProfilePromptData, ProfilePromptsListData, ProfilePromptsListResp, ProfileResourceData,
    ProfileResourceTemplateData, ProfileResourceTemplatesListData, ProfileResourceTemplatesListResp,
    ProfileResourcesListData, ProfileResourcesListResp, ProfileServerManageData, ProfileServerManageResp,
    ProfileToolData, ProfileToolsListData, ProfileToolsListResp,
};
use serde_json::{Map, Value};

type CapabilityAuditDetails = Value;

// Component type enumeration for type-safe operations
#[derive(Debug, Clone, Copy)]
enum ComponentType {
    Tool,
    Resource,
    Prompt,
    ResourceTemplate,
}

impl ComponentType {
    fn from_kind(kind: &str) -> Result<Self, ApiError> {
        match kind {
            "tools" => Ok(Self::Tool),
            "resources" => Ok(Self::Resource),
            "prompts" => Ok(Self::Prompt),
            "resource_templates" => Ok(Self::ResourceTemplate),
            _ => Err(ApiError::NotFound(format!("Unknown Capability kind: {kind}"))),
        }
    }

    /// Get component type name as string
    fn as_str(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Resource => "resource",
            Self::Prompt => "prompt",
            Self::ResourceTemplate => "resource_template",
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
    let mut prompts = Vec::new();
    for config in prompt_configs {
        let allowed_operations: Vec<String> = allowed_ops(config.enabled);
        prompts.push(ProfilePromptData {
            ref_id: config.id.unwrap_or_default(),
            server_id: config.server_id.clone(),
            server_name: config.server_name.clone(),
            prompt_name: config.prompt_name.clone(),
            unique_name: config.unique_name,
            description: config.description,
            enabled: config.enabled,
            state: config.state,
            state_generation: config.state_generation,
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
        source_revision_set: crate::core::capability::materializer::SurfaceAuthoringLoader::load_catalog_revision_set(
            &db.pool,
        )
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?
        .into_iter()
        .collect(),
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
    let mut resources = Vec::new();
    for config in resource_configs {
        let allowed_operations: Vec<String> = allowed_ops(config.enabled);
        resources.push(ProfileResourceData {
            ref_id: config.id.unwrap_or_default(),
            server_id: config.server_id.clone(),
            server_name: config.server_name.clone(),
            resource_uri: config.resource_uri.clone(),
            unique_uri: config.unique_uri,
            description: config.description,
            enabled: config.enabled,
            state: config.state,
            state_generation: config.state_generation,
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
        source_revision_set: crate::core::capability::materializer::SurfaceAuthoringLoader::load_catalog_revision_set(
            &db.pool,
        )
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?
        .into_iter()
        .collect(),
    };

    Ok(Json(ProfileResourcesListResp::success(response)))
}

/// List resource templates in a profile (standardized version)
///
/// **Endpoint:** `GET /mcp/profile/resource-templates/list?profile_id={profile_id}&enabled_only={bool}`
pub async fn resource_templates_list(
    State(state): State<Arc<AppState>>,
    Query(request): Query<ProfileComponentListReq>,
) -> Result<Json<ProfileResourceTemplatesListResp>, ApiError> {
    let db = get_database(&state).await?;

    let profile = get_profile_or_error(&db, &request.profile_id).await?;

    let template_configs = crate::config::profile::get_resource_templates_for_profile(&db.pool, &request.profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile resource templates: {e}")))?;

    let mut templates = Vec::new();
    for config in template_configs {
        let allowed_operations: Vec<String> = allowed_ops(config.enabled);
        templates.push(ProfileResourceTemplateData {
            ref_id: config.id.unwrap_or_default(),
            server_id: config.server_id.clone(),
            server_name: config.server_name.clone(),
            uri_template: config.resource_uri.clone(),
            unique_uri_template: config.unique_uri,
            description: config.description,
            enabled: config.enabled,
            state: config.state,
            state_generation: config.state_generation,
            allowed_operations,
        });
    }

    if request.enabled_only.unwrap_or(false) {
        templates.retain(|t| t.enabled);
    }

    let total = templates.len();
    let response = ProfileResourceTemplatesListData {
        profile_id: request.profile_id,
        profile_name: profile.name,
        templates,
        total,
        source_revision_set: crate::core::capability::materializer::SurfaceAuthoringLoader::load_catalog_revision_set(
            &db.pool,
        )
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?
        .into_iter()
        .collect(),
    };

    Ok(Json(ProfileResourceTemplatesListResp::success(response)))
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
                ref_id: tool_config.ref_id.clone(),
                server_id: tool_config.server_id.clone(),
                server_name: server.name,
                tool_name: tool_config.tool_name.clone(),
                unique_name: tool_config.unique_name.clone(),
                description: tool_config.description,
                enabled: tool_config.enabled,
                state: tool_config.state,
                state_generation: tool_config.state_generation,
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
        source_revision_set: crate::core::capability::materializer::SurfaceAuthoringLoader::load_catalog_revision_set(
            &db.pool,
        )
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?
        .into_iter()
        .collect(),
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
    let started_at = std::time::Instant::now();
    let db = get_database(&state).await?;

    // Verify profile exists
    let profile = get_profile_or_error(&db, &request.profile_id).await?;

    // Validate component IDs
    validate_component_ids(&request)?;
    let enabled = matches!(request.action, ProfileComponentAction::Enable);

    let capability_details = collect_capability_audit_details(&db, &request.component_ids).await;
    let audit_server_id = extract_single_component_string(&capability_details, "server_id");

    // Execute unified operations (single or batch)
    let result = execute_unified_operations(&state, &request).await;
    let mut data = Map::new();
    data.insert(
        "component_count".to_string(),
        Value::from(request.component_ids.len() as u64),
    );
    data.insert("profile_name".to_string(), Value::String(profile.name.clone()));
    data.insert(
        "component_action".to_string(),
        Value::String(if enabled { "enable" } else { "disable" }.to_string()),
    );
    data.insert("components".to_string(), Value::Array(capability_details));
    crate::audit::interceptor::emit_event(
        state.audit_service.as_ref(),
        crate::audit::interceptor::build_rest_event(
            if enabled {
                crate::audit::AuditAction::CapabilityGrant
            } else {
                crate::audit::AuditAction::CapabilityRevoke
            },
            if result.is_ok() {
                crate::audit::AuditStatus::Success
            } else {
                crate::audit::AuditStatus::Failed
            },
            "POST",
            "/api/mcp/profile/components/manage",
            Some(started_at.elapsed().as_millis() as u64),
            audit_server_id,
            Some(request.profile_id.clone()),
            Some(data),
            result.as_ref().err().map(ToString::to_string),
        ),
    )
    .await;
    result
}

async fn collect_capability_audit_details(
    db: &crate::config::database::Database,
    component_ids: &[String],
) -> Vec<CapabilityAuditDetails> {
    let mut details = Vec::new();

    for component_id in component_ids {
        let row = sqlx::query_as::<_, (String, String, String, String)>(
            r#"
            SELECT cr.kind, cr.server_id, sc.name, cr.origin_key
            FROM capability_refs cr
            JOIN server_config sc ON sc.id = cr.server_id
            WHERE cr.ref_id = ?
            "#,
        )
        .bind(component_id)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten();
        let Some((kind, server_id, server_name, origin_key)) = row else {
            details.push(serde_json::json!({
                "ref_id": component_id,
                "component_type": "unknown",
            }));
            continue;
        };
        let Ok(component_type) = ComponentType::from_kind(&kind) else {
            details.push(serde_json::json!({
                "ref_id": component_id,
                "component_type": "unknown",
            }));
            continue;
        };
        details.push(serde_json::json!({
            "ref_id": component_id,
            "component_type": component_type.as_str(),
            "server_id": server_id,
            "server_name": server_name,
            "origin_key": origin_key,
        }));
    }

    details
}

fn extract_single_component_string(
    details: &[CapabilityAuditDetails],
    key: &str,
) -> Option<String> {
    if details.len() != 1 {
        return None;
    }

    details
        .first()
        .and_then(|detail| detail.as_object())
        .and_then(|detail| detail.get(key))
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
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
) -> Result<Json<ProfileServerManageResp>, ApiError> {
    let db = get_database(state).await?;
    let action = match request.action {
        ProfileComponentAction::Enable => crate::core::capability::management::ProfileRelationshipAction::Enable,
        ProfileComponentAction::Disable => crate::core::capability::management::ProfileRelationshipAction::Disable,
        ProfileComponentAction::Remove => crate::core::capability::management::ProfileRelationshipAction::Remove,
        ProfileComponentAction::Replace => {
            return Err(ApiError::BadRequest(
                "Replace is only supported for profile server selections".to_string(),
            ));
        }
    };
    let management = crate::core::capability::management::ProfileSurfaceManagement::mutate_capabilities(
        &db.pool,
        &request.profile_id,
        &request.component_ids,
        action,
        request.source_revision_set.clone().into_iter().collect(),
        "profile_management",
    )
    .await
    .map_err(map_catalog_error)?;
    invalidate_profile_cache(state).await;
    super::emit_surface_publication_audits(
        state,
        "profile_management",
        Some(&request.profile_id),
        "/api/mcp/profile/capabilities/manage",
        management.materializations,
    )
    .await;

    let result_label = match request.action {
        ProfileComponentAction::Enable => "enabled",
        ProfileComponentAction::Disable => "disabled",
        ProfileComponentAction::Remove => "removed",
        ProfileComponentAction::Replace => unreachable!(),
    };
    let results = request
        .component_ids
        .iter()
        .zip(management.mutations)
        .map(|(component_id, mutation)| ComponentOperationResult {
            component_id: component_id.clone(),
            component_type: ComponentType::from_kind(mutation.kind.as_str())
                .expect("validated capability kind")
                .as_str()
                .to_string(),
            success: true,
            result: result_label.to_string(),
            error: None,
        })
        .collect();
    let response = ProfileServerManageData {
        profile_id: request.profile_id.clone(),
        results,
        summary: format!("{} succeeded, 0 failed", request.component_ids.len()),
        status: "completed".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(ProfileServerManageResp::success(response)))
}

fn map_catalog_error(error: mcpmate_capability_store::CatalogError) -> ApiError {
    match error {
        mcpmate_capability_store::CatalogError::ConcurrencyConflict { .. } => ApiError::Conflict(error.to_string()),
        _ => ApiError::InternalError(error.to_string()),
    }
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
