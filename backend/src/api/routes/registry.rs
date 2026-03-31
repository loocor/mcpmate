use std::sync::Arc;

use aide::axum::ApiRouter;
use aide::axum::routing::post_with;
use axum::routing::{get, post};

use super::AppState;
use crate::aide_wrapper_payload;
use crate::api::handlers::registry;
use crate::api::models::server::{ServerDetailsResp, ServerIdReq};

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .route("/mcp/registry/servers", get(registry::list_servers))
        .route("/mcp/registry/servers/cached", get(registry::list_cached_servers))
        .api_route(
            "/mcp/registry/servers/refresh",
            post_with(
                refresh_managed_server_metadata_aide,
                refresh_managed_server_metadata_docs,
            ),
        )
        .route("/mcp/registry/sync", post(registry::sync_registry))
        .route("/mcp/registry/install", post(registry::install_server))
        .with_state(state)
}

aide_wrapper_payload!(
    registry::refresh_managed_server_metadata,
    ServerIdReq,
    ServerDetailsResp,
    "Refresh a managed server's metadata from official registry"
);
