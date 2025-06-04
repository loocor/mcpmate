// MCP Proxy API routes for MCP specification-compliant information
// Contains route definitions for MCP specs endpoints

use std::sync::Arc;

use axum::{Router, routing::get};

use super::AppState;
use crate::api::handlers::specs;

/// Create MCP specification routes
pub fn routes(state: Arc<AppState>) -> Router {
    let tools_router = Router::new()
        .route("/", get(specs::tools::list_all))
        .route("/{server_name}", get(specs::tools::list_server))
        .route("/{server_name}/{tool_name}", get(specs::tools::get_tool));

    let prompts_router = Router::new()
        .route("/", get(specs::prompts::list_all))
        .route("/{server_name}", get(specs::prompts::list_server));

    let resources_router = Router::new()
        .route("/", get(specs::resources::list_all))
        .route("/{server_name}", get(specs::resources::list_server));

    let resource_templates_router = Router::new()
        .route("/", get(specs::resources::list_templates))
        .route(
            "/{server_name}",
            get(specs::resources::list_server_templates),
        );

    let specs_router = Router::new()
        .nest("/tools", tools_router)
        .nest("/prompts", prompts_router)
        .nest("/resources", resources_router)
        .nest("/resource_templates", resource_templates_router);

    Router::new()
        .nest("/mcp/specs", specs_router)
        .with_state(state)
}
