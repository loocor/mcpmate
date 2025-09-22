use crate::api::handlers::client;
use crate::api::models::client::{
    ClientBackupActionResp, ClientBackupListReq, ClientBackupListResp, ClientBackupOperateReq, ClientBackupPolicyReq,
    ClientBackupPolicyResp, ClientBackupPolicySetReq, ClientCheckReq, ClientCheckResp, ClientConfigReq,
    ClientConfigResp, ClientConfigRestoreReq, ClientConfigUpdateReq, ClientConfigUpdateResp, ClientManageReq,
    ClientManageResp,
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

// Configuration endpoints
aide_wrapper_query!(
    client::config_details,
    ClientConfigReq,
    ClientConfigResp,
    "Get client configuration details with optional server import"
);

aide_wrapper_payload!(
    client::config_apply,
    ClientConfigUpdateReq,
    ClientConfigUpdateResp,
    "Apply client configuration with specified settings"
);

aide_wrapper_payload!(
    client::config_restore,
    ClientConfigRestoreReq,
    ClientBackupActionResp,
    "Restore a client configuration from backup"
);

// Management toggle
aide_wrapper_payload!(
    client::manage,
    ClientManageReq,
    ClientManageResp,
    "Enable or disable MCPMate management for a client"
);

// Backup administration
aide_wrapper_query!(
    client::list_backups,
    ClientBackupListReq,
    ClientBackupListResp,
    "List stored configuration backups"
);

aide_wrapper_payload!(
    client::delete_backup,
    ClientBackupOperateReq,
    ClientBackupActionResp,
    "Delete a configuration backup"
);

aide_wrapper_query!(
    client::get_backup_policy,
    ClientBackupPolicyReq,
    ClientBackupPolicyResp,
    "Get backup retention policy for a client"
);

aide_wrapper_payload!(
    client::set_backup_policy,
    ClientBackupPolicySetReq,
    ClientBackupPolicyResp,
    "Set backup retention policy for a client"
);

/// Create client management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/client/list", get_with(list_aide, list_docs))
        .api_route(
            "/client/config/details",
            get_with(config_details_aide, config_details_docs),
        )
        .api_route("/client/config/apply", post_with(config_apply_aide, config_apply_docs))
        .api_route(
            "/client/config/restore",
            post_with(config_restore_aide, config_restore_docs),
        )
        .api_route("/client/manage", post_with(manage_aide, manage_docs))
        .api_route("/client/backups/list", get_with(list_backups_aide, list_backups_docs))
        .api_route(
            "/client/backups/delete",
            post_with(delete_backup_aide, delete_backup_docs),
        )
        .api_route(
            "/client/backups/policy",
            get_with(get_backup_policy_aide, get_backup_policy_docs)
                .post_with(set_backup_policy_aide, set_backup_policy_docs),
        )
        .with_state(state)
}
