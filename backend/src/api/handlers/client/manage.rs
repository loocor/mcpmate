use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::api::handlers::client::handlers::get_client_service;
use crate::api::models::client::{ClientManageAction, ClientManageData, ClientManageReq, ClientManageResp};
use crate::api::routes::AppState;
use crate::audit::{AuditAction, AuditEvent, AuditStatus};

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

    let action = match request.action {
        ClientManageAction::Enable => AuditAction::ClientManageEnable,
        ClientManageAction::Disable => AuditAction::ClientManageDisable,
    };

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        AuditEvent::new(action, AuditStatus::Success)
            .with_http_route("POST", "/api/client/manage")
            .with_client_id(request.identifier.clone())
            .with_target(request.identifier.clone())
            .with_data(serde_json::json!({ "managed": result }))
            .build(),
    )
    .await;

    let data = ClientManageData {
        identifier: request.identifier,
        managed: result,
    };

    Ok(Json(ClientManageResp::success(data)))
}
