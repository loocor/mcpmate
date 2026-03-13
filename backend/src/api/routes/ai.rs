//! AI API routes

use std::sync::Arc;

use crate::aide_wrapper_payload;
use crate::api::handlers::ai::extractor::{AIExtractReq, AIExtractResp};
use crate::api::{handlers::ai, routes::AppState};
use aide::axum::{ApiRouter, routing::post_with};

// Generate aide-compatible wrapper for AI extraction (with payload body)
aide_wrapper_payload!(
    ai::extract_config,
    AIExtractReq,
    AIExtractResp,
    "Extract MCP server configuration from text using AI"
);

/// Create AI routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/extract", post_with(extract_config_aide, extract_config_docs))
        .with_state(state)
}
