// Server resources handlers
// Provides handlers for server resource inspect endpoints

use axum::{
    extract::{Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{
    handlers::ApiError,
    models::server::{
        ServerCapabilityMeta, ServerCapabilityReq, ServerResourceTemplatesData, ServerResourceTemplatesResp,
        ServerResourcesData, ServerResourcesResp,
    },
    routes::AppState,
};

use super::capability::{CapabilityType, enrich_capability_items, respond_with_enriched};

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
) -> Result<Json<ServerResourcesResp>, ApiError> {
    let result = server_resources_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for listing server resources
#[tracing::instrument(skip(app_state), level = "debug")]
async fn server_resources_core(
    request: &ServerCapabilityReq,
    app_state: &Arc<AppState>,
) -> Result<ServerResourcesResp, ApiError> {
    // Convert request to internal query format
    let query = super::common::InspectQuery {
        refresh: request.refresh.as_ref().map(|r| (*r).into()),
        format: None,
        include_meta: None,
        timeout: None,
    };

    // Validate and get server info using unified function
    let (db, server_info, params) = super::common::get_server_info_for_inspect(app_state, &request.id, &query).await?;

    // CapabilityReadService owns cache and on-demand discovery orchestration.
    let refresh = match params.refresh {
        Some(super::common::RefreshStrategy::Force) => Some(crate::core::capability::runtime::RefreshStrategy::Force),
        _ => Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
    };
    let service = crate::core::capability::read_service::CapabilityReadService::from_runtime(
        db.clone(),
        app_state.connection_pool.clone(),
    );
    let list_result = service
        .list(&crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::Resources,
            server_id: server_info.server_id.clone(),
            refresh,
            timeout: Some(std::time::Duration::from_secs(10)),
            validation_session: None,
            runtime_identity: None,
            connection_selection: None,
            visibility_snapshot: None,
            name_domain: crate::core::capability::runtime::NameDomain::External,
        })
        .await
        .map_err(|error| {
            tracing::error!(server_id = %server_info.server_id, error = %error, "Failed to list resources");
            crate::core::capability::service::map_capability_read_error(&error)
        })?;
    let crate::core::capability::runtime::ListResult { items, meta } = list_result;
    let resource_items = match items {
        crate::core::capability::runtime::CapabilityItems::Resources(items) => items,
        _ => {
            tracing::error!("Capability read service returned non-resource items for resource capability");
            return Err(ApiError::InternalError(
                "Capability read service returned non-resource items for resource capability".to_string(),
            ));
        }
    };
    let json_items = resource_items
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            tracing::error!(server_id = %server_info.server_id, error = %error, "Failed to serialize resources");
            ApiError::InternalError(format!("Failed to serialize resources: {error}"))
        })?;
    let enriched = enrich_capability_items(CapabilityType::Resources, &db.pool, &server_info.server_id, json_items)
        .await
        .map_err(|error| {
            tracing::error!(
                server_id = %server_info.server_id,
                error = %error,
                "Resource naming projection failed"
            );
            ApiError::InternalError(format!("Resource naming projection failed: {error}"))
        })?;
    let response_data = respond_with_enriched(enriched, meta.cache_hit, params.refresh, meta.source.as_str());
    let resources_resp = json_to_server_resources_resp(response_data);
    Ok(ServerResourcesResp::success(resources_resp))
}

/// List resource templates for a specific server with standardized signature
pub async fn server_resource_templates(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerResourceTemplatesResp>, ApiError> {
    let result = server_resource_templates_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for listing server resource templates
#[tracing::instrument(skip(app_state), level = "debug")]
async fn server_resource_templates_core(
    request: &ServerCapabilityReq,
    app_state: &Arc<AppState>,
) -> Result<ServerResourceTemplatesResp, ApiError> {
    // Convert request to internal query format
    let query = super::common::InspectQuery {
        refresh: request.refresh.as_ref().map(|r| (*r).into()),
        format: None,
        include_meta: None,
        timeout: None,
    };

    // Validate and get server info using unified function
    let (db, server_info, params) = super::common::get_server_info_for_inspect(app_state, &request.id, &query).await?;

    // CapabilityReadService owns cache and on-demand discovery orchestration.
    let refresh = match params.refresh {
        Some(super::common::RefreshStrategy::Force) => Some(crate::core::capability::runtime::RefreshStrategy::Force),
        _ => Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
    };
    let service = crate::core::capability::read_service::CapabilityReadService::from_runtime(
        db.clone(),
        app_state.connection_pool.clone(),
    );
    let list_result = service
        .list(&crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::ResourceTemplates,
            server_id: server_info.server_id.clone(),
            refresh,
            timeout: Some(std::time::Duration::from_secs(10)),
            validation_session: None,
            runtime_identity: None,
            connection_selection: None,
            visibility_snapshot: None,
            name_domain: crate::core::capability::runtime::NameDomain::External,
        })
        .await
        .map_err(|error| {
            tracing::error!(server_id = %server_info.server_id, error = %error, "Failed to list resource templates");
            crate::core::capability::service::map_capability_read_error(&error)
        })?;
    let crate::core::capability::runtime::ListResult { items, meta } = list_result;
    let template_items = match items {
        crate::core::capability::runtime::CapabilityItems::ResourceTemplates(items) => items,
        _ => {
            tracing::error!("Capability read service returned non-template items for resource template capability");
            return Err(ApiError::InternalError(
                "Capability read service returned non-template items for resource template capability".to_string(),
            ));
        }
    };
    let json_items = template_items
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            tracing::error!(
                server_id = %server_info.server_id,
                error = %error,
                "Failed to serialize resource templates"
            );
            ApiError::InternalError(format!("Failed to serialize resource templates: {error}"))
        })?;
    let enriched = enrich_capability_items(
        CapabilityType::ResourceTemplates,
        &db.pool,
        &server_info.server_id,
        json_items,
    )
    .await
    .map_err(|error| {
        tracing::error!(
            server_id = %server_info.server_id,
            error = %error,
            "Resource template naming projection failed"
        );
        ApiError::InternalError(format!("Resource template naming projection failed: {error}"))
    })?;
    let response_data = respond_with_enriched(enriched, meta.cache_hit, params.refresh, meta.source.as_str());
    let templates_resp = json_to_server_resource_templates_resp(response_data);
    Ok(ServerResourceTemplatesResp::success(templates_resp))
}
