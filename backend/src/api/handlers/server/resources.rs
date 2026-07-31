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
        ServerCapabilityReq, ServerResourceTemplatesData, ServerResourceTemplatesResp, ServerResourcesData,
        ServerResourcesResp,
    },
    routes::AppState,
};

use super::capability::{CapabilityType, list_server_capability};

/// List all resources for a specific server with standardized signature
pub async fn server_resources(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerResourcesResp>, ApiError> {
    let payload = list_server_capability(&app_state, &request, CapabilityType::Resources).await?;
    Ok(Json(ServerResourcesResp::success(ServerResourcesData {
        items: payload.items,
        state: payload.state,
        degraded_reason: payload.degraded_reason,
        meta: payload.meta,
    })))
}

/// List resource templates for a specific server with standardized signature
pub async fn server_resource_templates(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerResourceTemplatesResp>, ApiError> {
    let payload = list_server_capability(&app_state, &request, CapabilityType::ResourceTemplates).await?;
    Ok(Json(ServerResourceTemplatesResp::success(
        ServerResourceTemplatesData {
            items: payload.items,
            state: payload.state,
            degraded_reason: payload.degraded_reason,
            meta: payload.meta,
        },
    )))
}
