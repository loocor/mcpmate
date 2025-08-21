use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use super::AppState;

use crate::api::models::runtime::{
    RuntimeCacheResetReq, RuntimeCacheResetResp, RuntimeCacheResp, RuntimeInstallReq, RuntimeInstallResp,
    RuntimeStatusResp,
};
use crate::{aide_wrapper, aide_wrapper_payload};

// Import the handlers for aide wrapper
use crate::api::handlers::runtime;

// Generate aide-compatible wrapper for runtime install (with JSON body)
aide_wrapper_payload!(
    runtime::install,
    RuntimeInstallReq,
    RuntimeInstallResp,
    "Install runtime package (UV or Bun) with optional configuration"
);

// Generate aide-compatible wrapper for runtime status (no parameters)
aide_wrapper!(runtime::status, RuntimeStatusResp, "Get runtime status for UV and Bun");

// Generate aide-compatible wrapper for runtime cache (no parameters)
aide_wrapper!(
    runtime::cache,
    RuntimeCacheResp,
    "Get runtime cache information and statistics"
);

// Generate aide-compatible wrapper for cache reset (with JSON payload)
aide_wrapper_payload!(
    runtime::reset_cache,
    RuntimeCacheResetReq,
    RuntimeCacheResetResp,
    "Reset runtime cache. Payload: {cache_type: 'all'|'uv'|'bun'} (default: 'all')"
);

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/runtime/install", post_with(install_aide, install_docs))
        .api_route("/runtime/status", get_with(status_aide, status_docs))
        .api_route("/runtime/cache", get_with(cache_aide, cache_docs))
        .api_route("/runtime/cache/reset", post_with(reset_cache_aide, reset_cache_docs))
        .with_state(state)
}
