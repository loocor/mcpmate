// MCP Proxy API routes for system management
// Contains route definitions for system endpoints

use std::sync::Arc;

use aide::axum::{ApiRouter, routing::get_with};

use super::AppState;
use crate::aide_wrapper;
use crate::api::handlers::system;
use crate::api::models::system::{StatusResponse, SystemMetricsResponse};

// Generate aide-compatible wrappers for system handlers
aide_wrapper!(
    system::get_status,
    StatusResponse,
    "Get system status including uptime and server counts"
);

aide_wrapper!(
    system::get_metrics,
    SystemMetricsResponse,
    "Get detailed system metrics including CPU, memory, and instance counts"
);

/// Create system management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/system/status", get_with(get_status_aide, get_status_docs))
        .api_route("/system/metrics", get_with(get_metrics_aide, get_metrics_docs))
        .with_state(state)
}
