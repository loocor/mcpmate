// Server tools handlers
// Provides handlers for server tool inspect endpoints

use crate::api::{
    models::server::{ServerCapabilityMeta, ServerCapabilityReq, ServerToolsData, ServerToolsResp},
    routes::AppState,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use std::sync::Arc;

use super::capability::{tool_json, tool_json_from_cached};
use super::common::{InspectQuery, create_inspect_response, create_runtime_cache_data};

/// List all tools for a specific server
pub async fn server_tools(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerToolsResp>, StatusCode> {
    let result = server_tools_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for server tools operation
async fn server_tools_core(
    request: &ServerCapabilityReq,
    state: &Arc<AppState>,
) -> Result<ServerToolsResp, StatusCode> {
    // Convert ServerCapabilityReq to InspectQuery for compatibility with existing logic
    let query = InspectQuery {
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
    let (db, server_info, params) = super::common::get_server_info_for_inspect(state, &request.id, &query).await?;

    // Check if server supports tools capability
    if let Some(response) = super::common::check_capability_or_error(
        &db.pool,
        &server_info,
        crate::common::capability::CapabilityToken::Tools,
        &params,
    )
    .await
    {
        let tools_resp = json_to_server_tools_resp(response);
        return Ok(ServerToolsResp::success(tools_resp));
    }

    // Try Redb cache first with freshness policy
    let cache_query = super::common::build_cache_query(&server_info.server_id, &params);

    if let Ok(cache_result) = state.redb_cache.get_server_data(&cache_query).await {
        if cache_result.cache_hit {
            if let Some(data) = cache_result.data {
                let processed: Vec<serde_json::Value> =
                    data.tools.into_iter().map(|t| tool_json_from_cached(&t)).collect();
                if !processed.is_empty() {
                    let result = create_inspect_response(
                        processed,
                        true,
                        params.refresh,
                        crate::common::constants::strategies::CACHE,
                    );
                    let tools_resp = json_to_server_tools_resp(result);
                    return Ok(ServerToolsResp::success(tools_resp));
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

                let result = create_inspect_response(
                    tools,
                    false,
                    params.refresh,
                    crate::common::constants::strategies::RUNTIME,
                );
                let tools_resp = json_to_server_tools_resp(result);
                return Ok(ServerToolsResp::success(tools_resp));
            }
        }
    }

    // Force refresh: create temporary instance if refresh=force and no runtime data found
    if let Some(response) = super::capability::create_temporary_instance_for_capability(
        state,
        &server_info,
        &params,
        super::capability::CapabilityType::Tools,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        let tools_resp = json_to_server_tools_resp(response);
        return Ok(ServerToolsResp::success(tools_resp));
    }

    // Last resort: return any cached tools ignoring freshness if available (support offline access)
    if let Ok(cached_tools) = state.redb_cache.get_server_tools(&server_info.server_id, false).await {
        if !cached_tools.is_empty() {
            let processed: Vec<serde_json::Value> =
                cached_tools.into_iter().map(|t| tool_json_from_cached(&t)).collect();
            let result = create_inspect_response(
                processed,
                true,
                params.refresh,
                crate::common::constants::strategies::CACHE,
            );
            let tools_resp = json_to_server_tools_resp(result);
            return Ok(ServerToolsResp::success(tools_resp));
        }
    }

    // No data available
    let result = create_inspect_response(
        Vec::new(),
        false,
        params.refresh,
        crate::common::constants::strategies::NONE,
    );
    let tools_resp = json_to_server_tools_resp(result);
    Ok(ServerToolsResp::success(tools_resp))
}

/// Helper function to convert Json response to ServerToolsResp
fn json_to_server_tools_resp(json_response: axum::Json<serde_json::Value>) -> ServerToolsData {
    let json_value = json_response.0;

    // Extract data from the JSON response
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

    ServerToolsData { data, state, meta }
}
