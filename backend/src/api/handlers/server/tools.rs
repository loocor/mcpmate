// Server tools handlers
// Provides handlers for server tool inspect endpoints

use crate::api::{
    handlers::ApiError,
    models::server::{ServerCapabilityReq, ServerToolsResp},
    routes::AppState,
};
use axum::{
    extract::{Query, State},
    response::Json,
};
use std::sync::Arc;

use super::capability::{CapabilityType, list_server_capability};

/// List all tools for a specific server
pub async fn server_tools(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerToolsResp>, ApiError> {
    let data = list_server_capability(&app_state, &request, CapabilityType::Tools).await?;
    Ok(Json(ServerToolsResp::success(data)))
}
