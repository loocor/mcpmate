// Server tools handlers
// Provides handlers for server tool inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{handlers::ApiError, routes::AppState};
use chrono::Utc;

use super::common::{
    InspectQuery, create_inspect_response, create_runtime_cache_data, get_database_from_state, resolve_server_identifier, validate_server_id,
};



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
/// This endpoint supports different caching strategies based on the `refresh` parameter:
///
/// ### Refresh Parameters:
/// - **`refresh=cacheFirst`**: Always use cached data if available (fastest, default)
/// - **`refresh=refreshIfStale`**: Use cache if data is less than 5 minutes old
/// - **`refresh=force`**: Always fetch fresh data (slowest)
///
/// ### Example Usage:
/// ```
/// # Use cached data (default)
/// GET /api/mcp/servers/context7/tools
///
/// # Force fresh data
/// GET /api/mcp/servers/context7/tools?refresh=force
///
/// # Refresh if data is stale
/// GET /api/mcp/servers/context7/tools?refresh=refreshIfStale
/// ```
///
/// ### Performance Notes:
/// - Cached requests: ~10-50ms response time
/// - Runtime requests: ~100-500ms response time
pub async fn list_tools(
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

    // Try Redb cache first with freshness policy
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);

    tracing::info!(
        "Querying cache for server '{}' with refresh strategy: {:?}",
        server_info.server_name,
        params.refresh.unwrap_or_default()
    );
    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        tracing::info!("Cache query result: cache_hit={}, strategy={:?}", cache_result.cache_hit, params.refresh.unwrap_or_default());
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
                    return Ok(create_inspect_response(processed, true, params.refresh, "cache"));
                }
                // empty cached snapshot is treated as miss; fall through to runtime/offline
            }
        }
    }

    // Runtime fallback: read tools from connected instances in the pool
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
                let server_data = create_runtime_cache_data(
                    &server_info,
                    cached_tools,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                );
                let _ = state.redb_cache.store_server_data(&server_data).await;

                return Ok(create_inspect_response(tools, false, params.refresh, "runtime"));
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::common::create_temporary_instance_for_capability(
        &state,
        &server_info,
        &params,
        super::common::CapabilityType::Tools,
    ).await? {
        return Ok(response);
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
            return Ok(create_inspect_response(processed, true, params.refresh, "cache"));
        }
    }

    // No data available
    Ok(create_inspect_response(Vec::new(), false, params.refresh, "none"))
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
    Query(query): Query<InspectQuery>,
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
