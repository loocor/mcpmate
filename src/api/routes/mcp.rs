// MCP Proxy API routes for MCP server management
// Contains route definitions for MCP server endpoints

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use super::AppState;
use crate::api::handlers::mcp;

/// Create MCP server management routes
pub fn routes(state: Arc<AppState>) -> Router {
    let servers_router = Router::new()
        .route("/", get(mcp::list_servers))
        .route("/:name", get(mcp::get_server));

    let instances_router = Router::new()
        .route("/", get(mcp::list_instances))
        .route("/:id", get(mcp::get_instance))
        .route("/:id/health", get(mcp::check_health))
        .route("/:id/disconnect", post(mcp::disconnect))
        .route("/:id/disconnect/force", post(mcp::force_disconnect))
        .route("/:id/reconnect", post(mcp::reconnect))
        .route("/:id/reconnect/reset", post(mcp::reset_reconnect))
        .route("/:id/cancel", post(mcp::cancel));

    let combined_router = servers_router.nest("/:name/instances", instances_router);

    Router::new().nest("/api/mcp/servers", combined_router.with_state(state))
}
