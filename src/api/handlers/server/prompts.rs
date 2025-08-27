// Server prompts handlers
// Provides handlers for server prompt inspect endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::api::{
    models::server::{
        ServerCapabilityMeta, ServerCapabilityReq, ServerPromptArgumentsData, ServerPromptArgumentsResp,
        ServerPromptsData, ServerPromptsResp,
    },
    routes::AppState,
};
use chrono::Utc;

use super::capability::{
    CapabilityKind, enrich_capability_items, prompt_json, prompt_json_from_cached, respond_with_enriched,
};
use super::common::{create_inspect_response, create_runtime_cache_data, get_database_from_state};

/// Helper function to convert Json response to ServerPromptsResp
fn json_to_server_prompts_resp(json_response: axum::Json<serde_json::Value>) -> ServerPromptsData {
    let json_value = json_response.0;

    let data = json_value
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();

    let state = json_value
        .get("state")
        .and_then(|s| s.as_str())
        .unwrap_or("ok")
        .to_string();

    let meta_value = json_value.get("meta").cloned().unwrap_or_default();
    let meta = ServerCapabilityMeta {
        cache_hit: meta_value.get("cache_hit").and_then(|v| v.as_bool()).unwrap_or(false),
        strategy: meta_value
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        source: meta_value
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
    };

    ServerPromptsData { data, state, meta }
}

/// Helper function to convert Json response to ServerPromptArgumentsResp
fn json_to_server_prompt_arguments_resp(json_response: axum::Json<serde_json::Value>) -> ServerPromptArgumentsData {
    let json_value = json_response.0;

    let data = json_value
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();

    let meta_value = json_value.get("meta").cloned().unwrap_or_default();
    let meta = ServerCapabilityMeta {
        cache_hit: meta_value.get("cache_hit").and_then(|v| v.as_bool()).unwrap_or(false),
        strategy: meta_value
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        source: meta_value
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
    };

    ServerPromptArgumentsData { data, meta }
}

