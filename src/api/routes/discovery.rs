// Discovery API routes
// Contains route definitions for MCP server capability discovery endpoints

use axum::{Router, routing::get};
use std::sync::Arc;

use super::AppState;
use crate::api::handlers::discovery;

/// Create discovery routes
pub fn routes(state: Arc<AppState>) -> Router {
    // Aggregation routes (new)
    let aggregation_router = Router::new()
        .route("/tools", get(discovery::aggregation::all_tools))
        .route("/prompts", get(discovery::aggregation::all_prompts))
        .route("/resources", get(discovery::aggregation::all_resources))
        .route(
            "/resource-templates",
            get(discovery::aggregation::all_resource_templates),
        );

    // Capabilities routes
    let capabilities_router = Router::new().route(
        "/capabilities/{server_id}",
        get(discovery::capabilities::server_capabilities),
    );

    // Tools routes
    let tools_router = Router::new()
        .route("/tools/{server_id}", get(discovery::tools::server_tools))
        .route(
            "/tools/{server_id}/enabled",
            get(discovery::tools::enabled_server_tools),
        )
        .route(
            "/tools/{server_id}/{tool_id}",
            get(discovery::tools::get_tool_detail),
        );

    // Prompts routes
    let prompts_router = Router::new()
        .route(
            "/prompts/{server_id}",
            get(discovery::prompts::server_prompts),
        )
        .route(
            "/prompts/{server_id}/enabled",
            get(discovery::prompts::enabled_server_prompts),
        );

    // Resources routes
    let resources_router = Router::new()
        .route(
            "/resources/{server_id}",
            get(discovery::resources::server_resources),
        )
        .route(
            "/resources/{server_id}/enabled",
            get(discovery::resources::enabled_server_resources),
        )
        .route(
            "/resource-templates/{server_id}",
            get(discovery::resources::server_resource_templates),
        );

    let discovery_router = Router::new()
        .merge(aggregation_router)
        .merge(capabilities_router)
        .merge(tools_router)
        .merge(prompts_router)
        .merge(resources_router);

    Router::new()
        .nest("/mcp/discovery", discovery_router)
        .with_state(state)
}
