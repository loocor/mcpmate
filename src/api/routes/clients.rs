use crate::api::handlers::clients;
use crate::api::models::clients::{
    ClientConfigReq, ClientConfigResp, ClientConfigUpdateReq, ClientConfigUpdateResp, ClientsCheckReq, ClientsCheckResp,
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
    clients::list,
    ClientsCheckReq,
    ClientsCheckResp,
    "Get all clients with optional force refresh."
);

// Generate aide-compatible wrapper for client details (with query parameters)
aide_wrapper_query!(
    clients::details,
    ClientConfigReq,
    ClientConfigResp,
    "Get client configuration details with optional server import"
);

// Generate aide-compatible wrapper for client update (with payload body)
aide_wrapper_payload!(
    clients::update,
    ClientConfigUpdateReq,
    ClientConfigUpdateResp,
    "Update client configuration with specified settings"
);

/// Create client management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/clients/list", get_with(list_aide, list_docs))
        .api_route("/clients/details", get_with(details_aide, details_docs))
        .api_route("/clients/update", post_with(update_aide, update_docs))
        .with_state(state)
}
