use crate::api::handlers::client;
use crate::api::models::client::{
    ApprovalRequest, ApprovalResponse, ClientAttachReq, ClientAttachResp, ClientBackupActionResp, ClientBackupListReq,
    ClientBackupListResp, ClientBackupOperateReq, ClientBackupPolicyReq, ClientBackupPolicyResp,
    ClientBackupPolicySetReq, ClientCapabilityConfigReq, ClientCapabilityConfigResp, ClientCheckReq, ClientCheckResp,
    ClientConfigFileParseInspectExistingReq, ClientConfigFileParseInspectExistingResp, ClientConfigFileParseInspectReq,
    ClientConfigFileParseInspectResp, ClientConfigReq, ClientConfigResp, ClientConfigRestoreReq, ClientConfigUpdateReq,
    ClientConfigUpdateResp, ClientDeleteReq, ClientDeleteResp, ClientDetachReq, ClientDetachResp, ClientDetectReq,
    ClientSettingsUpdateReq, ClientSettingsUpdateResp, SurfacePublicationListQuery, SurfacePublicationListResp,
    SurfaceReviewItemResp, SurfaceReviewListQuery, SurfaceReviewListResp, SurfaceReviewPath, SurfaceReviewSummaryResp,
};
use crate::api::routes::AppState;
use crate::{aide_wrapper, aide_wrapper_path, aide_wrapper_path_payload, aide_wrapper_payload, aide_wrapper_query};
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

aide_wrapper_query!(
    client::detect,
    ClientDetectReq,
    ClientCheckResp,
    "Detect installed clients without persisting discovered candidates."
);

// Configuration endpoints
aide_wrapper_query!(
    client::config_details,
    ClientConfigReq,
    ClientConfigResp,
    "Get client configuration details"
);

aide_wrapper_payload!(
    client::config_file_parse_inspect,
    ClientConfigFileParseInspectReq,
    ClientConfigFileParseInspectResp,
    "Inspect a client config file against parse rules"
);

