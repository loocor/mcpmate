use std::sync::Arc;

use crate::api::handlers::inspector;
use crate::api::models::inspector::{
    InspectorCapabilityPatchResp, InspectorCapabilityPatchUpsertReq, InspectorCompatibilitySnapshotResp,
    InspectorListQuery, InspectorLlmEvaluationReq, InspectorLlmEvaluationResp, InspectorPackageSafetySnapshotResp,
    InspectorPromptGetReq, InspectorPromptGetResp, InspectorPromptsListResp, InspectorResourceReadQuery,
    InspectorResourceReadResp, InspectorResourcesListResp, InspectorScratchServerCreateReq,
    InspectorScratchServerCreateResp, InspectorScratchServerDeleteReq, InspectorScratchServerDeleteResp,
    InspectorScratchServerListResp, InspectorSessionCloseReq, InspectorSessionCloseResp, InspectorSessionOpenReq,
    InspectorSessionOpenResp, InspectorSessionRefreshReq, InspectorSessionRefreshResp, InspectorTemplatesListResp,
    InspectorToolCallCancelReq, InspectorToolCallCancelResp, InspectorToolCallEvidenceQuery,
    InspectorToolCallEvidenceResp, InspectorToolCallReq, InspectorToolCallResp, InspectorToolCallStartResp,
    InspectorToolsListResp,
};
use crate::{aide_wrapper, aide_wrapper_payload, aide_wrapper_query};
use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