/// List all prompts for a specific server with standardized signature
pub async fn server_prompts(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerPromptsResp>, StatusCode> {
    let result = server_prompts_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for listing server prompts
#[tracing::instrument(skip(app_state), level = "debug")]
async fn server_prompts_core(
    request: &ServerCapabilityReq,
    app_state: &Arc<AppState>,
) -> Result<ServerPromptsResp, StatusCode> {
    // Convert request to internal query format
    let query = super::common::InspectQuery {
        refresh: request.refresh.as_ref().map(|r| match r {
            crate::api::models::server::ServerRefreshStrategy::Auto => super::common::RefreshStrategy::CacheFirst,
            crate::api::models::server::ServerRefreshStrategy::Force => super::common::RefreshStrategy::Force,
            crate::api::models::server::ServerRefreshStrategy::Cache => super::common::RefreshStrategy::CacheFirst,
        }),
        format: None,
        include_meta: None,
        timeout: None,
    };

    // Validate and get server info using unified function
    let (db, server_info, params) = super::common::get_server_info_for_inspect(app_state, &request.id, &query).await?;

    // Check if server supports prompts capability
    if let Some(response) = super::common::check_capability_or_error(
        &db.pool,
        &server_info,
        crate::common::capability::CapabilityToken::Prompts,
        &params,
    )
    .await
    {
        let prompts_resp = json_to_server_prompts_resp(response);
        return Ok(ServerPromptsResp::success(prompts_resp));
    }

    // Try Redb cache with freshness on full snapshot
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);
    if let Ok(cache_result) = app_state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> = data.prompts.into_iter().map(prompt_json_from_cached).collect();
                if !processed.is_empty() {
                    if let Ok(db) = get_database_from_state(app_state) {
                        let enriched = enrich_capability_items(
                            CapabilityKind::Prompts,
                            &db.pool,
                            &server_info.server_id,
                            processed,
                        )
                        .await;
                        let response_data = respond_with_enriched(
                            enriched,
                            true,
                            params.refresh,
                            crate::common::constants::strategies::CACHE,
                        );
                        let prompts_resp = json_to_server_prompts_resp(response_data);
                        return Ok(ServerPromptsResp::success(prompts_resp));
                    }
                    let response_data = create_inspect_response(
                        processed,
                        true,
                        params.refresh,
                        crate::common::constants::strategies::CACHE,
                    );
                    let prompts_resp = json_to_server_prompts_resp(response_data);
                    return Ok(ServerPromptsResp::success(prompts_resp));
                }
            }
        }
    }

    // Runtime fallback: aggregate prompts via protocol helper
    if let Ok(pool) = tokio::time::timeout(
        std::time::Duration::from_millis(crate::common::constants::timeouts::LOCK_MS),
        app_state.connection_pool.lock(),
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
                let _ = app_state.redb_cache.store_server_data(&server_data).await;

                if let Ok(db) = get_database_from_state(app_state) {
                    let enriched =
                        enrich_capability_items(CapabilityKind::Prompts, &db.pool, &server_info.server_id, prompts)
                            .await;
                    let response_data = respond_with_enriched(
                        enriched,
                        false,
                        params.refresh,
                        crate::common::constants::strategies::RUNTIME,
                    );
                    let prompts_resp = json_to_server_prompts_resp(response_data);
                    return Ok(ServerPromptsResp::success(prompts_resp));
                }
                let response_data = create_inspect_response(
                    prompts,
                    false,
                    params.refresh,
                    crate::common::constants::strategies::RUNTIME,
                );
                let prompts_resp = json_to_server_prompts_resp(response_data);
                return Ok(ServerPromptsResp::success(prompts_resp));
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::capability::create_temporary_instance_for_capability(
        app_state,
        &server_info,
        &params,
        super::capability::CapabilityType::Prompts,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let prompts_resp = json_to_server_prompts_resp(response);
        return Ok(ServerPromptsResp::success(prompts_resp));
    }

    // Last resort: offline cache
    if let Ok(cached) = app_state
        .redb_cache
        .get_server_prompts(&server_info.server_id, false)
        .await
    {
        if !cached.is_empty() {
            let processed: Vec<serde_json::Value> = cached.into_iter().map(prompt_json_from_cached).collect();
            if let Ok(db) = get_database_from_state(app_state) {
                let enriched =
                    enrich_capability_items(CapabilityKind::Prompts, &db.pool, &server_info.server_id, processed).await;
                let response_data = respond_with_enriched(
                    enriched,
                    true,
                    params.refresh,
                    crate::common::constants::strategies::CACHE,
                );
                let prompts_resp = json_to_server_prompts_resp(response_data);
                return Ok(ServerPromptsResp::success(prompts_resp));
            }
            let response_data = create_inspect_response(
                processed,
                true,
                params.refresh,
                crate::common::constants::strategies::CACHE,
            );
            let prompts_resp = json_to_server_prompts_resp(response_data);
            return Ok(ServerPromptsResp::success(prompts_resp));
        }
    }

    // Return empty result if no data available from cache or runtime
    let response_data = create_inspect_response(
        Vec::new(),
        false,
        params.refresh,
        crate::common::constants::strategies::NONE,
    );
    let prompts_resp = json_to_server_prompts_resp(response_data);
    Ok(ServerPromptsResp::success(prompts_resp))
}

/// Get detailed prompt argument information with standardized signature
pub async fn server_prompt_arguments(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerPromptArgumentsResp>, StatusCode> {
    let result = server_prompt_arguments_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for getting prompt arguments
async fn server_prompt_arguments_core(
    request: &ServerCapabilityReq,
    app_state: &Arc<AppState>,
) -> Result<ServerPromptArgumentsResp, StatusCode> {
    // Convert request to internal query format
    let query = super::common::InspectQuery {
        refresh: request.refresh.as_ref().map(|r| match r {
            crate::api::models::server::ServerRefreshStrategy::Auto => super::common::RefreshStrategy::CacheFirst,
            crate::api::models::server::ServerRefreshStrategy::Force => super::common::RefreshStrategy::Force,
            crate::api::models::server::ServerRefreshStrategy::Cache => super::common::RefreshStrategy::CacheFirst,
        }),
        format: None,
        include_meta: None,
        timeout: None,
    };

    // Validate and get server info using unified function
    let (_db, _server_info, params) =
        super::common::get_server_info_for_inspect(app_state, &request.id, &query).await?;

    // Return empty result if no data available from cache or runtime
    let response_data = axum::Json(serde_json::json!({
        "data": [],
        "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default() }
    }));
    let arguments_resp = json_to_server_prompt_arguments_resp(response_data);
    Ok(ServerPromptArgumentsResp::success(arguments_resp))
}
