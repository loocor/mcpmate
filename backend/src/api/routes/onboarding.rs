use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use super::AppState;
use crate::api::handlers::onboarding;
use crate::api::models::onboarding::OnboardingCompleteReq;
use crate::api::models::onboarding::OnboardingServerScanReq;
use crate::api::models::onboarding::OnboardingServerScanResp;
use crate::api::models::onboarding::OnboardingStatusResp;
use crate::api::models::onboarding::RuntimeCheckResp;
use crate::{aide_wrapper, aide_wrapper_payload};

aide_wrapper!(
    onboarding::get_status,
    OnboardingStatusResp,
    "Get current onboarding status"
);

aide_wrapper!(
    onboarding::runtime_check,
    RuntimeCheckResp,
    "Detect available runtimes on the host system"
);

aide_wrapper_payload!(
    onboarding::complete,
    OnboardingCompleteReq,
    onboarding::OnboardingActionResp,
    "Mark onboarding as completed"
);

aide_wrapper_payload!(
    onboarding::server_scan,
    OnboardingServerScanReq,
    OnboardingServerScanResp,
    "Scan selected client configs for existing MCP servers"
);

aide_wrapper!(
    onboarding::reset,
    onboarding::OnboardingActionResp,
    "Reset onboarding status for re-run"
);

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/onboarding/status", get_with(get_status_aide, get_status_docs))
        .api_route("/onboarding/complete", post_with(complete_aide, complete_docs))
        .api_route("/onboarding/reset", post_with(reset_aide, reset_docs))
        .api_route("/onboarding/server-scan", post_with(server_scan_aide, server_scan_docs))
        .api_route(
            "/onboarding/runtime-check",
            get_with(runtime_check_aide, runtime_check_docs),
        )
        .with_state(state)
}
