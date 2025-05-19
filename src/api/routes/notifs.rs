// MCP Proxy API routes for notifications
// Contains route definitions for notification endpoints

use std::sync::Arc;

use axum::{Router, routing::post};

use super::AppState;
use crate::api::handlers::notifs;

/// Create notification routes
pub fn routes(state: Arc<AppState>) -> Router {
    let notifs_router = Router::new()
        .route("/tools/changed", post(notifs::notify_tools_changed))
        .with_state(state);

    Router::new().nest("/api/notifs", notifs_router)
}
