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
        // Server-specific operations (ID-only)
        .route("/{id}", get(server::get_server))
        .route("/{id}", put(server::update_server))
        .route("/{id}", delete(server::delete_server))
        .route("/{id}/enable", post(server::enable_server))
        .route("/{id}/disable", post(server::disable_server))
        // Inspect endpoints per refactor spec
        .route("/{id}/tools", get(server::list_tools))
        .route("/{id}/resources", get(server::list_resources))
        .route("/{id}/resources/templates", get(server::list_resource_templates))
        .route("/{id}/prompts", get(server::list_prompts))
        .route("/{id}/prompts/arguments", get(server::get_prompt_arguments))
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

    let combined_router = servers_router.nest("/{id}/instances", instances_router);

    Router::new().nest("/mcp/servers", combined_router.with_state(state))
}
