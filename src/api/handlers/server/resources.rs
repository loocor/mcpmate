// Server resources handlers
// Provides handlers for server resource inspect endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::api::{
    models::server::{
        ServerCapabilityMeta, ServerCapabilityReq, ServerResourceTemplatesData, ServerResourceTemplatesResp,
        ServerResourcesData, ServerResourcesResp,
    },
    routes::AppState,
};
use chrono::Utc;

use super::capability::{
    CapabilityKind, enrich_capability_items, resource_json, resource_json_from_cached, resource_template_json,
    resource_template_json_from_cached, respond_with_enriched,
};
use super::common::{create_inspect_response, create_runtime_cache_data, get_database_from_state, validate_server_id};

/// Helper function to convert Json response to ServerResourcesResp
fn json_to_server_resources_resp(json_response: axum::Json<serde_json::Value>) -> ServerResourcesData {
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

    ServerResourcesData { data, state, meta }
}

/// Helper function to convert Json response to ServerResourceTemplatesResp
fn json_to_server_resource_templates_resp(json_response: axum::Json<serde_json::Value>) -> ServerResourceTemplatesData {
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

    ServerResourceTemplatesData { data, state, meta }
}

/// List all resources for a specific server with standardized signature
pub async fn server_resources(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerResourcesResp>, StatusCode> {
    let result = server_resources_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for listing server resources
#[tracing::instrument(skip(app_state), level = "debug")]
async fn server_resources_core(
    request: &ServerCapabilityReq,
    app_state: &Arc<AppState>,
) -> Result<ServerResourcesResp, StatusCode> {
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

    // Get database and load server by ID
    let db = get_database_from_state(app_state).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let server_row = crate::config::server::get_server_by_id(&db.pool, &request.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let server_info = super::common::ServerIdentification {
        server_id: request.id.clone(),
        server_name: server_row.name.clone(),
    };

    // Validate server ID format
    validate_server_id(&server_info.server_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse query parameters
    let params = query.to_params().map_err(|_| StatusCode::BAD_REQUEST)?;

    // Short-circuit only if the server explicitly declares capabilities and lacks resources capability
    if let Ok((server_row, _id)) = super::common::get_server_by_identifier(&db.pool, &server_info.server_name).await {
        if server_row.capabilities.is_some()
            && !server_row.has_capability(crate::common::capability::CapabilityToken::Resources)
        {
            let response_data =
                create_inspect_response(Vec::new(), false, params.refresh, "capability-resources-unsupported");
            let resources_resp = json_to_server_resources_resp(response_data);
            return Ok(ServerResourcesResp::success(resources_resp));
        }
    }

    // Try Redb cache with freshness policy on full server snapshot
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);
    if let Ok(cache_result) = app_state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> =
                    data.resources.into_iter().map(resource_json_from_cached).collect();
                if !processed.is_empty() {
                    if let Ok(db) = get_database_from_state(app_state) {
                        let enriched = enrich_capability_items(
                            CapabilityKind::Resources,
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
                        let resources_resp = json_to_server_resources_resp(response_data);
                        return Ok(ServerResourcesResp::success(resources_resp));
                    }
                    let response_data = create_inspect_response(
                        processed,
                        true,
                        params.refresh,
                        crate::common::constants::strategies::CACHE,
                    );
                    let resources_resp = json_to_server_resources_resp(response_data);
                    return Ok(ServerResourcesResp::success(resources_resp));
                }
            }
        }
    }

    // Runtime fallback: attempt to collect via proxy service across connected instances
    if let Ok(pool) = tokio::time::timeout(
        std::time::Duration::from_millis(crate::common::constants::timeouts::LOCK_MS),
        app_state.connection_pool.lock(),
    )
    .await
    {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            let connected_instances: Vec<_> = instances
                .values()
                .filter(|conn| conn.is_connected() && conn.supports_resources())
                .collect();

            let mut resources = Vec::new();
            let mut cached_resources = Vec::new();

            for conn in connected_instances {
                // Use protocol helper to fetch all resources from this instance
                if let Some(service) = &conn.service {
                    let now = Utc::now();
                    let mut cursor = None;
                    while let Ok(result) = service
                        .list_resources(Some(rmcp::model::PaginatedRequestParam { cursor }))
                        .await
                    {
                        for r in result.resources {
                            resources.push(resource_json(
                                &r.uri,
                                Some(r.name.clone()),
                                r.description.clone(),
                                r.mime_type.clone(),
                                None,
                                None,
                            ));
                            cached_resources.push(crate::core::cache::CachedResourceInfo {
                                uri: r.uri.clone(),
                                name: Some(r.name.clone()),
                                description: r.description.clone(),
                                mime_type: r.mime_type.clone(),
                                enabled: true,
                                cached_at: now,
                            });
                        }
                        cursor = result.next_cursor;
                        if cursor.is_none() {
                            break;
                        }
                    }
                }
            }

            if !resources.is_empty() {
                // Persist partial snapshot into Redb
                let server_data =
                    create_runtime_cache_data(&server_info, Vec::new(), cached_resources, Vec::new(), Vec::new());
                let _ = app_state.redb_cache.store_server_data(&server_data).await;

                // Enrich resources with id/unique_name from DB mapping
                if let Ok(db) = get_database_from_state(app_state) {
                    let enriched =
                        enrich_capability_items(CapabilityKind::Resources, &db.pool, &server_info.server_id, resources)
                            .await;
                    let response_data = respond_with_enriched(
                        enriched,
                        false,
                        params.refresh,
                        crate::common::constants::strategies::RUNTIME,
                    );
                    let resources_resp = json_to_server_resources_resp(response_data);
                    return Ok(ServerResourcesResp::success(resources_resp));
                }
                let response_data = create_inspect_response(
                    resources,
                    false,
                    params.refresh,
                    crate::common::constants::strategies::RUNTIME,
                );
                let resources_resp = json_to_server_resources_resp(response_data);
                return Ok(ServerResourcesResp::success(resources_resp));
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::capability::create_temporary_instance_for_capability(
        app_state,
        &server_info,
        &params,
        super::capability::CapabilityType::Resources,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let resources_resp = json_to_server_resources_resp(response);
        return Ok(ServerResourcesResp::success(resources_resp));
    }

    // Last resort: return any cached copy ignoring freshness for offline access
    if let Ok(cached) = app_state
        .redb_cache
        .get_server_resources(&server_info.server_id, false)
        .await
    {
        if !cached.is_empty() {
            let processed: Vec<serde_json::Value> = cached.into_iter().map(resource_json_from_cached).collect();
            if let Ok(db) = get_database_from_state(app_state) {
                let enriched =
                    enrich_capability_items(CapabilityKind::Resources, &db.pool, &server_info.server_id, processed)
                        .await;
                let response_data = respond_with_enriched(
                    enriched,
                    true,
                    params.refresh,
                    crate::common::constants::strategies::CACHE,
                );
                let resources_resp = json_to_server_resources_resp(response_data);
                return Ok(ServerResourcesResp::success(resources_resp));
            }
            let response_data = create_inspect_response(
                processed,
                true,
                params.refresh,
                crate::common::constants::strategies::CACHE,
            );
            let resources_resp = json_to_server_resources_resp(response_data);
            return Ok(ServerResourcesResp::success(resources_resp));
        }
    }

    // Fallback empty
    let response_data = create_inspect_response(Vec::new(), false, params.refresh, "none");
    let resources_resp = json_to_server_resources_resp(response_data);
    Ok(ServerResourcesResp::success(resources_resp))
}

/// List resource templates for a specific server with standardized signature
pub async fn server_resource_templates(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerResourceTemplatesResp>, StatusCode> {
    let result = server_resource_templates_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for listing server resource templates
#[tracing::instrument(skip(app_state), level = "debug")]
async fn server_resource_templates_core(
    request: &ServerCapabilityReq,
    app_state: &Arc<AppState>,
) -> Result<ServerResourceTemplatesResp, StatusCode> {
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

    // Get database and load server by ID
    let db = get_database_from_state(app_state).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let server_row = crate::config::server::get_server_by_id(&db.pool, &request.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let server_info = super::common::ServerIdentification {
        server_id: request.id.clone(),
        server_name: server_row.name.clone(),
    };

    // Validate server ID format
    validate_server_id(&server_info.server_id).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Parse query parameters
    let params = query.to_params().map_err(|_| StatusCode::BAD_REQUEST)?;

    // Short-circuit only if the server explicitly declares capabilities and lacks resources capability
    if let Ok((server_row, _id)) = super::common::get_server_by_identifier(&db.pool, &server_info.server_name).await {
        if server_row.capabilities.is_some()
            && !server_row.has_capability(crate::common::capability::CapabilityToken::Resources)
        {
            let response_data =
                create_inspect_response(Vec::new(), false, params.refresh, "capability-resources-unsupported");
            let templates_resp = json_to_server_resource_templates_resp(response_data);
            return Ok(ServerResourceTemplatesResp::success(templates_resp));
        }
    }

    // Try Redb cache first with freshness policy on full snapshot
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);
    if let Ok(cache_result) = app_state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> = data
                    .resource_templates
                    .into_iter()
                    .map(resource_template_json_from_cached)
                    .collect();
                if !processed.is_empty() {
                    if let Ok(db) = get_database_from_state(app_state) {
                        let enriched = enrich_capability_items(
                            CapabilityKind::ResourceTemplates,
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
                        let templates_resp = json_to_server_resource_templates_resp(response_data);
                        return Ok(ServerResourceTemplatesResp::success(templates_resp));
                    }
                    let response_data = create_inspect_response(
                        processed,
                        true,
                        params.refresh,
                        crate::common::constants::strategies::CACHE,
                    );
                    let templates_resp = json_to_server_resource_templates_resp(response_data);
                    return Ok(ServerResourceTemplatesResp::success(templates_resp));
                }
            }
        }
    }
    // Runtime fallback aggregation using protocol helper
    if let Ok(pool) = tokio::time::timeout(
        std::time::Duration::from_millis(crate::common::constants::timeouts::LOCK_MS),
        app_state.connection_pool.lock(),
    )
    .await
    {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            let connected_instances: Vec<_> = instances
                .values()
                .filter(|conn| conn.is_connected() && conn.supports_resources())
                .collect();

            let mut templates = Vec::new();
            let mut cached_templates = Vec::new();

            for conn in connected_instances {
                if let Some(service) = &conn.service {
                    let now = Utc::now();
                    let mut cursor = None;
                    while let Ok(result) = service
                        .list_resource_templates(Some(rmcp::model::PaginatedRequestParam { cursor }))
                        .await
                    {
                        for t in result.resource_templates {
                            templates.push(resource_template_json(
                                &t.uri_template,
                                Some(t.name.clone()),
                                t.description.clone(),
                                t.mime_type.clone(),
                                None,
                                None,
                            ));
                            cached_templates.push(crate::core::cache::CachedResourceTemplateInfo {
                                uri_template: t.uri_template.clone(),
                                name: Some(t.name.clone()),
                                description: t.description.clone(),
                                mime_type: t.mime_type.clone(),
                                enabled: true,
                                cached_at: now,
                            });
                        }
                        cursor = result.next_cursor;
                        if cursor.is_none() {
                            break;
                        }
                    }
                }
            }

            if !templates.is_empty() {
                let server_data =
                    create_runtime_cache_data(&server_info, Vec::new(), Vec::new(), Vec::new(), cached_templates);
                let _ = app_state.redb_cache.store_server_data(&server_data).await;

                if let Ok(db) = get_database_from_state(app_state) {
                    let enriched = enrich_capability_items(
                        CapabilityKind::ResourceTemplates,
                        &db.pool,
                        &server_info.server_id,
                        templates,
                    )
                    .await;
                    let response_data = respond_with_enriched(
                        enriched,
                        false,
                        params.refresh,
                        crate::common::constants::strategies::RUNTIME,
                    );
                    let templates_resp = json_to_server_resource_templates_resp(response_data);
                    return Ok(ServerResourceTemplatesResp::success(templates_resp));
                }
                let response_data = create_inspect_response(
                    templates,
                    false,
                    params.refresh,
                    crate::common::constants::strategies::RUNTIME,
                );
                let templates_resp = json_to_server_resource_templates_resp(response_data);
                return Ok(ServerResourceTemplatesResp::success(templates_resp));
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::capability::create_temporary_instance_for_capability(
        app_state,
        &server_info,
        &params,
        super::capability::CapabilityType::ResourceTemplates,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let templates_resp = json_to_server_resource_templates_resp(response);
        return Ok(ServerResourceTemplatesResp::success(templates_resp));
    }

    // Last resort: return any cached copy ignoring freshness
    if let Ok(cached) = app_state
        .redb_cache
        .get_server_resource_templates(&server_info.server_id, false)
        .await
    {
        if !cached.is_empty() {
            let processed: Vec<serde_json::Value> =
                cached.into_iter().map(resource_template_json_from_cached).collect();
            if let Ok(db) = get_database_from_state(app_state) {
                let enriched = enrich_capability_items(
                    CapabilityKind::ResourceTemplates,
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
                let templates_resp = json_to_server_resource_templates_resp(response_data);
                return Ok(ServerResourceTemplatesResp::success(templates_resp));
            }
            let response_data = create_inspect_response(
                processed,
                true,
                params.refresh,
                crate::common::constants::strategies::CACHE,
            );
            let templates_resp = json_to_server_resource_templates_resp(response_data);
            return Ok(ServerResourceTemplatesResp::success(templates_resp));
        }
    }

    let response_data = create_inspect_response(
        Vec::new(),
        false,
        params.refresh,
        crate::common::constants::strategies::NONE,
    );
    let templates_resp = json_to_server_resource_templates_resp(response_data);
    Ok(ServerResourceTemplatesResp::success(templates_resp))
}
