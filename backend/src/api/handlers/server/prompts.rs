// Server prompts handlers
// Provides handlers for server prompt inspect endpoints

use axum::{
    extract::{Query, State},
    response::Json,
};
use std::sync::Arc;

use crate::api::{
    handlers::ApiError,
    models::server::{ServerCapabilityReq, ServerPromptsData, ServerPromptsResp},
    routes::AppState,
};

use super::capability::{CapabilityType, list_server_capability};

/// List all prompts for a specific server with standardized signature
pub async fn server_prompts(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerPromptsResp>, ApiError> {
    let payload = list_server_capability(&app_state, &request, CapabilityType::Prompts).await?;
    Ok(Json(ServerPromptsResp::success(ServerPromptsData {
        items: payload.items,
        state: payload.state,
        meta: payload.meta,
    })))
}
