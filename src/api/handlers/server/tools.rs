// Server tools handlers
// Provides handlers for server tool inspect endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{handlers::ApiError, routes::AppState};
use chrono::Utc;

use super::capability::{CapabilityKind, tool_json, tool_json_from_cached};
use super::common::{
    InspectQuery, create_inspect_response, create_runtime_cache_data, get_database_from_state, validate_server_id,
};

/// Convert database error to ApiError
#[inline]
fn db_error(e: impl std::fmt::Display) -> ApiError {
    ApiError::InternalError(format!("Database error: {e}"))
}


/// List all tools for a specific server
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
pub async fn list_tools(
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

    // If the server explicitly declares capabilities and lacks tools, short-circuit
    if let Ok((server_row, _id)) = super::common::get_server_by_identifier(&db.pool, &server_info.server_name).await {
        if server_row.capabilities.is_some()
            && !server_row.has_capability(crate::common::capability::CapabilityToken::Tools)
        {
            return Ok(create_inspect_response(
                Vec::new(),
                false,
                params.refresh,
                "capability-tools-unsupported",
            ));
        }
    }

    // Try Redb cache first with freshness policy
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);

    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> =
                    data.tools.into_iter().map(|t| tool_json_from_cached(&t)).collect();
                if !processed.is_empty() {
                    return super::common::try_enrich_or_fallback(
                        &state,
                        CapabilityKind::Tools,
                        &server_info.server_id,
                        processed,
                        true,
                        params.refresh,
                        crate::common::constants::strategies::CACHE,
                    )
                    .await;
                }
            }
        }
    }

    // Runtime fallback: read tools from connected instances in the pool
    if let Ok(pool) = tokio::time::timeout(
        std::time::Duration::from_millis(crate::common::constants::timeouts::LOCK_MS),
        state.connection_pool.lock(),
    )
    .await
    {
        if let Some(instances) = pool.connections.get(&server_info.server_name) {
            let connected_instances: Vec<_> = instances.values().filter(|conn| conn.is_connected()).collect();

            let mut tools = Vec::new();
            let mut cached_tools = Vec::new();
            let now = Utc::now();

            for conn in connected_instances {
                for t in &conn.tools {
                    let schema = t.schema_as_json_value();
                    tools.push(tool_json(
                        &t.name,
                        t.description.clone().map(|d| d.into_owned()),
                        schema.clone(),
                        None,
                        None,
                    ));

                    // Build cacheable tool info
                    let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
                    cached_tools.push(crate::core::cache::CachedToolInfo {
                        name: t.name.to_string(),
                        description: t.description.clone().map(|d| d.into_owned()),
                        input_schema_json,
                        unique_name: None,
                        enabled: true,
                        cached_at: now,
                    });
                }
            }
            if !tools.is_empty() {
                // Persist into Redb cache for future requests
                let server_data =
                    create_runtime_cache_data(&server_info, cached_tools, Vec::new(), Vec::new(), Vec::new());
                let _ = state.redb_cache.store_server_data(&server_data).await;

                // Enrich tool list with id/unique_name from DB mapping
                return super::common::try_enrich_or_fallback(
                    &state,
                    CapabilityKind::Tools,
                    &server_info.server_id,
                    tools,
                    false,
                    params.refresh,
                    crate::common::constants::strategies::RUNTIME,
                )
                .await;
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::capability::create_temporary_instance_for_capability(
        &state,
        &server_info,
        &params,
        super::capability::CapabilityType::Tools,
    )
    .await?
    {
        return Ok(response);
    }

    // Last resort: return any cached tools ignoring freshness if available (support offline access)
    if let Ok(cached_tools) = state.redb_cache.get_server_tools(&server_info.server_id, false).await {
        if !cached_tools.is_empty() {
            let processed: Vec<serde_json::Value> =
                cached_tools.into_iter().map(|t| tool_json_from_cached(&t)).collect();
            return super::common::try_enrich_or_fallback(
                &state,
                CapabilityKind::Tools,
                &server_info.server_id,
                processed,
                true,
                params.refresh,
                crate::common::constants::strategies::CACHE,
            )
            .await;
        }
    }

    // No data available
    Ok(create_inspect_response(
        Vec::new(),
        false,
        params.refresh,
        crate::common::constants::strategies::NONE,
    ))
}
