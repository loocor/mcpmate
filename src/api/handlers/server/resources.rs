// Server resources handlers
// Provides handlers for server resource inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{handlers::ApiError, routes::AppState};
use chrono::Utc;

use super::capability::{
    CapabilityKind, enrich_capability_items, respond_with_enriched,
    resource_json, resource_json_from_cached, resource_template_json, resource_template_json_from_cached,
};
use super::common::{
    InspectQuery, create_inspect_response, create_runtime_cache_data, get_database_from_state,
    resolve_server_identifier, validate_server_id,
};

/// List all resources for a specific server
///
/// Strategy order:
/// 1) Cache-first: query Redb snapshot with freshness policy.
/// 2) Runtime fallback: aggregate via connected instances (proxy service).
/// 3) Force refresh (if requested): create a temporary instance to fetch data.
/// 4) Offline cache: return any cached copy ignoring freshness.
/// 5) None: return empty.
///
/// Supports both `server_name` and `server_id` as identifier.
pub async fn list_resources(
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

    // Short-circuit if server declares no resources capability
    if let Ok((server_row, _id)) = super::common::get_server_by_identifier(&db.pool, &server_info.server_name).await {
        if !server_row.has_capability(crate::common::capability::CapabilityToken::Resources) {
            return Ok(create_inspect_response(
                Vec::new(),
                false,
                params.refresh,
                "capability-resources-unsupported",
            ));
        }
    }

    // Try Redb cache with freshness policy on full server snapshot
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);
    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> =
                    data.resources.into_iter().map(resource_json_from_cached).collect();
                if !processed.is_empty() {
                    if let Ok(db) = get_database_from_state(&state) {
                        let enriched = enrich_capability_items(
                            CapabilityKind::Resources,
                            &db.pool,
                            &server_info.server_id,
                            processed,
                        )
                        .await;
                        return Ok(respond_with_enriched(enriched, true, params.refresh, "cache"));
                    }
                    return Ok(create_inspect_response(processed, true, params.refresh, "cache"));
                }
            }
        }
    }

    // Runtime fallback: attempt to collect via proxy service across connected instances
    if let Ok(pool) = tokio::time::timeout(std::time::Duration::from_millis(500), state.connection_pool.lock()).await {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            // Aggregate resources across connected instances using rmcp client
            let mut resources: Vec<serde_json::Value> = Vec::new();
            let mut cached_resources: Vec<crate::core::cache::CachedResourceInfo> = Vec::new();

            for conn in instances.values() {
                if !conn.is_connected() || !conn.supports_resources() {
                    continue;
                }
                // Use protocol helper to fetch all resources from this instance
                if let Some(service) = &conn.service {
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
                                cached_at: Utc::now(),
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
                let _ = state.redb_cache.store_server_data(&server_data).await;

                // Enrich resources with id/unique_name from DB mapping
                if let Ok(db) = get_database_from_state(&state) {
                    let enriched =
                        enrich_capability_items(CapabilityKind::Resources, &db.pool, &server_info.server_id, resources)
                            .await;
                    return Ok(respond_with_enriched(enriched, false, params.refresh, "runtime"));
                }
                return Ok(create_inspect_response(resources, false, params.refresh, "runtime"));
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::capability::create_temporary_instance_for_capability(
        &state,
        &server_info,
        &params,
        super::capability::CapabilityType::Resources,
    )
    .await?
    {
        return Ok(response);
    }

    // Last resort: return any cached copy ignoring freshness for offline access
    if let Ok(cached) = state
        .redb_cache
        .get_server_resources(&server_info.server_id, false)
        .await
    {
        if !cached.is_empty() {
            let processed: Vec<serde_json::Value> = cached.into_iter().map(resource_json_from_cached).collect();
            if let Ok(db) = get_database_from_state(&state) {
                let enriched =
                    enrich_capability_items(CapabilityKind::Resources, &db.pool, &server_info.server_id, processed)
                        .await;
                return Ok(respond_with_enriched(enriched, true, params.refresh, "cache"));
            }
            return Ok(create_inspect_response(processed, true, params.refresh, "cache"));
        }
    }

    // Fallback empty
    Ok(create_inspect_response(Vec::new(), false, params.refresh, "none"))
}

/// List resource templates for a specific server
///
/// Strategy order:
/// 1) Cache-first: query Redb snapshot with freshness policy.
/// 2) Runtime fallback: aggregate via connected instances (proxy service).
/// 3) Force refresh (if requested): create a temporary instance to fetch data.
/// 4) Offline cache: return any cached copy ignoring freshness.
/// 5) None: return empty.
///
/// Supports both `server_name` and `server_id` as identifier.
pub async fn list_resource_templates(
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

    // Short-circuit if server declares no resourceTemplates capability
    if let Ok((server_row, _id)) = super::common::get_server_by_identifier(&db.pool, &server_info.server_name).await {
        if !server_row.has_capability(crate::common::capability::CapabilityToken::ResourceTemplates) {
            return Ok(create_inspect_response(
                Vec::new(),
                false,
                params.refresh,
                "capability-resourceTemplates-unsupported",
            ));
        }
    }

    // Try Redb cache first with freshness policy on full snapshot
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);
    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> = data
                    .resource_templates
                    .into_iter()
                    .map(resource_template_json_from_cached)
                    .collect();
                if !processed.is_empty() {
                    if let Ok(db) = get_database_from_state(&state) {
                        let enriched = enrich_capability_items(
                            CapabilityKind::ResourceTemplates,
                            &db.pool,
                            &server_info.server_id,
                            processed,
                        )
                        .await;
                        return Ok(respond_with_enriched(enriched, true, params.refresh, "cache"));
                    }
                    return Ok(create_inspect_response(processed, true, params.refresh, "cache"));
                }
            }
        }
    }
    // Runtime fallback aggregation using protocol helper
    if let Ok(pool) = tokio::time::timeout(std::time::Duration::from_millis(500), state.connection_pool.lock()).await {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            let mut templates: Vec<serde_json::Value> = Vec::new();
            let mut cached_templates: Vec<crate::core::cache::CachedResourceTemplateInfo> = Vec::new();

            for conn in instances.values() {
                if !conn.is_connected() || !conn.supports_resources() {
                    continue;
                }
                if let Some(service) = &conn.service {
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
                                cached_at: Utc::now(),
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
                let _ = state.redb_cache.store_server_data(&server_data).await;

                if let Ok(db) = get_database_from_state(&state) {
                    let enriched = enrich_capability_items(
                        CapabilityKind::ResourceTemplates,
                        &db.pool,
                        &server_info.server_id,
                        templates,
                    )
                    .await;
                    return Ok(respond_with_enriched(enriched, false, params.refresh, "runtime"));
                }
                return Ok(create_inspect_response(templates, false, params.refresh, "runtime"));
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::capability::create_temporary_instance_for_capability(
        &state,
        &server_info,
        &params,
        super::capability::CapabilityType::ResourceTemplates,
    )
    .await?
    {
        return Ok(response);
    }

    // Last resort: return any cached copy ignoring freshness
    if let Ok(cached) = state
        .redb_cache
        .get_server_resource_templates(&server_info.server_id, false)
        .await
    {
        if !cached.is_empty() {
            let processed: Vec<serde_json::Value> =
                cached.into_iter().map(resource_template_json_from_cached).collect();
            if let Ok(db) = get_database_from_state(&state) {
                let enriched = enrich_capability_items(
                    CapabilityKind::ResourceTemplates,
                    &db.pool,
                    &server_info.server_id,
                    processed,
                )
                .await;
                return Ok(respond_with_enriched(enriched, true, params.refresh, "cache"));
            }
            return Ok(create_inspect_response(processed, true, params.refresh, "cache"));
        }
    }

    Ok(create_inspect_response(Vec::new(), false, params.refresh, "none"))
}
