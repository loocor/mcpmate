// Runtime API routes - Simple RESTful wrapper around CLI functionality

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use crate::api::{
    handlers::runtime::{install_runtime, runtime_cache, runtime_cache_reset, runtime_status},
    routes::AppState,
};

/// Create runtime management routes
///
/// API structure:
/// - POST /runtime/install              # Install runtime
/// - GET  /runtime/status               # Unified status (combines original list + basic cache)
/// - GET  /runtime/cache                # Detailed cache information (optional)
/// - POST /runtime/cache/reset          # Cache management (future extensions lock/rollback/check)
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/runtime/install", post(install_runtime))
        .route("/runtime/status", get(runtime_status))
        .route("/runtime/cache", get(runtime_cache))
        .route("/runtime/cache/reset", post(runtime_cache_reset))
        .with_state(state)
}
