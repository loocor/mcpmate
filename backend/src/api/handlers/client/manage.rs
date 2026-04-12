use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::api::handlers::client::handlers::get_client_service;
use crate::api::models::client::{
    ClientDeleteData, ClientDeleteReq, ClientDeleteResp, ClientManageAction, ClientManageData, ClientManageReq,
    ClientManageResp,
};
use crate::api::routes::AppState;
use crate::audit::{AuditAction, AuditEvent, AuditStatus};

pub async fn manage(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientManageReq>,
) -> Result<Json<ClientManageResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let managed = matches!(request.action, ClientManageAction::Enable);

    let result = service
        .set_client_managed(&request.identifier, managed)
        .await
        .map_err(|err| {
            tracing::error!("Failed to update managed state for {}: {}", request.identifier, err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let action = if managed {
        AuditAction::ClientManageEnable
    } else {
        AuditAction::ClientManageDisable
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

pub async fn delete_client(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<ClientDeleteReq>,
) -> Result<Json<ClientDeleteResp>, StatusCode> {
    let service = get_client_service(&app_state)?;
    let deleted = service.delete_client_record(&request.identifier).await.map_err(|err| {
        tracing::error!(client = %request.identifier, error = %err, "Failed to delete client record");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if !deleted {
        return Err(StatusCode::NOT_FOUND);
    }

    // Release lock before spawning downstream session cleanup to avoid holding the global proxy lock
    // during async work. Follows the repo pattern: do synchronous work, then release lock before async.
    if let Some(proxy) = crate::core::proxy::server::ProxyServer::global() {
        if proxy.try_lock().is_ok() {
            // If we can acquire the lock, the proxy is responsive. Now spawn cleanup in the background.
            let client_id = request.identifier.clone();
            tokio::spawn(async move {
                if let Ok(guard) = proxy.try_lock() {
                    let removed_sessions = guard.remove_downstream_sessions_for_client(&client_id).await;
                    tracing::info!(client = %client_id, removed_sessions, "Removed downstream sessions after client deletion");
                }
            });
        }
    }

    crate::audit::interceptor::emit_event(
        app_state.audit_service.as_ref(),
        AuditEvent::new(AuditAction::ClientDelete, AuditStatus::Success)
            .with_http_route("POST", "/api/client/delete")
            .with_client_id(request.identifier.clone())
            .with_target(request.identifier.clone())
            .build(),
    )
    .await;

    Ok(Json(ClientDeleteResp::success(ClientDeleteData {
        identifier: request.identifier,
        deleted,
    })))
}
