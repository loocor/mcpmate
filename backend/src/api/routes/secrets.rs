use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with},
};

use super::AppState;
use crate::{
    aide_wrapper, aide_wrapper_payload, aide_wrapper_query,
    api::{
        handlers::{secrets, secrets_password},
        models::secrets::{
            PassphraseRotateReq, PasswordChangeReq, PasswordClearReq, PasswordScopeUpdateReq, PasswordSetReq,
            PasswordSetResp, PasswordStatusResp, PasswordVerifyReq, PasswordVerifyResp, ProviderSwitchReq,
            ProviderSwitchResp, SecretCreateReq, SecretDeleteReq, SecretDeleteResp, SecretDetailsReq, SecretListResp,
            SecretMetadataResp, SecretStoreStatusResp, SecretStoreUnlockReq, SecretUpdateReq, SecretUsageListResp,
            SecretUsageReq,
        },
    },
};

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/secrets/status",
            get_with(get_secret_store_status_aide, get_secret_store_status_docs),
        )
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
        .api_route(
            "/secrets/provider/switch",
            post_with(switch_provider_aide, switch_provider_docs),
        )
        .api_route("/secrets/unlock", post_with(unlock_secret_store_aide, unlock_secret_store_docs))
        .api_route(
            "/secrets/passphrase/rotate",
            post_with(rotate_passphrase_aide, rotate_passphrase_docs),
        )
        // Password protection endpoints
        .api_route(
            "/secrets/password/status",
            get_with(get_password_status_aide, get_password_status_docs),
        )
        .api_route(
            "/secrets/password/set",
            post_with(set_password_aide, set_password_docs),
        )
        .api_route(
            "/secrets/password/verify",
            post_with(verify_password_endpoint_aide, verify_password_endpoint_docs),
        )
        .api_route(
            "/secrets/password/change",
            post_with(change_password_aide, change_password_docs),
        )
        .api_route(
            "/secrets/password/clear",
            post_with(clear_password_aide, clear_password_docs),
        )
        .api_route(
            "/secrets/password/scope",
            post_with(update_password_scope_aide, update_password_scope_docs),
        )
        .with_state(state)
}

aide_wrapper!(
    secrets::list_secrets,
    SecretListResp,
    "List secure-store secret metadata"
);

aide_wrapper!(
    secrets::get_secret_store_status,
    SecretStoreStatusResp,
    "Get secure-store readiness status"
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

aide_wrapper_payload!(
    secrets::switch_provider,
    ProviderSwitchReq,
    ProviderSwitchResp,
    "Switch secret store provider mode"
);

aide_wrapper_payload!(
    secrets::unlock_secret_store,
    SecretStoreUnlockReq,
    SecretStoreStatusResp,
    "Unlock passphrase-protected secret store"
);

aide_wrapper_payload!(
    secrets::rotate_passphrase,
    PassphraseRotateReq,
    SecretStoreStatusResp,
    "Rotate passphrase encryption password"
);

// Password protection aide wrappers
aide_wrapper!(
    secrets_password::get_password_status,
    PasswordStatusResp,
    "Get password protection status"
);

aide_wrapper_payload!(
    secrets_password::set_password,
    PasswordSetReq,
    PasswordSetResp,
    "Set master password"
);

aide_wrapper_payload!(
    secrets_password::verify_password_endpoint,
    PasswordVerifyReq,
    PasswordVerifyResp,
    "Verify master password"
);

aide_wrapper_payload!(
    secrets_password::change_password,
    PasswordChangeReq,
    PasswordStatusResp,
    "Change master password"
);

aide_wrapper_payload!(
    secrets_password::clear_password,
    PasswordClearReq,
    PasswordStatusResp,
    "Clear master password"
);

aide_wrapper_payload!(
    secrets_password::update_password_scope,
    PasswordScopeUpdateReq,
    PasswordStatusResp,
    "Update password protection scope"
);
