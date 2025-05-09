// MCP Proxy API routes for MCP tool management
// Contains route definitions for MCP tool endpoints

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use super::AppState;
use crate::api::handlers::tool;

/// Create MCP tool management routes
pub fn routes(state: Arc<AppState>) -> Router {
    let tools_router = Router::new()
        .route("/", get(tool::list_tools))
        .route("/{server_name}/{tool_name}", get(tool::get_tool))
        .route("/{server_name}/{tool_name}/enable", post(tool::enable_tool))
        .route(
            "/{server_name}/{tool_name}/disable",
            post(tool::disable_tool),
        )
        .route("/{server_name}/{tool_name}", post(tool::update_tool));

    Router::new()
        .nest("/api/mcp/tools", tools_router)
        .with_state(state)
}
