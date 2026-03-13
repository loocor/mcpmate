use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::api::handlers::client::handlers::get_client_service;
use crate::api::models::client::{ClientManageAction, ClientManageData, ClientManageReq, ClientManageResp};
use crate::api::routes::AppState;

pub async fn manage(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientManageReq>,
) -> Result<Json<ClientManageResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let managed = match request.action {
        ClientManageAction::Enable => true,
        ClientManageAction::Disable => false,
    };

    let result = service
        .set_client_managed(&request.identifier, managed)
        .await
        .map_err(|err| {
            tracing::error!("Failed to update managed state for {}: {}", request.identifier, err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let data = ClientManageData {
        identifier: request.identifier,
        managed: result,
    };

    Ok(Json(ClientManageResp::success(data)))
}
