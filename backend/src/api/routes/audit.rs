use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use crate::{
    aide_wrapper, aide_wrapper_payload, aide_wrapper_query,
    api::{
        handlers::audit,
        models::audit::{AuditListReq, AuditListResp, AuditPolicyResp, AuditPolicySetReq},
        routes::AppState,
    },
};

aide_wrapper_query!(audit::list_events, AuditListReq, AuditListResp, "List audit events");
aide_wrapper!(audit::get_policy, AuditPolicyResp, "Get audit retention policy");
aide_wrapper_payload!(
    audit::set_policy,
    AuditPolicySetReq,
    AuditPolicyResp,
    "Set audit retention policy"
);

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/audit/events", get_with(list_events_aide, list_events_docs))
        .api_route("/audit/policy", get_with(get_policy_aide, get_policy_docs))
        .api_route("/audit/policy", post_with(set_policy_aide, set_policy_docs))
        .with_state(state)
}
