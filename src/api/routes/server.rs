// MCP Proxy API routes for MCP server management
// Contains route definitions for MCP server endpoints

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post, put},
};

use super::AppState;
use crate::api::handlers::{instance, server};

/// Create MCP server management routes
pub fn routes(state: Arc<AppState>) -> Router {
    let servers_router = Router::new()
        .route("/", get(server::list_servers))
        .route("/", post(server::create_server))
        .route("/import", post(server::import_servers))
        .route("/{name}", get(server::get_server))
        .route("/{name}", put(server::update_server))
        .route("/{name}/enable", post(server::enable_server))
        .route("/{name}/disable", post(server::disable_server))
        .route("/{name}/instances", get(server::list_instances));

    let instances_router = Router::new()
        .route("/{id}", get(instance::get_instance))
        .route("/{id}/health", get(instance::check_health))
        .route("/{id}/disconnect", post(instance::disconnect))
        .route("/{id}/disconnect/force", post(instance::force_disconnect))
        .route("/{id}/reconnect", post(instance::reconnect))
        .route("/{id}/reconnect/reset", post(instance::reset_reconnect))
        .route("/{id}/cancel", post(instance::cancel));

    let combined_router = servers_router.nest("/{name}/instances", instances_router);

    Router::new().nest("/api/mcp/servers", combined_router.with_state(state))
}
