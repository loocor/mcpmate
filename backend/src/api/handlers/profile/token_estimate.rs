use std::{collections::HashSet, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
};

use super::{common::*, unified_capability_query::query_unified_capabilities};
use crate::{
    api::{
        handlers::{ApiError, server::common::InspectParams},
        models::token_estimate::{CapTypeEstimate, TokenBreakdownResponse, TokenEstimateQuery, TokenEstimateResponse},
    },
    core::{
        capability::{CapabilityItem, CapabilityType},
        token_estimate,
    },
};

pub async fn token_estimate(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TokenEstimateQuery>,
) -> Result<Json<TokenEstimateResponse>, ApiError> {
    let profile_id = params.profile_id;

    let db = get_database(&state).await?;
    let unified_query = state
        .unified_query
        .clone()
        .ok_or_else(|| ApiError::InternalError("Unified capability query is unavailable".to_string()))?;

    let profile = crate::config::profile::get_profile(&db.pool, &profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile: {e}")))?;

    let Some(_) = profile else {
        return Err(ApiError::NotFound(format!("Profile '{profile_id}' not found")));
    };

    let profile_servers = crate::config::profile::get_profile_servers(&db.pool, &profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile servers: {e}")))?;

    let enabled_tools = get_enabled_tools_for_profile(&db.pool, &profile_id).await?;
    let enabled_prompts = get_enabled_prompts_for_profile(&db.pool, &profile_id).await?;
    let enabled_resources = get_enabled_resources_for_profile(&db.pool, &profile_id).await?;
    let enabled_templates = get_enabled_resource_templates_for_profile(&db.pool, &profile_id).await?;

    let params = InspectParams::default();

    let mut tools_estimate = CapTypeEstimate::default();
    let mut prompts_estimate = CapTypeEstimate::default();
    let mut resources_estimate = CapTypeEstimate::default();
    let mut templates_estimate = CapTypeEstimate::default();

    for profile_server in &profile_servers {
        let server_id = &profile_server.server_id;

        accumulate_estimate(
            query_unified_capabilities(&unified_query, server_id, CapabilityType::Tools, &params).await,
            &mut tools_estimate,
            profile_server.enabled,
            |item| match item {
                CapabilityItem::Tool(tool) => Some((server_id.clone(), tool.name.clone())),
                _ => None,
            },
            &enabled_tools,
        );

        accumulate_estimate(
            query_unified_capabilities(&unified_query, server_id, CapabilityType::Prompts, &params).await,
            &mut prompts_estimate,
            profile_server.enabled,
            |item| match item {
                CapabilityItem::Prompt(prompt) => Some((server_id.clone(), prompt.name.clone())),
                _ => None,
            },
            &enabled_prompts,
        );

        accumulate_estimate(
            query_unified_capabilities(&unified_query, server_id, CapabilityType::Resources, &params).await,
            &mut resources_estimate,
            profile_server.enabled,
            |item| match item {
                CapabilityItem::Resource(resource) => Some((server_id.clone(), resource.uri.clone())),
                _ => None,
            },
            &enabled_resources,
        );

        accumulate_estimate(
            query_unified_capabilities(&unified_query, server_id, CapabilityType::ResourceTemplates, &params).await,
            &mut templates_estimate,
            profile_server.enabled,
            |item| match item {
                CapabilityItem::ResourceTemplate(template) => Some((server_id.clone(), template.uri_template.clone())),
                _ => None,
            },
            &enabled_templates,
        );
    }

    finalize_estimate(&mut tools_estimate);
    finalize_estimate(&mut prompts_estimate);
    finalize_estimate(&mut resources_estimate);
    finalize_estimate(&mut templates_estimate);

    let total_available_tokens = tools_estimate.available_tokens
        + prompts_estimate.available_tokens
        + resources_estimate.available_tokens
        + templates_estimate.available_tokens;

    let visible_tokens = tools_estimate.visible_tokens
        + prompts_estimate.visible_tokens
        + resources_estimate.visible_tokens
        + templates_estimate.visible_tokens;

    let savings_tokens = tools_estimate.savings_tokens
        + prompts_estimate.savings_tokens
        + resources_estimate.savings_tokens
        + templates_estimate.savings_tokens;

    let response = TokenEstimateResponse {
        total_available_tokens,
        visible_tokens,
        savings_tokens,
        breakdown: TokenBreakdownResponse {
            tools: tools_estimate,
            prompts: prompts_estimate,
            resources: resources_estimate,
            templates: templates_estimate,
        },
        estimation_method: "chars_div_4".to_string(),
        approximate: true,
    };

    Ok(Json(response))
}

fn accumulate_estimate<F>(
    items: Option<Vec<CapabilityItem>>,
    estimate: &mut CapTypeEstimate,
    server_enabled: bool,
    key_fn: F,
    enabled_keys: &HashSet<(String, String)>,
) where
    F: Fn(&CapabilityItem) -> Option<(String, String)>,
{
    let Some(items) = items else {
        return;
    };

    for item in items {
        let Some(key) = key_fn(&item) else {
            continue;
        };

        let tokens = estimate_item_tokens(&item);
        estimate.available_count += 1;
        estimate.available_tokens += tokens;

        if server_enabled && enabled_keys.contains(&key) {
            estimate.visible_count += 1;
            estimate.visible_tokens += tokens;
        }
    }
}

fn finalize_estimate(estimate: &mut CapTypeEstimate) {
    estimate.disabled_count = estimate.available_count.saturating_sub(estimate.visible_count);
    estimate.savings_tokens = estimate.available_tokens.saturating_sub(estimate.visible_tokens);
}

fn estimate_item_tokens(item: &CapabilityItem) -> u32 {
    serde_json::to_string(item)
        .map(|json| token_estimate::estimate_capability_tokens(&json))
        .unwrap_or(0)
}

async fn get_enabled_tools_for_profile(
    pool: &sqlx::SqlitePool,
    profile_id: &str,
) -> Result<HashSet<(String, String)>, ApiError> {
    let tools = crate::config::profile::get_profile_tools(pool, profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile tools: {e}")))?;

    Ok(tools
        .into_iter()
        .filter(|tool| tool.enabled)
        .map(|tool| (tool.server_id, tool.tool_name))
        .collect())
}

async fn get_enabled_prompts_for_profile(
    pool: &sqlx::SqlitePool,
    profile_id: &str,
) -> Result<HashSet<(String, String)>, ApiError> {
    let prompts = crate::config::profile::get_enabled_prompts_for_profile(pool, profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get enabled prompts: {e}")))?;

    Ok(prompts
        .into_iter()
        .map(|prompt| (prompt.server_id, prompt.prompt_name))
        .collect())
}

async fn get_enabled_resources_for_profile(
    pool: &sqlx::SqlitePool,
    profile_id: &str,
) -> Result<HashSet<(String, String)>, ApiError> {
    let resources = crate::config::profile::get_enabled_resources_for_profile(pool, profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get enabled resources: {e}")))?;

    Ok(resources
        .into_iter()
        .map(|resource| (resource.server_id, resource.resource_uri))
        .collect())
}

async fn get_enabled_resource_templates_for_profile(
    pool: &sqlx::SqlitePool,
    profile_id: &str,
) -> Result<HashSet<(String, String)>, ApiError> {
    let templates = crate::config::profile::get_enabled_resource_templates_for_profile(pool, profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get enabled resource templates: {e}")))?;

    Ok(templates
        .into_iter()
        .map(|template| (template.server_id, template.resource_uri))
        .collect())
}
