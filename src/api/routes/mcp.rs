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
        .route("/:name", get(mcp::get_server))
        .route("/:name/details", get(mcp::get_server_details))
        .route("/:name/health", get(mcp::check_server_health))
        .route("/:name/pause", post(mcp::pause_server))
        .route("/:name/enable", post(mcp::enable_server))
        .route("/:name/disable", post(mcp::disable_server))
        .route("/:name/connect", post(mcp::connect_server))
        .route("/:name/reconnect", post(mcp::reconnect_server))
        .route("/:name/disconnect", post(mcp::disconnect_server))
        .with_state(state);

    Router::new().nest("/api/mcp/servers", servers_router)
}
