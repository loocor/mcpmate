// MCP Proxy API routes for notifications
// Contains route definitions for notification endpoints

use std::sync::Arc;

use aide::axum::{ApiRouter, routing::post_with};

use super::AppState;
use crate::{
    aide_wrapper_payload,
    api::{
        handlers::notifs,
        models::notifs::{ToolsChangedReq, ToolsChangedResp},
    },
};

// Generate aide-compatible wrapper for tools_changed function (with JSON payload)
aide_wrapper_payload!(
    notifs::tools_changed,
    ToolsChangedReq,
    ToolsChangedResp,
    "Notify clients that the tools list has changed"
);

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    let notifs_router = ApiRouter::new()
        .api_route("/tools/changed", post_with(tools_changed_aide, tools_changed_docs))
        .with_state(state);

    ApiRouter::new().nest("/notifs", notifs_router)
}
