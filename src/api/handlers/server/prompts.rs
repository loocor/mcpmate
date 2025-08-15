// Server prompts handlers
// Provides handlers for server prompt inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{handlers::ApiError, routes::AppState};
use chrono::Utc;

use super::common::{
    InspectQuery, create_inspect_response, create_runtime_cache_data, get_database_from_state,
    resolve_server_identifier, validate_server_id,
};

/// List all prompts for a specific server
///
/// Returns a list of prompts available on the specified server with
/// configurable filtering and formatting options.
///
/// Supports both server_name and server_id as identifier.
pub async fn list_prompts(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<InspectQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Try Redb cache with freshness on full snapshot
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);
    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> = data
                    .prompts
                    .into_iter()
                    .map(|p| {
                        serde_json::json!({
                            "name": p.name,
                            "description": p.description,
                            "arguments": p.arguments,
                        })
                    })
                    .collect();
                if !processed.is_empty() {
                    return Ok(create_inspect_response(processed, true, params.refresh, "cache"));
                }
            }
        }
    }

    // Runtime fallback: aggregate prompts via protocol helper
    if let Ok(pool) = tokio::time::timeout(std::time::Duration::from_millis(500), state.connection_pool.lock()).await {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            let mut prompts: Vec<serde_json::Value> = Vec::new();
            let mut cached_prompts: Vec<crate::core::cache::CachedPromptInfo> = Vec::new();

            for conn in instances.values() {
                if !conn.is_connected() || !conn.supports_prompts() {
                    continue;
                }
                if let Some(service) = &conn.service {
                    if let Ok(result) = service.list_prompts(None).await {
                        for p in result.prompts {
                            prompts.push(serde_json::json!({
                                "name": p.name,
                                "description": p.description,
                                "arguments": p.arguments.clone().unwrap_or_default(),
                            }));
                            let converted_args = p
                                .arguments
                                .unwrap_or_default()
                                .into_iter()
                                .map(|arg| crate::core::cache::PromptArgument {
                                    name: arg.name,
                                    description: arg.description,
                                    required: arg.required.unwrap_or(false),
                                })
                                .collect();
                            cached_prompts.push(crate::core::cache::CachedPromptInfo {
                                name: p.name,
                                description: p.description,
                                arguments: converted_args,
                                enabled: true,
                                cached_at: Utc::now(),
                            });
                        }
                    }
                }
            }

            if !prompts.is_empty() {
                let server_data =
                    create_runtime_cache_data(&server_info, Vec::new(), Vec::new(), cached_prompts, Vec::new());
                let _ = state.redb_cache.store_server_data(&server_data).await;

                return Ok(create_inspect_response(prompts, false, params.refresh, "runtime"));
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::common::create_temporary_instance_for_capability(
        &state,
        &server_info,
        &params,
        super::common::CapabilityType::Prompts,
    )
    .await?
    {
        return Ok(response);
    }

    // Last resort: offline cache
    if let Ok(cached) = state.redb_cache.get_server_prompts(&server_info.server_id, false).await {
        if !cached.is_empty() {
            let processed: Vec<serde_json::Value> = cached
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "description": p.description,
                        "arguments": p.arguments,
                    })
                })
                .collect();
            return Ok(create_inspect_response(processed, true, params.refresh, "cache"));
        }
    }

    // Return empty result if no data available from cache or runtime
    Ok(create_inspect_response(Vec::new(), false, params.refresh, "none"))
}

/// Get detailed prompt argument information
///
/// Returns detailed information about prompt arguments for form generation
/// and validation purposes.
///
/// Supports both server_name and server_id as identifier.
pub async fn get_prompt_arguments(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<InspectQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Return empty result if no data available from cache or runtime
    Ok(Json(serde_json::json!({
        "data": [],
        "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default() }
    })))
}
