// Server prompts handlers
// Provides handlers for server prompt inspect endpoints

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;

use crate::api::{
    models::server::{ServerCapabilityMeta, ServerCapabilityReq, ServerPromptsData, ServerPromptsResp},
    routes::AppState,
};

use super::capability::{CapabilityType, enrich_capability_items, respond_with_enriched};
use super::common::{create_inspect_response, get_database_from_state};

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

    // Use CapabilityService (REDB-first → runtime → optional temporary via pool)
    let refresh = match params.refresh {
        Some(super::common::RefreshStrategy::Force) => Some(crate::core::capability::runtime::RefreshStrategy::Force),
        _ => Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
    };
    let service = crate::core::capability::CapabilityService::new(
        app_state.redb_cache.clone(),
        app_state.connection_pool.clone(),
        db.clone(),
    );
    let list_result = service
        .list(&crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::Prompts,
            server_id: server_info.server_id.clone(),
            refresh,
            timeout: Some(std::time::Duration::from_secs(10)),
            validation_session: Some(crate::core::capability::service::CAPABILITY_VALIDATION_SESSION.to_string()),
        })
        .await;
    // TODO: introduce unified naming module for prompts to avoid potential name collisions (similar to tools)
    match list_result {
        Ok(list_result) => {
            let crate::core::capability::runtime::ListResult { items, meta } = list_result;
            if let Some(prompt_items) = items.into_prompts() {
                if !prompt_items.is_empty() {
                    let json_items: Vec<serde_json::Value> = prompt_items
                        .into_iter()
                        .map(|prompt| serde_json::to_value(prompt).unwrap_or(serde_json::Value::Null))
                        .collect();

                    if let Ok(db) = get_database_from_state(app_state) {
                        let enriched = enrich_capability_items(
                            CapabilityType::Prompts,
                            &db.pool,
                            &server_info.server_id,
                            json_items.clone(),
                        )
                        .await;
                        let response_data =
                            respond_with_enriched(enriched, meta.cache_hit, params.refresh, meta.source.as_str());
                        let prompts_resp = json_to_server_prompts_resp(response_data);
                        return Ok(ServerPromptsResp::success(prompts_resp));
                    }

                    let response_data =
                        create_inspect_response(json_items, meta.cache_hit, params.refresh, meta.source.as_str());
                    let prompts_resp = json_to_server_prompts_resp(response_data);
                    return Ok(ServerPromptsResp::success(prompts_resp));
                }
            } else {
                tracing::warn!("Capability runtime returned non-prompt items for prompt capability");
            }

            // Temporary instance fallback is handled by CapabilityService; handler remains unaware of pool
        }
        Err(e) => {
            tracing::error!("Failed to list prompts via unified entry: {}", e);
        }
    }

    // No handler-side temporary creation
    // No offline fallback to avoid stale uncertainty; return empty if still not available
    let response_data = create_inspect_response(
        Vec::new(),
        false,
        params.refresh,
        crate::common::constants::strategies::NONE,
    );
    let prompts_resp = json_to_server_prompts_resp(response_data);
    Ok(ServerPromptsResp::success(prompts_resp))
}
