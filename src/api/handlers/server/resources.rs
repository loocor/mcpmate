// Server resources handlers
// Provides handlers for server resource inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{handlers::ApiError, routes::AppState};
use chrono::Utc;

use super::common::CacheQueryExt;
use super::common::{
    InspectParams, RefreshStrategy, get_database_from_state, register_session_if_needed, resolve_server_identifier,
    validate_server_id,
};

/// Query parameters for resources endpoints
#[derive(Debug, serde::Deserialize)]
pub struct ResourcesQuery {
    /// Refresh strategy for resource queries
    pub refresh: Option<RefreshStrategy>,
    /// Response format
    pub format: Option<String>,
    /// Whether to include metadata
    pub include_meta: Option<bool>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
    /// Instance type per refactor spec (production|exploration|validation)
    pub instance_type: Option<String>,
}

impl ResourcesQuery {
    /// Convert to InspectParams
    pub fn to_params(&self) -> Result<InspectParams, ApiError> {
        // Explicit refresh parameter takes priority over instance_type defaults
        let mapped_refresh = if self.refresh.is_some() {
            // Use explicit refresh parameter if provided
            self.refresh
        } else if let Some(ref it) = self.instance_type {
            // Fall back to instance_type defaults only if no explicit refresh
            match it.to_lowercase().as_str() {
                "production" => Some(RefreshStrategy::CacheFirst),
                "exploration" => Some(RefreshStrategy::RefreshIfStale),
                "validation" => Some(RefreshStrategy::CacheFirst),
                _ => None,
            }
        } else {
            None
        };

        Ok(InspectParams {
            refresh: mapped_refresh,
            format: self.format.clone(),
            include_meta: self.include_meta,
        })
    }
}

/// List all resources for a specific server
///
/// Returns a list of resources available on the specified server with
/// configurable filtering and formatting options.
///
/// Supports both server_name and server_id as identifier.
pub async fn list_resources(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<ResourcesQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Register exploration/validation session for runtime/status accounting
    register_session_if_needed(&state, &query.instance_type).await;

    // Try Redb cache with freshness policy on full server snapshot
    let instance_type = super::common::parse_instance_type(&query.instance_type);
    let cache_query =
        super::common::build_cache_query(&server_info.server_id, &params).update_instance_type(instance_type.clone());
    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> = data
                    .resources
                    .into_iter()
                    .map(|r| {
                        serde_json::json!({
                            "uri": r.uri,
                            "name": r.name,
                            "description": r.description,
                            "mime_type": r.mime_type,
                        })
                    })
                    .collect();
                if !processed.is_empty() {
                    return Ok(Json(serde_json::json!({
                        "data": processed,
                        "meta": { "cache_hit": true, "strategy": params.refresh.unwrap_or_default(), "source": "cache" }
                    })));
                }
            }
        }
    }

    // Runtime fallback: attempt to collect via proxy service from connected instances
    if let Ok(pool) = tokio::time::timeout(std::time::Duration::from_millis(500), state.connection_pool.lock()).await {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            // Aggregate resources using rmcp client from the first connected instance
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
                            resources.push(serde_json::json!({
                                "uri": r.uri.clone(),
                                "name": r.name.clone(),
                                "description": r.description.clone(),
                                "mime_type": r.mime_type.clone(),
                            }));
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
                let server_data = crate::core::cache::CachedServerData {
                    server_id: server_info.server_id.clone(),
                    server_name: server_info.server_name.clone(),
                    server_version: None,
                    protocol_version: "latest".to_string(),
                    tools: Vec::new(),
                    resources: cached_resources,
                    prompts: Vec::new(),
                    resource_templates: Vec::new(),
                    cached_at: Utc::now(),
                    fingerprint: format!("runtime:{}:{}", server_info.server_id, Utc::now().timestamp()),
                    instance_type: instance_type.clone(),
                };
                let _ = state.redb_cache.store_server_data(&server_data).await;

                return Ok(Json(serde_json::json!({
                    "data": resources,
                    "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default(), "source": "runtime" }
                })));
            }
        }
    }

    // Last resort: return any cached copy ignoring freshness for offline access
    if let Ok(cached) = state
        .redb_cache
        .get_server_resources(&server_info.server_id, false)
        .await
    {
        if !cached.is_empty() {
            let processed: Vec<serde_json::Value> = cached
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "uri": r.uri,
                        "name": r.name,
                        "description": r.description,
                        "mime_type": r.mime_type,
                    })
                })
                .collect();
            return Ok(Json(serde_json::json!({
                "data": processed,
                "meta": { "cache_hit": true, "strategy": params.refresh.unwrap_or_default(), "source": "cache" }
            })));
        }
    }

    // Fallback empty
    Ok(Json(serde_json::json!({
        "data": [],
        "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default() }
    })))
}

/// List resource templates for a specific server
///
/// Returns resource templates that define URI patterns for dynamic resources.
/// Templates are used to generate resource URIs with parameters.
///
/// Supports both server_name and server_id as identifier.
pub async fn list_resource_templates(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<ResourcesQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Try Redb cache first with freshness policy on full snapshot
    let instance_type = super::common::parse_instance_type(&query.instance_type);
    let cache_query =
        super::common::build_cache_query(&server_info.server_id, &params).update_instance_type(instance_type.clone());
    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> = data
                    .resource_templates
                    .into_iter()
                    .map(|t| {
                        serde_json::json!({
                            "uri_template": t.uri_template,
                            "name": t.name,
                            "description": t.description,
                            "mime_type": t.mime_type,
                        })
                    })
                    .collect();
                if !processed.is_empty() {
                    return Ok(Json(serde_json::json!({
                        "data": processed,
                        "meta": { "cache_hit": true, "strategy": params.refresh.unwrap_or_default(), "source": "cache" }
                    })));
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
                            templates.push(serde_json::json!({
                                "uri_template": t.uri_template,
                                "name": t.name,
                                "description": t.description,
                                "mime_type": t.mime_type,
                            }));
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
                let server_data = crate::core::cache::CachedServerData {
                    server_id: server_info.server_id.clone(),
                    server_name: server_info.server_name.clone(),
                    server_version: None,
                    protocol_version: "latest".to_string(),
                    tools: Vec::new(),
                    resources: Vec::new(),
                    prompts: Vec::new(),
                    resource_templates: cached_templates,
                    cached_at: Utc::now(),
                    fingerprint: format!("runtime:{}:{}", server_info.server_id, Utc::now().timestamp()),
                    instance_type: instance_type.clone(),
                };
                let _ = state.redb_cache.store_server_data(&server_data).await;

                return Ok(Json(serde_json::json!({
                    "data": templates,
                    "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default(), "source": "runtime" }
                })));
            }
        }
    }

    // Last resort: return any cached copy ignoring freshness
    if let Ok(cached) = state
        .redb_cache
        .get_server_resource_templates(&server_info.server_id, false)
        .await
    {
        if !cached.is_empty() {
            let processed: Vec<serde_json::Value> = cached
                .into_iter()
                .map(|t| {
                    serde_json::json!({
                        "uri_template": t.uri_template,
                        "name": t.name,
                        "description": t.description,
                        "mime_type": t.mime_type,
                    })
                })
                .collect();
            return Ok(Json(serde_json::json!({
                "data": processed,
                "meta": { "cache_hit": true, "strategy": params.refresh.unwrap_or_default(), "source": "cache" }
            })));
        }
    }

    Ok(Json(serde_json::json!({
        "data": [],
        "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default() }
    })))
}
