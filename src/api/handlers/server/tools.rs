// Server tools handlers
// Provides handlers for server tool inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{handlers::ApiError, routes::AppState};
use chrono::Utc;

use super::common::CacheQueryExt;
use super::common::{
    InspectParams, get_database_from_state, register_session_if_needed, resolve_server_identifier, validate_server_id,
};

/// Query parameters for tools endpoints
#[derive(Debug, serde::Deserialize)]
pub struct ToolsQuery {
    /// Refresh strategy for tool queries
    pub refresh: Option<super::common::RefreshStrategy>,
    /// Response format
    pub format: Option<String>,
    /// Whether to include metadata
    pub include_meta: Option<bool>,
    /// Timeout in seconds
    pub timeout: Option<u64>,
    /// Instance type per refactor spec (production|exploration|validation)
    pub instance_type: Option<String>,
}

impl ToolsQuery {
    /// Convert to InspectParams
    pub fn to_params(&self) -> Result<InspectParams, ApiError> {
        // Explicit refresh parameter takes priority over instance_type defaults
        let mapped_refresh = if self.refresh.is_some() {
            // Use explicit refresh parameter if provided
            self.refresh
        } else if let Some(ref it) = self.instance_type {
            // Fall back to instance_type defaults only if no explicit refresh
            match it.to_lowercase().as_str() {
                // Production -> prefer cached data
                "production" => Some(super::common::RefreshStrategy::CacheFirst),
                // Exploration -> refresh if stale
                "exploration" => Some(super::common::RefreshStrategy::RefreshIfStale),
                // Validation -> prefer cached data for performance
                "validation" => Some(super::common::RefreshStrategy::CacheFirst),
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

/// Validate tool name format
fn validate_tool_name(tool_name: &str) -> Result<(), ApiError> {
    if tool_name.trim().is_empty() {
        return Err(ApiError::BadRequest("Tool name cannot be empty".to_string()));
    }

    if tool_name.len() > 255 {
        return Err(ApiError::BadRequest(
            "Tool name too long (max 255 characters)".to_string(),
        ));
    }

    Ok(())
}

/// List all tools for a specific server
///
/// Returns a list of tools available on the specified server with
/// configurable filtering and formatting options.
///
/// Supports both server_name and server_id as identifier.
///
/// ## Cache and Refresh Strategies
///
/// This endpoint supports different caching strategies based on `instance_type` and `refresh` parameters:
///
/// ### Instance Types:
/// - **`production`**: Uses cached data by default (CacheFirst)
/// - **`exploration`**: Refreshes if data is older than 5 minutes (RefreshIfStale)
/// - **`validation`**: Uses cached data for performance (CacheFirst)
///
/// ### Refresh Parameters (override instance type defaults):
/// - **`refresh=cacheFirst`**: Always use cached data if available (fastest)
/// - **`refresh=refreshIfStale`**: Use cache if data is less than 5 minutes old
/// - **`refresh=force`**: Always fetch fresh data by creating new instance (slowest)
/// 
/// **Priority**: Explicit `refresh` parameter > `instance_type` default > system default
///
/// ### Example Usage:
/// ```
/// # Fast validation (uses cache)
/// GET /api/mcp/servers/context7/tools?instance_type=validation
///
/// # Force fresh validation data
/// GET /api/mcp/servers/context7/tools?instance_type=validation&refresh=force
///
/// # Refresh if validation data is stale
/// GET /api/mcp/servers/context7/tools?instance_type=validation&refresh=refreshIfStale
///
/// # Production with forced refresh
/// GET /api/mcp/servers/context7/tools?instance_type=production&refresh=force
/// ```
///
/// ### Performance Notes:
/// - Cached requests: ~10-50ms response time
/// - Fresh instance creation: ~1000-2000ms response time
/// - Validation instances are temporary and destroyed after use
pub async fn list_tools(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<ToolsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Register exploration/validation session for runtime/status accounting as early as possible
    register_session_if_needed(&state, &query.instance_type).await;

    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate server ID format
    validate_server_id(&server_info.server_id)?;

    // Parse query parameters
    let params = query.to_params()?;

    // Try Redb cache first with freshness policy
    let instance_type = super::common::parse_instance_type(&query.instance_type);
    let cache_query =
        super::common::build_cache_query(&server_info.server_id, &params).update_instance_type(instance_type.clone());

    tracing::info!(
        "Querying cache for server '{}' with instance_type: {:?}",
        server_info.server_name,
        instance_type
    );
    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        tracing::info!("Cache query result: cache_hit={}", cache_result.cache_hit);
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> = data
                    .tools
                    .into_iter()
                    .map(|t| {
                        let schema = t.input_schema().unwrap_or_else(|_| serde_json::json!({}));
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "input_schema": schema,
                            "unique_name": t.unique_name
                        })
                    })
                    .collect();
                if !processed.is_empty() {
                    return Ok(Json(serde_json::json!({
                        "data": processed,
                        "meta": { "cache_hit": true, "strategy": params.refresh.unwrap_or_default(), "source": "cache" }
                    })));
                }
                // empty cached snapshot is treated as miss; fall through to runtime/offline
            }
        }
    }

    // Validation instance handling: create temporary connection if needed
    if let Some(ref instance_type_str) = query.instance_type {
        if instance_type_str.to_lowercase() == "validation" {
            if let Ok(mut pool) =
                tokio::time::timeout(std::time::Duration::from_secs(10), state.connection_pool.lock()).await
            {
                // Try to get or create validation instance
                match pool
                    .get_or_create_validation_instance(
                        &server_info.server_name,
                        "api",
                        std::time::Duration::from_secs(5 * 60), // TTL not used but required by signature
                    )
                    .await
                {
                    Ok(Some(validation_conn)) => {
                        // Extract tools from validation instance
                        let mut tools: Vec<serde_json::Value> = Vec::new();
                        let mut cached_tools: Vec<crate::core::cache::CachedToolInfo> = Vec::new();

                        for t in &validation_conn.tools {
                            let schema = t.schema_as_json_value();
                            tools.push(serde_json::json!({
                                "name": t.name,
                                "description": t.description,
                                "input_schema": schema,
                                "unique_name": serde_json::Value::Null
                            }));

                            // Build cacheable tool info
                            let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
                            cached_tools.push(crate::core::cache::CachedToolInfo {
                                name: t.name.to_string(),
                                description: t.description.clone().map(|d| d.into_owned()),
                                input_schema_json,
                                unique_name: None,
                                enabled: true,
                                cached_at: Utc::now(),
                            });
                        }

                        // Cache the results for future requests (async operation)
                        if !cached_tools.is_empty() {
                            let server_data = crate::core::cache::CachedServerData {
                                server_id: server_info.server_id.clone(),
                                server_name: server_info.server_name.clone(),
                                server_version: None,
                                protocol_version: "latest".to_string(),
                                tools: cached_tools,
                                resources: Vec::new(),
                                prompts: Vec::new(),
                                resource_templates: Vec::new(),
                                cached_at: Utc::now(),
                                fingerprint: format!("validation:{}:{}", server_info.server_id, Utc::now().timestamp()),
                                instance_type: instance_type.clone(),
                            };
                            match state.redb_cache.store_server_data(&server_data).await {
                                Ok(_) => tracing::info!(
                                    "[CACHE][STORE CALL OK] server_id={} instance_type=validation",
                                    server_info.server_id
                                ),
                                Err(e) => tracing::error!(
                                    "[CACHE][STORE CALL ERR] server_id={} err={:?}",
                                    server_info.server_id,
                                    e
                                ),
                            }
                        }

                        // Immediately destroy the validation instance (create-use-destroy lifecycle)
                        let _ = pool.destroy_validation_instance(&server_info.server_name, "api").await;

                        return Ok(Json(serde_json::json!({
                            "data": tools,
                            "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default(), "source": "validation" }
                        })));
                    }
                    Ok(None) => {
                        tracing::warn!(
                            "Failed to create validation instance for server '{}' - returned None",
                            server_info.server_name
                        );
                        // Fall through to other methods
                    }
                    Err(e) => {
                        tracing::error!(
                            "Error creating validation instance for server '{}': {:?}",
                            server_info.server_name,
                            e
                        );
                        // Fall through to other methods
                    }
                }
            }
        }
    }

    // Runtime fallback: read tools from connected instances in the pool (no inspect)
    if let Ok(pool) = tokio::time::timeout(std::time::Duration::from_millis(500), state.connection_pool.lock()).await {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            // Collect tools from any connected instance
            let mut tools: Vec<serde_json::Value> = Vec::new();
            let mut cached_tools: Vec<crate::core::cache::CachedToolInfo> = Vec::new();
            for conn in instances.values() {
                if !conn.is_connected() {
                    continue;
                }
                for t in &conn.tools {
                    let schema = t.schema_as_json_value();
                    tools.push(serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": schema,
                        "unique_name": serde_json::Value::Null
                    }));

                    // Build cacheable tool info
                    let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
                    cached_tools.push(crate::core::cache::CachedToolInfo {
                        name: t.name.to_string(),
                        description: t.description.clone().map(|d| d.into_owned()),
                        input_schema_json,
                        unique_name: None,
                        enabled: true,
                        cached_at: Utc::now(),
                    });
                }
            }
            if !tools.is_empty() {
                // Persist into Redb cache for future requests
                let server_data = crate::core::cache::CachedServerData {
                    server_id: server_info.server_id.clone(),
                    server_name: server_info.server_name.clone(),
                    server_version: None,
                    protocol_version: "latest".to_string(),
                    tools: cached_tools,
                    resources: Vec::new(),
                    prompts: Vec::new(),
                    resource_templates: Vec::new(),
                    cached_at: Utc::now(),
                    fingerprint: format!("runtime:{}:{}", server_info.server_id, Utc::now().timestamp()),
                    instance_type: instance_type.clone(),
                };
                let _ = state.redb_cache.store_server_data(&server_data).await;

                return Ok(Json(serde_json::json!({
                    "data": tools,
                    "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default(), "source": "runtime" }
                })));
            }
        }
    }

    // Last resort: return any cached tools ignoring freshness if available (support offline access)
    if let Ok(cached_tools) = state.redb_cache.get_server_tools(&server_info.server_id, false).await {
        if !cached_tools.is_empty() {
            let processed: Vec<serde_json::Value> = cached_tools
                .into_iter()
                .map(|t| {
                    let schema = t.input_schema().unwrap_or_else(|_| serde_json::json!({}));
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": schema,
                        "unique_name": t.unique_name
                    })
                })
                .collect();
            return Ok(Json(serde_json::json!({
                "data": processed,
                "meta": { "cache_hit": true, "strategy": params.refresh.unwrap_or_default(), "source": "cache" }
            })));
        }
    }

    // No data available
    Ok(Json(serde_json::json!({
        "data": [],
        "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default() }
    })))
}

