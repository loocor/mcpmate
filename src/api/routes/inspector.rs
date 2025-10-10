use std::sync::Arc;

use crate::api::handlers::inspector;
use crate::api::models::inspector::{
    InspectorListQuery, InspectorPromptGetReq, InspectorPromptGetResp, InspectorPromptsListResp,
    InspectorResourceReadQuery, InspectorResourceReadResp, InspectorResourcesListResp, InspectorSessionCloseReq,
    InspectorSessionCloseResp, InspectorSessionOpenReq, InspectorSessionOpenResp, InspectorTemplatesListResp,
    InspectorToolCallCancelReq, InspectorToolCallCancelResp, InspectorToolCallReq, InspectorToolCallResp,
    InspectorToolCallStartResp, InspectorToolsListResp,
};
use crate::{aide_wrapper_payload, aide_wrapper_query};
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
        .with_state(state)
}
