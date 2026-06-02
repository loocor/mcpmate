use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with},
};

use super::AppState;
use crate::{
    aide_wrapper, aide_wrapper_payload, aide_wrapper_query,
    api::{
        handlers::secrets,
        models::secrets::{
            SecretCreateReq, SecretDeleteReq, SecretDeleteResp, SecretDetailsReq, SecretListResp, SecretMetadataResp,
            SecretUpdateReq, SecretUsageListResp, SecretUsageReq,
        },
    },
};

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/secrets/list", get_with(list_secrets_aide, list_secrets_docs))
        .api_route(
            "/secrets/details",
            get_with(get_secret_details_aide, get_secret_details_docs),
        )
        .api_route(
            "/secrets/usages",
            get_with(list_secret_usages_aide, list_secret_usages_docs),
        )
        .api_route("/secrets/create", post_with(create_secret_aide, create_secret_docs))
        .api_route("/secrets/update", post_with(update_secret_aide, update_secret_docs))
        .api_route("/secrets/delete", delete_with(delete_secret_aide, delete_secret_docs))
        .with_state(state)
}

aide_wrapper!(
    secrets::list_secrets,
    SecretListResp,
    "List secure-store secret metadata"
);

aide_wrapper_query!(
    secrets::get_secret_details,
    SecretDetailsReq,
    SecretMetadataResp,
    "Get secure-store secret metadata"
);

aide_wrapper_query!(
    secrets::list_secret_usages,
    SecretUsageReq,
    SecretUsageListResp,
    "List secure-store secret usages"
);

aide_wrapper_payload!(
    secrets::create_secret,
    SecretCreateReq,
    SecretMetadataResp,
    "Create a secure-store secret"
);

aide_wrapper_payload!(
    secrets::update_secret,
    SecretUpdateReq,
    SecretMetadataResp,
    "Update secure-store secret metadata or value"
);

aide_wrapper_payload!(
    secrets::delete_secret,
    SecretDeleteReq,
    SecretDeleteResp,
    "Delete a secure-store secret"
);
