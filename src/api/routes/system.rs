// MCP Proxy API routes for system management
// Contains route definitions for system endpoints

use std::sync::Arc;

use axum::{Router, routing::get};

use super::AppState;
use crate::api::handlers::system;

/// Create system management routes
pub fn routes(state: Arc<AppState>) -> Router {
    let system_router = Router::new()
        .route("/status", get(system::get_status))
        .route("/metrics", get(system::get_metrics))
        .with_state(state);

    Router::new().nest("/api/system", system_router)
}
