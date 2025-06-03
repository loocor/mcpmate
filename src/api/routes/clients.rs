use crate::api::handlers::clients;
use crate::api::routes::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

/// Create client management routes
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        // Client detection and info
        .route("/clients", get(clients::get_clients))
        // Configuration details
        .route(
            "/clients/{client_identifier}/config",
            get(clients::get_config),
        )
        // Configuration generate and apply
        .route(
            "/clients/{client_identifier}/config",
            post(clients::manage_config),
        )
        .with_state(state)
}
