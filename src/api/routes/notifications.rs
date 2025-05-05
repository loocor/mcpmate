// MCP Proxy API routes for notifications
// Contains route definitions for notification endpoints

use axum::{
    routing::post,
    Router,
};
use std::sync::Arc;

use super::AppState;
use crate::api::handlers::notification;

/// Create notification routes
pub fn routes(state: Arc<AppState>) -> Router {
    let notifications_router = Router::new()
        .route("/tools/changed", post(notification::notify_tools_changed))
        .with_state(state);

    Router::new().nest("/api/notifications", notifications_router)
}
