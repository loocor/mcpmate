// MCP Proxy API routes for system management
// Contains route definitions for system endpoints

use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use super::AppState;
use crate::aide_wrapper;
use crate::api::handlers::system;
use crate::api::models::system::ManagementActionResp;
use crate::api::models::system::{SystemMetricsResp, SystemPortsResp, SystemStatusResp};

// Generate aide-compatible wrappers for system handlers
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
    system::get_ports,
    SystemPortsResp,
    "Get runtime REST and MCP listener ports"
);

// Management controls under system group for consistency
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

/// Create system management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/system/status", get_with(get_status_aide, get_status_docs))
        .api_route("/system/ports", get_with(get_ports_aide, get_ports_docs))
        .api_route("/system/metrics", get_with(get_metrics_aide, get_metrics_docs))
        .api_route("/system/shutdown", post_with(shutdown_aide, shutdown_docs))
        .api_route("/system/restart", post_with(restart_aide, restart_docs))
        .with_state(state)
}
