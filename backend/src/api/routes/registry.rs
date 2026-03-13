use std::sync::Arc;

use aide::axum::ApiRouter;
use axum::routing::get;

use super::AppState;
use crate::api::handlers::registry;

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .route("/mcp/registry/servers", get(registry::list_servers))
        .with_state(state)
}