// Aide wrappers to satisfy OperationHandler trait and generate docs
aide_wrapper_query!(
    inspector::tools_list,
    InspectorListQuery,
    InspectorToolsListResp,
    "Inspector: list tools"
);
aide_wrapper_query!(
    inspector::prompts_list,
    InspectorListQuery,
    InspectorPromptsListResp,
    "Inspector: list prompts"
);
aide_wrapper_query!(
    inspector::resources_list,
    InspectorListQuery,
    InspectorResourcesListResp,
    "Inspector: list resources"
);
aide_wrapper_query!(
    inspector::templates_list,
    InspectorListQuery,
    InspectorTemplatesListResp,
    "Inspector: list templates"
);
aide_wrapper_payload!(
    inspector::tool_call,
    InspectorToolCallReq,
    InspectorToolCallResp,
    "Inspector: call tool"
);
aide_wrapper_payload!(
    inspector::tool_call_start,
    InspectorToolCallReq,
    InspectorToolCallStartResp,
    "Inspector: start tool call"
);
aide_wrapper_payload!(
    inspector::tool_call_cancel,
    InspectorToolCallCancelReq,
    InspectorToolCallCancelResp,
    "Inspector: cancel tool call"
);
aide_wrapper_payload!(
    inspector::capability_patch_upsert,
    InspectorCapabilityPatchUpsertReq,
    InspectorCapabilityPatchResp,
    "Inspector: upsert capability patch"
);
aide_wrapper_payload!(
    inspector::llm_evaluate,
    InspectorLlmEvaluationReq,
    InspectorLlmEvaluationResp,
    "Inspector: run LLM evaluation"
);
aide_wrapper!(
    inspector::scratch_server_list,
    InspectorScratchServerListResp,
    "Inspector: list scratch server records"
);
aide_wrapper_payload!(
    inspector::scratch_server_create,
    InspectorScratchServerCreateReq,
    InspectorScratchServerCreateResp,
    "Inspector: create scratch server record"
);
aide_wrapper_payload!(
    inspector::scratch_server_delete,
    InspectorScratchServerDeleteReq,
    InspectorScratchServerDeleteResp,
    "Inspector: delete scratch server record"
);
aide_wrapper_query!(
    inspector::tool_call_evidence,
    InspectorToolCallEvidenceQuery,
    InspectorToolCallEvidenceResp,
    "Inspector: get tool call evidence"
);
aide_wrapper_query!(
    inspector::compatibility_snapshot,
    InspectorListQuery,
    InspectorCompatibilitySnapshotResp,
    "Inspector: get compatibility snapshot"
);
aide_wrapper_query!(
    inspector::package_safety_snapshot,
    InspectorListQuery,
    InspectorPackageSafetySnapshotResp,
    "Inspector: get package safety snapshot"
);
aide_wrapper_query!(
    inspector::resource_read,
    InspectorResourceReadQuery,
    InspectorResourceReadResp,
    "Inspector: read resource"
);
aide_wrapper_payload!(
    inspector::prompt_get,
    InspectorPromptGetReq,
    InspectorPromptGetResp,
    "Inspector: get prompt"
);
aide_wrapper_payload!(
    inspector::session_open,
    InspectorSessionOpenReq,
    InspectorSessionOpenResp,
    "Inspector: open session"
);
aide_wrapper_payload!(
    inspector::session_close,
    InspectorSessionCloseReq,
    InspectorSessionCloseResp,
    "Inspector: close session"
);
aide_wrapper_payload!(
    inspector::session_refresh,
    InspectorSessionRefreshReq,
    InspectorSessionRefreshResp,
    "Inspector: refresh session"
);
use crate::api::routes::AppState;

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        // tools
        .api_route("/mcp/inspector/tool/list", get_with(tools_list_aide, tools_list_docs))
        .api_route("/mcp/inspector/tool/call", post_with(tool_call_aide, tool_call_docs))
        .api_route(
            "/mcp/inspector/tool/call/start",
            post_with(tool_call_start_aide, tool_call_start_docs),
        )
        .api_route(
            "/mcp/inspector/tool/call/cancel",
            post_with(tool_call_cancel_aide, tool_call_cancel_docs),
        )
        .api_route(
            "/mcp/inspector/tool/call/evidence",
            get_with(tool_call_evidence_aide, tool_call_evidence_docs),
        )
        .api_route(
            "/mcp/inspector/capability-patch/upsert",
            post_with(capability_patch_upsert_aide, capability_patch_upsert_docs),
        )
        .api_route(
            "/mcp/inspector/llm/evaluate",
            post_with(llm_evaluate_aide, llm_evaluate_docs),
        )
        .api_route(
            "/mcp/inspector/scratch/server/list",
            get_with(scratch_server_list_aide, scratch_server_list_docs),
        )
        .api_route(
            "/mcp/inspector/scratch/server/create",
            post_with(scratch_server_create_aide, scratch_server_create_docs),
        )
        .api_route(
            "/mcp/inspector/scratch/server/delete",
            post_with(scratch_server_delete_aide, scratch_server_delete_docs),
        )
        // compatibility
        .api_route(
            "/mcp/inspector/compatibility/snapshot",
            get_with(compatibility_snapshot_aide, compatibility_snapshot_docs),
        )
        .api_route(
            "/mcp/inspector/package-safety/snapshot",
            get_with(package_safety_snapshot_aide, package_safety_snapshot_docs),
        )
        // resources
        .api_route(
            "/mcp/inspector/resource/list",
            get_with(resources_list_aide, resources_list_docs),
        )
        .api_route(
            "/mcp/inspector/resource/read",
            get_with(resource_read_aide, resource_read_docs),
        )
        // templates
        .api_route(
            "/mcp/inspector/template/list",
            get_with(templates_list_aide, templates_list_docs),
        )
        // prompts
        .api_route(
            "/mcp/inspector/prompt/list",
            get_with(prompts_list_aide, prompts_list_docs),
        )
        .api_route("/mcp/inspector/prompt/get", post_with(prompt_get_aide, prompt_get_docs))
        // sessions
        .api_route(
            "/mcp/inspector/session/open",
            post_with(session_open_aide, session_open_docs),
        )
        .api_route(
            "/mcp/inspector/session/close",
            post_with(session_close_aide, session_close_docs),
        )
        .api_route(
            "/mcp/inspector/session/refresh",
            post_with(session_refresh_aide, session_refresh_docs),
        )
        .with_state(state)
}
