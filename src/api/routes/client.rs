use crate::api::handlers::client;
use crate::api::models::client::{
    ClientBackupActionResp, ClientBackupListReq, ClientBackupListResp, ClientBackupOperateReq, ClientBackupPolicyReq,
    ClientBackupPolicyResp, ClientBackupPolicySetReq, ClientCheckReq, ClientCheckResp, ClientConfigReq,
    ClientConfigResp, ClientConfigRestoreReq, ClientConfigUpdateReq, ClientConfigUpdateResp, ClientManageReq,
    ClientManageResp,
};
use crate::api::routes::AppState;
use crate::{aide_wrapper_payload, aide_wrapper_query};
use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};
use axum::http::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{extract::State, middleware};
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use std::sync::Arc;

// Generate aide-compatible wrapper for client check (with query parameters)
aide_wrapper_query!(
    client::list,
    ClientCheckReq,
    ClientCheckResp,
    "Get all client with optional force refresh."
);

// Configuration endpoints
aide_wrapper_query!(
    client::config_details,
    ClientConfigReq,
    ClientConfigResp,
    "Get client configuration details with optional server import"
);

aide_wrapper_payload!(
    client::config_apply,
    ClientConfigUpdateReq,
    ClientConfigUpdateResp,
    "Apply client configuration with specified settings"
);

aide_wrapper_payload!(
    client::config_restore,
    ClientConfigRestoreReq,
    ClientBackupActionResp,
    "Restore a client configuration from backup"
);

// Management toggle
aide_wrapper_payload!(
    client::manage,
    ClientManageReq,
    ClientManageResp,
    "Enable or disable MCPMate management for a client"
);

// Backup administration
aide_wrapper_query!(
    client::list_backups,
    ClientBackupListReq,
    ClientBackupListResp,
    "List stored configuration backups"
);

aide_wrapper_payload!(
    client::delete_backup,
    ClientBackupOperateReq,
    ClientBackupActionResp,
    "Delete a configuration backup"
);

aide_wrapper_query!(
    client::get_backup_policy,
    ClientBackupPolicyReq,
    ClientBackupPolicyResp,
    "Get backup retention policy for a client"
);

aide_wrapper_payload!(
    client::set_backup_policy,
    ClientBackupPolicySetReq,
    ClientBackupPolicyResp,
    "Set backup retention policy for a client"
);

/// Create client management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    // Build routes then attach a per-request template-reload middleware
    let router = ApiRouter::new()
        .api_route("/client/list", get_with(list_aide, list_docs))
        .api_route(
            "/client/config/details",
            get_with(config_details_aide, config_details_docs),
        )
        .api_route("/client/config/apply", post_with(config_apply_aide, config_apply_docs))
        .api_route(
            "/client/config/restore",
            post_with(config_restore_aide, config_restore_docs),
        )
        .api_route("/client/manage", post_with(manage_aide, manage_docs))
        .api_route("/client/backups/list", get_with(list_backups_aide, list_backups_docs))
        .api_route(
            "/client/backups/delete",
            post_with(delete_backup_aide, delete_backup_docs),
        )
        .api_route(
            "/client/backups/policy",
            get_with(get_backup_policy_aide, get_backup_policy_docs)
                .post_with(set_backup_policy_aide, set_backup_policy_docs),
        )
        .with_state(state.clone());

    router.route_layer(middleware::from_fn_with_state(state, reload_templates_middleware))
}

/// Per-request middleware to ensure latest on-disk client templates are loaded
async fn reload_templates_middleware(
    State(app_state): State<Arc<AppState>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Short TTL to coalesce bursty requests and reduce I/O
    const TEMPLATE_RELOAD_TTL: Duration = Duration::from_secs(crate::common::constants::timeouts::TEMPLATE_RELOAD_TTL_SEC);

    if let Some(service) = &app_state.client_service {
        // Fast path: skip reload if still fresh
        {
            let guard = RELOAD_FRESHNESS.lock().await;
            if guard.is_fresh(TEMPLATE_RELOAD_TTL) {
                return next.run(req).await;
            }
        }

        // Coalesce concurrent reloads with a global async mutex
        {
            let _lock = RELOAD_LOCK.lock().await;
            // Re-check after acquiring the lock to avoid redundant reloads
            {
                let guard = RELOAD_FRESHNESS.lock().await;
                if guard.is_fresh(TEMPLATE_RELOAD_TTL) {
                    return next.run(req).await;
                }
            }

            // Otherwise reload and update freshness
            if let Err(err) = service.reload_templates().await {
                tracing::error!("Failed to reload client templates (middleware): {}", err);
                return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            let mut guard = RELOAD_FRESHNESS.lock().await;
            guard.mark_now();
        }
    } else {
        // Keep behavior consistent with handlers: if service missing, let handler decide (likely 503)
    }

    next.run(req).await
}

// Global freshness tracker for /api/client/** template reloads
static RELOAD_FRESHNESS: Lazy<Mutex<ReloadFreshness>> = Lazy::new(|| Mutex::new(ReloadFreshness::default()));
static RELOAD_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

#[derive(Default)]
struct ReloadFreshness {
    last: Option<Instant>,
}

impl ReloadFreshness {
    fn is_fresh(&self, ttl: Duration) -> bool {
        match self.last {
            Some(t) => t.elapsed() < ttl,
            None => false,
        }
    }

    fn mark_now(&mut self) {
        self.last = Some(Instant::now());
    }
}
