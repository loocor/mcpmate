// MCP Proxy API routes for Config Suit management
// Contains route definitions for Config Suit endpoints

use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use super::AppState;
use crate::api::handlers::suits;

/// Create Config Suit management routes
pub fn routes(state: Arc<AppState>) -> Router {
    // Basic Config Suit operations
    let suits_router = Router::new()
        .route("/", get(suits::list_suits))
        .route("/", post(suits::create_suit))
        .route("/{id}", get(suits::get_suit))
        .route("/{id}", put(suits::update_suit))
        .route("/{id}", delete(suits::delete_suit))
        .route("/{id}/activate", post(suits::activate_suit))
        .route("/{id}/deactivate", post(suits::deactivate_suit))
        .route("/batch/activate", post(suits::batch_activate_suits))
        .route("/batch/deactivate", post(suits::batch_deactivate_suits));

    // Server management in Config Suits
    let servers_router = Router::new()
        .route("/", get(suits::list_servers))
        .route("/{server_id}/enable", post(suits::enable_server))
        .route("/{server_id}/disable", post(suits::disable_server))
        .route("/batch/enable", post(suits::batch_enable_servers))
        .route("/batch/disable", post(suits::batch_disable_servers));

    // Tool management in Config Suits
    let tools_router = Router::new()
        .route("/", get(suits::list_tools))
        .route("/{tool_id}/enable", post(suits::enable_tool))
        .route("/{tool_id}/disable", post(suits::disable_tool))
        .route("/batch/enable", post(suits::batch_enable_tools))
        .route("/batch/disable", post(suits::batch_disable_tools));

    // Combine all routers
    let combined_router = suits_router
        .nest("/{id}/servers", servers_router)
        .nest("/{id}/tools", tools_router);

    Router::new()
        .nest("/api/mcp/suits", combined_router)
        .with_state(state)
}
