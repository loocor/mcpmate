// Runtime API routes - Simple RESTful wrapper around CLI functionality

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use crate::api::{
    handlers::runtime::{
        install_runtime,
        list_runtimes,
        runtime_status,
        runtime_cache,
        runtime_cache_clear,
        runtime_cache_rebuild,
        runtime_versions,
    },
    routes::AppState,
};

/// Create runtime management routes
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/runtime/install", post(install_runtime))
        .route("/runtime/list", get(list_runtimes))
        // Spec-aligned endpoints
        .route("/runtime/status", get(runtime_status))
        .route("/runtime/cache", get(runtime_cache))
        .route("/runtime/cache/clear", post(runtime_cache_clear))
        .route("/runtime/cache/rebuild", post(runtime_cache_rebuild))
        .route("/runtime/versions", get(runtime_versions))
        .with_state(state)
}