aide_wrapper_payload!(
    client::config_file_parse_inspect_existing,
    ClientConfigFileParseInspectExistingReq,
    ClientConfigFileParseInspectExistingResp,
    "Inspect a stored client config file against parse rules"
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

aide_wrapper_payload!(
    client::delete_client,
    ClientDeleteReq,
    ClientDeleteResp,
    "Delete a client record"
);

// Update client settings (config_mode/transport/client_version)
aide_wrapper_payload!(
    client::update_settings,
    ClientSettingsUpdateReq,
    ClientSettingsUpdateResp,
    "Update client settings (config_mode/transport/client_version)"
);

aide_wrapper_payload!(
    client::update_capability_config,
    ClientCapabilityConfigReq,
    ClientCapabilityConfigResp,
    "Update client capability configuration"
);

aide_wrapper_query!(
    client::get_capability_config,
    ClientConfigReq,
    ClientCapabilityConfigResp,
    "Get client capability configuration"
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

aide_wrapper_payload!(
    client::approve_client,
    ApprovalRequest,
    ApprovalResponse,
    "Approve a pending client"
);

aide_wrapper_payload!(
    client::suspend_client,
    ApprovalRequest,
    ApprovalResponse,
    "Suspend a client"
);

aide_wrapper_payload!(
    client::client_detach,
    ClientDetachReq,
    ClientDetachResp,
    "Detach MCPMate from a client's configuration"
);

aide_wrapper_payload!(
    client::client_attach,
    ClientAttachReq,
    ClientAttachResp,
    "Re-attach MCPMate to a client's external configuration"
);

aide_wrapper_query!(
    client::list_surface_reviews,
    SurfaceReviewListQuery,
    SurfaceReviewListResp,
    "List Consumer Surface review items."
);

aide_wrapper!(
    client::summarize_surface_reviews,
    SurfaceReviewSummaryResp,
    "Summarize pending Consumer Surface review items."
);

aide_wrapper_query!(
    client::list_surface_publications,
    SurfacePublicationListQuery,
    SurfacePublicationListResp,
    "List Consumer Surface publication history."
);

aide_wrapper_path!(
    client::get_surface_review,
    SurfaceReviewPath,
    SurfaceReviewItemResp,
    "Get a Consumer Surface review item and its field diff."
);

aide_wrapper_path_payload!(
    client::approve_surface_review,
    SurfaceReviewPath,
    crate::api::models::client::SurfaceReviewActionReq,
    crate::api::models::client::SurfaceReviewActionResp,
    "Approve the current target of a Consumer Surface review item."
);

aide_wrapper_path_payload!(
    client::rollback_surface_publication,
    crate::api::models::client::SurfacePublicationPath,
    crate::api::models::client::SurfaceRollbackReq,
    crate::api::models::client::SurfaceRollbackResp,
    "Rollback a Consumer Surface to an executable historical publication."
);

aide_wrapper_path_payload!(
    client::preview_surface_intent_resolution,
    SurfaceReviewPath,
    crate::api::models::client::SurfaceIntentPreviewReq,
    crate::api::models::client::SurfaceIntentPreviewResp,
    "Preview the Owner-scoped impact of a Surface intent action."
);

aide_wrapper_path_payload!(
    client::resolve_surface_intent,
    SurfaceReviewPath,
    crate::api::models::client::SurfaceIntentResolveReq,
    crate::api::models::client::SurfaceReviewActionResp,
    "Resolve a Missing or manual-rebind Surface review item."
);

aide_wrapper_path_payload!(
    client::reject_surface_review,
    SurfaceReviewPath,
    crate::api::models::client::SurfaceReviewActionReq,
    crate::api::models::client::SurfaceReviewActionResp,
    "Reject the current target of a Consumer Surface review item."
);

/// Create client management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/client/list", get_with(list_aide, list_docs))
        .api_route("/client/detect", get_with(detect_aide, detect_docs))
        .api_route(
            "/client/config/details",
            get_with(config_details_aide, config_details_docs),
        )
        .api_route(
            "/client/config-file-parse/inspect",
            post_with(config_file_parse_inspect_aide, config_file_parse_inspect_docs),
        )
        .api_route(
            "/client/config-file-parse/inspect-existing",
            post_with(
                config_file_parse_inspect_existing_aide,
                config_file_parse_inspect_existing_docs,
            ),
        )
        .api_route("/client/config/apply", post_with(config_apply_aide, config_apply_docs))
        .api_route(
            "/client/config/restore",
            post_with(config_restore_aide, config_restore_docs),
        )
        .api_route("/client/delete", post_with(delete_client_aide, delete_client_docs))
        .api_route("/client/update", post_with(update_settings_aide, update_settings_docs))
        .api_route(
            "/client/capability-config",
            get_with(get_capability_config_aide, get_capability_config_docs)
                .post_with(update_capability_config_aide, update_capability_config_docs),
        )
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
        .api_route(
            "/client/manage/approve",
            post_with(approve_client_aide, approve_client_docs),
        )
        .api_route(
            "/client/manage/suspend",
            post_with(suspend_client_aide, suspend_client_docs),
        )
        .api_route("/client/detach", post_with(client_detach_aide, client_detach_docs))
        .api_route("/client/attach", post_with(client_attach_aide, client_attach_docs))
        .api_route(
            "/client/surface/reviews/summary",
            get_with(summarize_surface_reviews_aide, summarize_surface_reviews_docs),
        )
        .api_route(
            "/client/surface/reviews",
            get_with(list_surface_reviews_aide, list_surface_reviews_docs),
        )
        .api_route(
            "/client/surface/reviews/{review_item_id}",
            get_with(get_surface_review_aide, get_surface_review_docs),
        )
        .api_route(
            "/client/surface/reviews/{review_item_id}/approve",
            post_with(approve_surface_review_aide, approve_surface_review_docs),
        )
        .api_route(
            "/client/surface/reviews/{review_item_id}/reject",
            post_with(reject_surface_review_aide, reject_surface_review_docs),
        )
        .api_route(
            "/client/surface/reviews/{review_item_id}/resolve-intent/preview",
            post_with(
                preview_surface_intent_resolution_aide,
                preview_surface_intent_resolution_docs,
            ),
        )
        .api_route(
            "/client/surface/reviews/{review_item_id}/resolve-intent",
            post_with(resolve_surface_intent_aide, resolve_surface_intent_docs),
        )
        .api_route(
            "/client/surface/publications",
            get_with(list_surface_publications_aide, list_surface_publications_docs),
        )
        .api_route(
            "/client/surface/publications/{publication_id}/rollback",
            post_with(rollback_surface_publication_aide, rollback_surface_publication_docs),
        )
        .with_state(state)
}
