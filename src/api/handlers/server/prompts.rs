// Server prompts handlers
// Provides handlers for server prompt inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{handlers::ApiError, routes::AppState};

use super::common::{
    InspectParams, RefreshStrategy, get_database_from_state, register_session_if_needed, resolve_server_identifier, validate_server_id,
};

/// Query parameters for prompts endpoints
#[derive(Debug, serde::Deserialize)]
pub struct PromptsQuery {
    /// Refresh strategy for prompt queries
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

impl PromptsQuery {
    /// Convert to InspectParams
    pub fn to_params(&self) -> Result<InspectParams, ApiError> {
        // Map instance_type to refresh strategy for backward compatibility
        let mapped_refresh = if let Some(ref it) = self.instance_type {
            match it.to_lowercase().as_str() {
                "production" => Some(RefreshStrategy::CacheFirst),
                "exploration" => Some(RefreshStrategy::RefreshIfStale),
                "validation" => Some(RefreshStrategy::Force),
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

/// List all prompts for a specific server
///
/// Returns a list of prompts available on the specified server with
/// configurable filtering and formatting options.
///
/// Supports both server_name and server_id as identifier.
pub async fn list_prompts(
    State(state): State<Arc<AppState>>,
    Path(identifier): Path<String>,
    Query(query): Query<PromptsQuery>,
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

    // Try Redb cache first when strategy prefers cache
    let prefer_cache = params.refresh.unwrap_or_default() == RefreshStrategy::CacheFirst;
    if prefer_cache {
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
                return Ok(Json(serde_json::json!({
                    "data": processed,
                    "meta": { "cache_hit": true, "strategy": params.refresh.unwrap_or_default() }
                })));
            }
        }
    }

    // Return empty result if no data available from cache or runtime
    Ok(Json(serde_json::json!({
        "data": [],
        "meta": { "cache_hit": false, "strategy": params.refresh.unwrap_or_default() }
    })))
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
    Query(query): Query<PromptsQuery>,
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

