// Cache routes namespace - top-level cache endpoints
use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use super::AppState;
use crate::api::handlers::cache::capabilities;
use crate::api::models::cache::{CacheDetailsReq, CacheDetailsResp, CacheResetResp};
use crate::{aide_wrapper, aide_wrapper_query};

// Generate aide-compatible wrapper for details function (with query parameters)
aide_wrapper_query!(
    capabilities::details,
    CacheDetailsReq,
    CacheDetailsResp,
    "Get cache details. Query params: view (stats|keys), server_id (optional), limit (optional, max 1000)"
);

// Generate aide-compatible wrapper for reset function (simple scenario)
aide_wrapper!(capabilities::reset, CacheResetResp, "Reset and clear all cache data");

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/cache/capabilities/details", get_with(details_aide, details_docs))
        .api_route("/cache/capabilities/reset", post_with(reset_aide, reset_docs))
        .with_state(state)
}
