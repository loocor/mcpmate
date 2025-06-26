// MCP Proxy API routes for MCP server management
// Contains route definitions for MCP server endpoints

use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use super::AppState;
use crate::api::handlers::{instance, server};

/// Create MCP server management routes
pub fn routes(state: Arc<AppState>) -> Router {
    let servers_router = Router::new()
        // Basic server management
        .route("/", get(server::list_servers))
        .route("/", post(server::create_server))
        .route("/import", post(server::import_servers))

        // Server-specific operations (supports both name and ID)
        .route("/{identifier}", get(server::get_server))
        .route("/{identifier}", put(server::update_server))
        .route("/{identifier}", delete(server::delete_server))
        .route("/{identifier}/enable", post(server::enable_server))
        .route("/{identifier}/disable", post(server::disable_server))

        // Discovery capabilities (integrated from discovery module)
        .route("/{identifier}/capabilities", get(server::get_capabilities))
        .route("/{identifier}/tools", get(server::list_tools))
        .route("/{identifier}/tools/{tool_name}", get(server::get_tool_detail))
        .route("/{identifier}/resources", get(server::list_resources))
        .route("/{identifier}/resource-templates", get(server::list_resource_templates))
        .route("/{identifier}/prompts", get(server::list_prompts))
        .route("/{identifier}/prompts/arguments", get(server::get_prompt_arguments))

        // Instance management
        .route("/{identifier}/instances", get(server::list_instances));

    let instances_router = Router::new()
        .route("/{id}", get(instance::get_instance))
        .route("/{id}/health", get(instance::check_health))
        .route("/{id}/disconnect", post(instance::disconnect))
        .route("/{id}/disconnect/force", post(instance::force_disconnect))
        .route("/{id}/reconnect", post(instance::reconnect))
        .route("/{id}/reconnect/reset", post(instance::reset_reconnect))
        .route("/{id}/recover", post(instance::recover_instance))
        .route("/{id}/cancel", post(instance::cancel));

    let combined_router = servers_router.nest("/{name}/instances", instances_router);

    Router::new().nest("/mcp/servers", combined_router.with_state(state))
}
