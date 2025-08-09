// Server tools handlers
// Provides handlers for server tool inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{handlers::ApiError, routes::AppState};

use super::common::{
    InspectParams, RefreshStrategy, get_database_from_state, register_session_if_needed, resolve_server_identifier,
    validate_server_id,
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
        // Map instance_type to refresh strategy for backward compatibility
        let mapped_refresh = if let Some(ref it) = self.instance_type {
            match it.to_lowercase().as_str() {
                // Production -> prefer cached data
                "production" => Some(super::common::RefreshStrategy::CacheFirst),
                // Exploration -> refresh if stale
                "exploration" => Some(super::common::RefreshStrategy::RefreshIfStale),
                // Validation -> always force refresh
                "validation" => Some(super::common::RefreshStrategy::Force),
                _ => self.refresh,
            }
        } else {
            self.refresh
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

    // Try Redb cache first when strategy prefers cache
    let prefer_cache = params.refresh.unwrap_or_default() == RefreshStrategy::CacheFirst;
    if prefer_cache {
        if let Ok(cached_tools) = state.redb_cache.get_server_tools(&server_info.server_id, false).await {
            if !cached_tools.is_empty() {
                let processed: Vec<serde_json::Value> = cached_tools
                    .into_iter()
                    .map(|t| {
                        let desc = t.description.clone();
                        let unique_name = t.unique_name.clone();
                        let schema = t.input_schema().unwrap_or_else(|_| serde_json::json!({}));
                        serde_json::json!({
                            "name": t.name,
                            "description": desc,
                            "input_schema": schema,
                            "unique_name": unique_name
                        })
                    })
                    .collect();
                return Ok(Json(serde_json::json!({
                    "data": processed,
                    "meta": { "cache_hit": true, "strategy": params.refresh.unwrap_or_default() }
                })));
            }
        }
    }

    // Runtime fallback: read tools from connected instances in the pool (no inspect)
    if let Ok(pool) = tokio::time::timeout(std::time::Duration::from_millis(500), state.connection_pool.lock()).await {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            // Collect tools from any connected instance
            let mut tools: Vec<serde_json::Value> = Vec::new();
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
                }
            }
            if !tools.is_empty() {
                return Ok(Json(serde_json::json!({
                    "data": tools,
                    "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default(), "source": "runtime" }
                })));
            }
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
                if !conn.is_connected() { continue; }
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