/// Get detailed information for a specific tool
///
/// Returns complete tool information including input schema, description,
/// and configuration status for the specified tool on the given server.
///
/// Uses tool_name instead of tool_id for clearer semantics.
/// Supports both server_name and server_id as identifier.
pub async fn get_tool_detail(
    State(state): State<Arc<AppState>>,
    Path((identifier, tool_name)): Path<(String, String)>,
    Query(query): Query<ToolsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Get database and resolve server identifier
    let db = get_database_from_state(&state)?;
    let server_info = resolve_server_identifier(&db.pool, &identifier).await?;

    // Validate parameters
    validate_server_id(&server_info.server_id)?;
    validate_tool_name(&tool_name)?;

    // Parse query parameters
    let _params = query.to_params()?;

    // Tool details can be retrieved from cache or runtime pool
    // Try cache first
    if let Ok(cached_tools) = state.redb_cache.get_server_tools(&server_info.server_id, false).await {
        if let Some(tool) = cached_tools.iter().find(|t| t.name == tool_name) {
            let schema = tool.input_schema().unwrap_or_else(|_| serde_json::json!({}));
            return Ok(Json(serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": schema,
                "unique_name": tool.unique_name,
                "enabled": tool.enabled,
                "cached_at": tool.cached_at.to_rfc3339()
            })));
        }
    }

    // Fallback to runtime pool if available
    if let Ok(pool) = tokio::time::timeout(std::time::Duration::from_millis(500), state.connection_pool.lock()).await {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            for conn in instances.values() {
                if !conn.is_connected() {
                    continue;
                }
                for tool in &conn.tools {
                    if tool.name == tool_name {
                        return Ok(Json(serde_json::json!({
                            "name": tool.name,
                            "description": tool.description,
                            "input_schema": tool.schema_as_json_value(),
                            "source": "runtime"
                        })));
                    }
                }
            }
        }
    }

    Err(ApiError::NotFound(format!(
        "Tool '{}' not found for server '{}'",
        tool_name, server_info.server_name
    )))
}
