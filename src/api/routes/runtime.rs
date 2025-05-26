// Runtime API routes - Simple RESTful wrapper around CLI functionality

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use crate::api::{
    handlers::runtime::{install_runtime, list_runtimes},
    routes::AppState,
};

/// Create runtime management routes
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/runtime/install", post(install_runtime))
        .route("/runtime/list", get(list_runtimes))
        .with_state(state)
}
