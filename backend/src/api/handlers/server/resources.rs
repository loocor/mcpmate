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

use super::capability::{CapabilityType, enrich_capability_items, respond_with_enriched};
use super::common::{create_inspect_response, get_database_from_state};

/// Helper function to convert Json response to ServerResourcesResp
fn json_to_server_resources_resp(json_response: axum::Json<serde_json::Value>) -> ServerResourcesData {
    let json_value = json_response.0;

    let items = json_value
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

    ServerResourcesData { items, state, meta }
}

/// Helper function to convert Json response to ServerResourceTemplatesResp
fn json_to_server_resource_templates_resp(json_response: axum::Json<serde_json::Value>) -> ServerResourceTemplatesData {
    let json_value = json_response.0;

    let items = json_value
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

    ServerResourceTemplatesData { items, state, meta }
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
        refresh: request.refresh.as_ref().map(|r| (*r).into()),
        format: None,
        include_meta: None,
        timeout: None,
    };

    // Validate and get server info using unified function
    let (db, server_info, params) = super::common::get_server_info_for_inspect(app_state, &request.id, &query).await?;

    // Check if server supports resource templates capability
    if let Some(response) = super::common::check_capability_or_error(
        &db.pool,
        &server_info,
        crate::common::capability::CapabilityToken::Resources,
        &params,
    )
    .await
    {
        let resources_resp = json_to_server_resources_resp(response);
        return Ok(ServerResourcesResp::success(resources_resp));
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
            capability: crate::core::capability::CapabilityType::Resources,
            server_id: server_info.server_id.clone(),
            refresh,
            timeout: Some(std::time::Duration::from_secs(10)),
            validation_session: Some(crate::core::capability::service::CAPABILITY_VALIDATION_SESSION.to_string()),
        })
        .await;

    // TODO: unify resource naming/unique URI strategy to prevent collisions (parity with tools naming module)
    match list_result {
        Ok(list_result) => {
            let crate::core::capability::runtime::ListResult { items, meta } = list_result;
            if let Some(resource_items) = items.into_resources() {
                if !resource_items.is_empty() {
                    let json_items: Vec<serde_json::Value> = resource_items
                        .into_iter()
                        .map(|resource| serde_json::to_value(resource).unwrap_or(serde_json::Value::Null))
                        .collect();

                    if let Ok(db) = get_database_from_state(app_state) {
                        let enriched = enrich_capability_items(
                            CapabilityType::Resources,
                            &db.pool,
                            &server_info.server_id,
                            json_items.clone(),
                        )
                        .await;
                        let response_data =
                            respond_with_enriched(enriched, meta.cache_hit, params.refresh, meta.source.as_str());
                        let resources_resp = json_to_server_resources_resp(response_data);
                        return Ok(ServerResourcesResp::success(resources_resp));
                    }

                    let response_data =
                        create_inspect_response(json_items, meta.cache_hit, params.refresh, meta.source.as_str());
                    let resources_resp = json_to_server_resources_resp(response_data);
                    return Ok(ServerResourcesResp::success(resources_resp));
                }
            } else {
                tracing::warn!("Capability runtime returned non-resource items for resource capability");
            }

            // Temporary instance fallback is handled by CapabilityService; handler remains unaware of pool
        }
        Err(e) => {
            tracing::error!("Failed to list resources via unified entry: {}", e);
        }
    }

    // Runtime fallback is handled by the capability runtime pipeline; no duplicate fetch here

    // No offline fallback; return empty if still not available
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
        refresh: request.refresh.as_ref().map(|r| (*r).into()),
        format: None,
        include_meta: None,
        timeout: None,
    };

    // Validate and get server info using unified function
    let (db, server_info, params) = super::common::get_server_info_for_inspect(app_state, &request.id, &query).await?;

    // Check if server supports resource templates capability
    if let Some(response) = super::common::check_capability_or_error(
        &db.pool,
        &server_info,
        crate::common::capability::CapabilityToken::Resources,
        &params,
    )
    .await
    {
        let templates_resp = json_to_server_resource_templates_resp(response);
        return Ok(ServerResourceTemplatesResp::success(templates_resp));
    }

    // Use capability runtime pipeline (REDB-first)
    let refresh = match params.refresh {
        Some(super::common::RefreshStrategy::Force) => Some(crate::core::capability::runtime::RefreshStrategy::Force),
        _ => Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
    };
    let list_result = crate::core::capability::runtime::list(
        &crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::ResourceTemplates,
            server_id: server_info.server_id.clone(),
            refresh,
            timeout: Some(std::time::Duration::from_secs(10)),
            validation_session: Some(crate::core::capability::service::CAPABILITY_VALIDATION_SESSION.to_string()),
        },
        &app_state.redb_cache,
        &app_state.connection_pool,
        &db,
    )
    .await;
    match list_result {
        // TODO: align resource template naming with generalized naming module when extended beyond tools
        Ok(list_result) => {
            let crate::core::capability::runtime::ListResult { items, meta } = list_result;
            if let Some(template_items) = items.into_resource_templates() {
                if !template_items.is_empty() {
                    let json_items: Vec<serde_json::Value> = template_items
                        .into_iter()
                        .map(|template| serde_json::to_value(template).unwrap_or(serde_json::Value::Null))
                        .collect();

                    if let Ok(db) = get_database_from_state(app_state) {
                        let enriched = enrich_capability_items(
                            CapabilityType::ResourceTemplates,
                            &db.pool,
                            &server_info.server_id,
                            json_items.clone(),
                        )
                        .await;
                        let response_data =
                            respond_with_enriched(enriched, meta.cache_hit, params.refresh, meta.source.as_str());
                        let templates_resp = json_to_server_resource_templates_resp(response_data);
                        return Ok(ServerResourceTemplatesResp::success(templates_resp));
                    }

                    let response_data =
                        create_inspect_response(json_items, meta.cache_hit, params.refresh, meta.source.as_str());
                    let templates_resp = json_to_server_resource_templates_resp(response_data);
                    return Ok(ServerResourceTemplatesResp::success(templates_resp));
                }
            } else {
                tracing::warn!("Capability runtime returned non-template items for resource template capability");
            }

            let should_try_temp = !meta.had_peer;
            if should_try_temp {
                if let Some(response) = super::capability::create_temporary_instance_for_capability(
                    app_state,
                    &server_info,
                    &params,
                    super::capability::CapabilityType::ResourceTemplates,
                    should_try_temp,
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                {
                    let templates_resp = json_to_server_resource_templates_resp(response);
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
        Err(e) => {
            tracing::error!("Failed to list resource templates via unified entry: {}", e);
            let response_data = create_inspect_response(
                Vec::new(),
                false,
                params.refresh,
                crate::common::constants::strategies::NONE,
            );
            let templates_resp = json_to_server_resource_templates_resp(response_data);
            Ok(ServerResourceTemplatesResp::success(templates_resp))
        }
    }
}
