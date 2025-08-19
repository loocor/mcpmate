// Server prompts handlers
// Provides handlers for server prompt inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{handlers::ApiError, routes::AppState};
use chrono::Utc;

use super::capability::{
    CapabilityKind, enrich_capability_items, prompt_json, prompt_json_from_cached, respond_with_enriched,
};
use super::common::{
    InspectQuery, create_inspect_response, create_runtime_cache_data, get_database_from_state, validate_server_id,
};

/// Convert database error to ApiError
#[inline]
fn db_error(e: impl std::fmt::Display) -> ApiError {
    ApiError::InternalError(format!("Database error: {e}"))
}

/// List all prompts for a specific server
///
/// Strategy order:
/// 1) Cache-first: query Redb snapshot with freshness policy.
/// 2) Runtime fallback: aggregate via connected instances (proxy service).
/// 3) Force refresh (if requested): create a temporary instance to fetch data.
/// 4) Offline cache: return any cached copy ignoring freshness.
/// 5) None: return empty.
///
/// Supports both `server_name` and `server_id` as identifier.
#[tracing::instrument(skip(state))]
pub async fn list_prompts(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<InspectQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get database and load server by ID
    let db = get_database_from_state(&state)?;
    let server_row = crate::config::server::get_server_by_id(&db.pool, &id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{id}' not found")))?;
    let server_info = super::common::ServerIdentification {
        server_id: id.clone(),
        server_name: server_row.name.clone(),
    };

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Short-circuit only if the server explicitly declares capabilities and lacks prompts capability
    if let Ok((server_row, _id)) = super::common::get_server_by_identifier(&db.pool, &server_info.server_name).await {
        if server_row.capabilities.is_some()
            && !server_row.has_capability(crate::common::capability::CapabilityToken::Prompts)
        {
            return Ok(create_inspect_response(
                Vec::new(),
                false,
                params.refresh,
                "capability-prompts-unsupported",
            ));
        }
    }

    // Try Redb cache with freshness on full snapshot
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);
    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> = data.prompts.into_iter().map(prompt_json_from_cached).collect();
                if !processed.is_empty() {
                    if let Ok(db) = get_database_from_state(&state) {
                        let enriched = enrich_capability_items(
                            CapabilityKind::Prompts,
                            &db.pool,
                            &server_info.server_id,
                            processed,
                        )
                        .await;
                        return Ok(respond_with_enriched(
                            enriched,
                            true,
                            params.refresh,
                            crate::common::constants::strategies::CACHE,
                        ));
                    }
                    return Ok(create_inspect_response(
                        processed,
                        true,
                        params.refresh,
                        crate::common::constants::strategies::CACHE,
                    ));
                }
            }
        }
    }

    // Runtime fallback: aggregate prompts via protocol helper
    if let Ok(pool) = tokio::time::timeout(
        std::time::Duration::from_millis(crate::common::constants::timeouts::LOCK_MS),
        state.connection_pool.lock(),
    )
    .await
    {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            let connected_instances: Vec<_> = instances
                .values()
                .filter(|conn| conn.is_connected() && conn.supports_prompts())
                .collect();

            let mut prompts = Vec::new();
            let mut cached_prompts = Vec::new();

            for conn in connected_instances {
                if let Some(service) = &conn.service {
                    if let Ok(result) = service.list_prompts(None).await {
                        let now = Utc::now();
                        for p in result.prompts {
                            let prompt_args = p.arguments.unwrap_or_default();
                            let converted_args: Vec<crate::core::cache::PromptArgument> = prompt_args
                                .into_iter()
                                .map(|arg| crate::core::cache::PromptArgument {
                                    name: arg.name,
                                    description: arg.description,
                                    required: arg.required.unwrap_or(false),
                                })
                                .collect();

                            prompts.push(prompt_json(
                                &p.name,
                                p.description.clone(),
                                converted_args.clone(),
                                None,
                                None,
                            ));
                            cached_prompts.push(crate::core::cache::CachedPromptInfo {
                                name: p.name,
                                description: p.description,
                                arguments: converted_args,
                                enabled: true,
                                cached_at: now,
                            });
                        }
                    }
                }
            }

            if !prompts.is_empty() {
                let server_data =
                    create_runtime_cache_data(&server_info, Vec::new(), Vec::new(), cached_prompts, Vec::new());
                let _ = state.redb_cache.store_server_data(&server_data).await;

                if let Ok(db) = get_database_from_state(&state) {
                    let enriched =
                        enrich_capability_items(CapabilityKind::Prompts, &db.pool, &server_info.server_id, prompts)
                            .await;
                    return Ok(respond_with_enriched(
                        enriched,
                        false,
                        params.refresh,
                        crate::common::constants::strategies::RUNTIME,
                    ));
                }
                return Ok(create_inspect_response(
                    prompts,
                    false,
                    params.refresh,
                    crate::common::constants::strategies::RUNTIME,
                ));
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::capability::create_temporary_instance_for_capability(
        &state,
        &server_info,
        &params,
        super::capability::CapabilityType::Prompts,
    )
    .await?
    {
        return Ok(response);
    }

    // Last resort: offline cache
    if let Ok(cached) = state.redb_cache.get_server_prompts(&server_info.server_id, false).await {
        if !cached.is_empty() {
            let processed: Vec<serde_json::Value> = cached.into_iter().map(prompt_json_from_cached).collect();
            if let Ok(db) = get_database_from_state(&state) {
                let enriched =
                    enrich_capability_items(CapabilityKind::Prompts, &db.pool, &server_info.server_id, processed).await;
                return Ok(respond_with_enriched(
                    enriched,
                    true,
                    params.refresh,
                    crate::common::constants::strategies::CACHE,
                ));
            }
            return Ok(create_inspect_response(
                processed,
                true,
                params.refresh,
                crate::common::constants::strategies::CACHE,
            ));
        }
    }

    // Return empty result if no data available from cache or runtime
    Ok(create_inspect_response(
        Vec::new(),
        false,
        params.refresh,
        crate::common::constants::strategies::NONE,
    ))
}

/// Get detailed prompt argument information
///
/// Returns detailed information about prompt arguments for form generation
/// and validation purposes.
///
/// Supports both `server_name` and `server_id` as identifier.
pub async fn get_prompt_arguments(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<InspectQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get database and load server by ID
    let db = get_database_from_state(&state)?;
    let server_row = crate::config::server::get_server_by_id(&db.pool, &id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| ApiError::NotFound(format!("Server with ID '{id}' not found")))?;
    let server_info = super::common::ServerIdentification {
        server_id: id.clone(),
        server_name: server_row.name.clone(),
    };

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
