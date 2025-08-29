use crate::api::handlers::client;
use crate::api::models::client::{
    ClientConfigReq, ClientConfigResp, ClientConfigUpdateReq, ClientConfigUpdateResp, ClientCheckReq, ClientCheckResp,
};
use crate::api::routes::AppState;
use crate::{aide_wrapper_payload, aide_wrapper_query};
use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use std::sync::Arc;

// Generate aide-compatible wrapper for client check (with query parameters)
aide_wrapper_query!(
    client::list,
    ClientCheckReq,
    ClientCheckResp,
    "Get all client with optional force refresh."
);

// Generate aide-compatible wrapper for client details (with query parameters)
aide_wrapper_query!(
    client::details,
    ClientConfigReq,
    ClientConfigResp,
    "Get client configuration details with optional server import"
);

// Generate aide-compatible wrapper for client update (with payload body)
aide_wrapper_payload!(
    client::update,
    ClientConfigUpdateReq,
    ClientConfigUpdateResp,
    "Update client configuration with specified settings"
);

/// Create client management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/client/list", get_with(list_aide, list_docs))
        .api_route("/client/details", get_with(details_aide, details_docs))
        .api_route("/client/update", post_with(update_aide, update_docs))
        .with_state(state)
}
