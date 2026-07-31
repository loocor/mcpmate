use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use super::get_client_service;
use super::manage::invalidate_client_runtime_visibility;
use crate::api::models::client::{ApprovalRequest, ApprovalResponse};
use crate::api::routes::AppState;
use crate::audit::{AuditAction, AuditEvent, AuditStatus};

pub async fn approve_client(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ApprovalRequest>,
) -> Result<Json<ApprovalResponse>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let (status, materialization) = service.approve_client(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to approve client {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        AuditEvent::new(AuditAction::ClientApprove, AuditStatus::Success)
            .with_http_route("POST", "/api/client/manage/approve")
            .with_client_id(request.identifier.clone())
            .with_target(request.identifier.clone())
            .with_data(serde_json::json!({ "approval_status": "approved" }))
            .build(),
    )
    .await;
    if let Some(commit) = materialization
        && commit.effective_surface_changed
        && let Some(binding) = commit.binding
    {
        crate::audit::interceptor::emit_event(
            app_state.audit_service.as_ref(),
            AuditEvent::new(AuditAction::SurfacePublish, AuditStatus::Success)
                .with_http_route("POST", "/api/client/manage/approve")
                .with_actor("client_management")
                .with_client_id(binding.consumer_id.clone())
                .with_target(binding.active_publication_id)
                .with_data(serde_json::json!({
                    "binding_generation": binding.generation,
                    "proposal_id": commit.proposal_id,
                    "trigger": "consumer_approval",
                }))
                .build(),
        )
        .await;
    }

    invalidate_client_runtime_visibility(&request.identifier).await;

    Ok(Json(ApprovalResponse {
        identifier: request.identifier,
        status,
    }))
}

pub async fn suspend_client(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ApprovalRequest>,
) -> Result<Json<ApprovalResponse>, StatusCode> {
    let service = get_client_service(&app_state)?;

    let status = service.suspend_client(&request.identifier).await.map_err(|err| {
        tracing::error!("Failed to suspend client {}: {}", request.identifier, err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        AuditEvent::new(AuditAction::ClientSuspend, AuditStatus::Success)
            .with_http_route("POST", "/api/client/manage/suspend")
            .with_client_id(request.identifier.clone())
            .with_target(request.identifier.clone())
            .with_data(serde_json::json!({ "approval_status": "suspended" }))
            .build(),
    )
    .await;

    invalidate_client_runtime_visibility(&request.identifier).await;

    Ok(Json(ApprovalResponse {
        identifier: request.identifier,
        status,
    }))
}
