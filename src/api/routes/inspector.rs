use std::sync::Arc;

use crate::api::handlers::inspector;
use crate::api::models::inspector::{
    InspectorListQuery, InspectorPromptGetReq, InspectorPromptGetResp, InspectorPromptsListResp,
    InspectorResourceReadQuery, InspectorResourceReadResp, InspectorResourcesListResp, InspectorToolCallReq,
    InspectorToolCallResp, InspectorToolsListResp,
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
aide_wrapper_payload!(
    inspector::tool_call,
    InspectorToolCallReq,
    InspectorToolCallResp,
    "Inspector: call tool"
);
// manual wrapper for SSE (aide_wrapper macros target JSON handlers)
use aide::transform::TransformOperation;
pub async fn tool_call_stream_aide(
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    inspector::tool_call_stream(axum::extract::State(state), axum::extract::Query(q))
        .await
        .into_response()
}
pub fn tool_call_stream_docs(op: TransformOperation) -> TransformOperation {
    op.description("Inspector: tool call stream").tag("inspector")
}
aide_wrapper_payload!(
    inspector::tool_call_cancel,
    serde_json::Value,
    serde_json::Value,
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
aide_wrapper_query!(inspector::calls_recent, std::collections::HashMap<String, String>, serde_json::Value, "Inspector: recent calls");
aide_wrapper_query!(inspector::calls_details, std::collections::HashMap<String, String>, serde_json::Value, "Inspector: call details");
aide_wrapper!(inspector::calls_clear, serde_json::Value, "Inspector: clear calls");

use crate::api::routes::AppState;

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        // tools
        .api_route("/mcp/inspector/tool/list", get_with(tools_list_aide, tools_list_docs))
        .api_route("/mcp/inspector/tool/call", post_with(tool_call_aide, tool_call_docs))
        .api_route(
            "/mcp/inspector/tool/call/stream",
            get_with(tool_call_stream_aide, tool_call_stream_docs),
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
        // prompts
        .api_route(
            "/mcp/inspector/prompt/list",
            get_with(prompts_list_aide, prompts_list_docs),
        )
        .api_route("/mcp/inspector/prompt/get", post_with(prompt_get_aide, prompt_get_docs))
        // recent calls
        .api_route(
            "/mcp/inspector/calls/recent",
            get_with(calls_recent_aide, calls_recent_docs),
        )
        .api_route(
            "/mcp/inspector/calls/details",
            get_with(calls_details_aide, calls_details_docs),
        )
        .api_route(
            "/mcp/inspector/calls/clear",
            post_with(calls_clear_aide, calls_clear_docs),
        )
        .with_state(state)
}
