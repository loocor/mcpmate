// MCP Proxy API routes for system management
// Contains route definitions for system endpoints

use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use super::AppState;
use crate::api::handlers::system;
use crate::api::models::system::{
    ManagementActionResp, SystemMetricsResp, SystemSettingsResp, SystemSettingsUpdateReq, SystemStatusResp,
};
use crate::{aide_wrapper, aide_wrapper_payload};

aide_wrapper!(
    system::get_status,
    SystemStatusResp,
    "Get system status including uptime and server counts"
);

aide_wrapper!(
    system::get_metrics,
    SystemMetricsResp,
    "Get detailed system metrics including CPU, memory, and instance counts"
);

aide_wrapper!(
    system::get_settings,
    SystemSettingsResp,
    "Get structured system settings"
);

aide_wrapper_payload!(
    system::set_settings,
    SystemSettingsUpdateReq,
    SystemSettingsResp,
    "Update one or more structured system settings fields"
);

aide_wrapper!(
    system::shutdown,
    ManagementActionResp,
    "Request graceful shutdown of MCP proxy service"
);

aide_wrapper!(
    system::restart,
    ManagementActionResp,
    "Restart MCP proxy service on configured port"
);

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/system/status", get_with(get_status_aide, get_status_docs))
        .api_route(
            "/system/settings",
            get_with(get_settings_aide, get_settings_docs).post_with(set_settings_aide, set_settings_docs),
        )
        .api_route("/system/metrics", get_with(get_metrics_aide, get_metrics_docs))
        .api_route("/system/shutdown", post_with(shutdown_aide, shutdown_docs))
        .api_route("/system/restart", post_with(restart_aide, restart_docs))
        .with_state(state)
}
