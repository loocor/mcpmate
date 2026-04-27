use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::api::handlers::client::handlers::get_client_service;
use crate::api::models::client::{
    ClientDeleteData, ClientDeleteReq, ClientDeleteResp, ClientManageAction, ClientManageData, ClientManageReq,
    ClientManageResp,
};
use crate::api::routes::AppState;
use crate::audit::{AuditAction, AuditEvent, AuditStatus};

pub(crate) async fn invalidate_client_runtime_visibility(identifier: &str) {
    let mut affinity_fragments = vec![format!("#client:{identifier}")];
    let mut removed_sessions = 0_usize;

    if let Some(proxy) = crate::core::proxy::server::ProxyServer::global() {
        let proxy_server = proxy.try_lock().ok().map(|guard| guard.clone());
        if let Some(proxy_server) = proxy_server {
            let session_ids = proxy_server
                .client_context_resolver
                .session_bindings
                .iter()
                .filter(|entry| entry.client_id == identifier)
                .map(|entry| entry.session_id.clone())
                .collect::<Vec<_>>();
            affinity_fragments.extend(session_ids.iter().map(|session_id| format!("#session:{session_id}")));

            for session_id in session_ids {
                proxy_server.remove_downstream_session(&session_id).await;
                removed_sessions += 1;
            }

            let (tools_count, prompts_count, resources_count) = proxy_server.notify_all_list_changed().await;
            tracing::info!(
                client = %identifier,
                removed_sessions,
                tools_count,
                prompts_count,
                resources_count,
                "Invalidated downstream client runtime visibility"
            );
        }
    }

    if let Ok(cache_manager) = crate::core::cache::RedbCacheManager::global() {
        if let Err(error) = cache_manager
            .invalidate_by_affinity_fragments(&affinity_fragments)
            .await
        {
            tracing::warn!(
                client = %identifier,
                error = %error,
                "Failed to invalidate client-filtered cache entries by downstream affinity"
            );
        }
    }
}

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

    if !managed {
        invalidate_client_runtime_visibility(&request.identifier).await;
    }

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

    invalidate_client_runtime_visibility(&request.identifier).await;

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
