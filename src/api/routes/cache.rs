// Cache routes namespace - top-level cache endpoints
use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use super::AppState;
use crate::api::handlers::cache::capabilities as cache_capabilities;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/cache/capabilities/details", get(cache_capabilities::details))
        .route("/cache/capabilities/reset", post(cache_capabilities::reset))
        .with_state(state)
}
