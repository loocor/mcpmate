// MCP Proxy API routes for MCP specification-compliant information
// Contains route definitions for MCP specs endpoints

use axum::{routing::get, Router};
use std::sync::Arc;

use super::AppState;
use crate::api::handlers::specs;

/// Create MCP specification routes
pub fn routes(state: Arc<AppState>) -> Router {
    let tools_router = Router::new()
        .route("/", get(specs::tools::list_all))
        .route("/{server_name}", get(specs::tools::list_server))
        .route("/{server_name}/{tool_name}", get(specs::tools::get_tool));

    let specs_router = Router::new().nest("/tools", tools_router);

    Router::new()
        .nest("/api/mcp/specs", specs_router)
        .with_state(state)
}
