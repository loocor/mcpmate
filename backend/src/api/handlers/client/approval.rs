use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::api::handlers::client::handlers::get_client_service;
use crate::api::handlers::client::manage::invalidate_client_runtime_visibility;
use crate::api::models::client::{ApprovalRequest, ApprovalResponse};
use crate::api::routes::AppState;
use crate::audit::{AuditAction, AuditEvent, AuditStatus};

pub async fn approve_client(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ApprovalRequest>,
) -> Result<Json<ApprovalResponse>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let (status, managed) = service.approve_client(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to approve client {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        AuditEvent::new(AuditAction::ClientApprove, AuditStatus::Success)
            .with_http_route("POST", "/api/client/manage/approve")
            .with_client_id(request.identifier.clone())
            .with_target(request.identifier.clone())
            .with_data(serde_json::json!({ "approval_status": "approved", "managed": managed }))
            .build(),
    )
    .await;

    invalidate_client_runtime_visibility(&request.identifier).await;

    Ok(Json(ApprovalResponse {
        identifier: request.identifier,
        status,
        managed,
    }))
}

pub async fn reject_client(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ApprovalRequest>,
) -> Result<Json<ApprovalResponse>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let (status, managed) = service.reject_client(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to reject client {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        AuditEvent::new(AuditAction::ClientReject, AuditStatus::Success)
            .with_http_route("POST", "/api/client/manage/reject")
            .with_client_id(request.identifier.clone())
            .with_target(request.identifier.clone())
            .with_data(serde_json::json!({ "approval_status": "rejected" }))
            .build(),
    )
    .await;

    invalidate_client_runtime_visibility(&request.identifier).await;

    Ok(Json(ApprovalResponse {
        identifier: request.identifier,
        status,
        managed,
    }))
}

pub async fn suspend_client(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ApprovalRequest>,
) -> Result<Json<ApprovalResponse>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let (status, managed) = service.suspend_client(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to suspend client {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        AuditEvent::new(AuditAction::ClientSuspend, AuditStatus::Success)
            .with_http_route("POST", "/api/client/manage/suspend")
            .with_client_id(request.identifier.clone())
            .with_target(request.identifier.clone())
            .with_data(serde_json::json!({ "approval_status": "suspended", "managed": managed }))
            .build(),
    )
    .await;

    invalidate_client_runtime_visibility(&request.identifier).await;

    Ok(Json(ApprovalResponse {
        identifier: request.identifier,
        status,
        managed,
    }))
}
