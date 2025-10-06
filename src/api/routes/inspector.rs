use std::sync::Arc;

use crate::api::handlers::inspector;
use crate::api::models::inspector::{
    InspectorListQuery, InspectorPromptGetReq, InspectorPromptGetResp, InspectorPromptsListResp,
    InspectorResourceReadQuery, InspectorResourceReadResp, InspectorResourcesListResp, InspectorToolCallReq,
    InspectorToolCallResp, InspectorToolsListResp,
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
aide_wrapper_payload!(
    inspector::tool_call,
    InspectorToolCallReq,
    InspectorToolCallResp,
    "Inspector: call tool"
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
use crate::api::routes::AppState;

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        // tools
        .api_route("/mcp/inspector/tool/list", get_with(tools_list_aide, tools_list_docs))
        .api_route("/mcp/inspector/tool/call", post_with(tool_call_aide, tool_call_docs))
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
        .with_state(state)
}
