// MCP Proxy API routes for Config Suit management
// Contains route definitions for Config Suit endpoints

use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with},
};

use super::AppState;
use crate::api::handlers::suits;
use crate::api::models::suits::{
    SuitComponentListReq, SuitComponentManageReq, SuitCreateReq, SuitDeleteReq, SuitDetailsReq, SuitDetailsResp,
    SuitManageReq, SuitManageResp, SuitPromptsListResp, SuitResourcesListResp, SuitResp, SuitServerManageResp,
    SuitServersListResp, SuitToolsListResp, SuitUpdateReq, SuitsListReq, SuitsListResp,
};
use crate::{aide_wrapper_payload, aide_wrapper_query};

/// Create Config Suit management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/mcp/suits/list", get_with(suits_list_aide, suits_list_docs))
        .api_route("/mcp/suits/create", post_with(suit_create_aide, suit_create_docs))
        .api_route("/mcp/suits/details", get_with(suit_details_aide, suit_details_docs))
        .api_route("/mcp/suits/update", post_with(suit_update_aide, suit_update_docs))
        .api_route("/mcp/suits/delete", delete_with(suit_delete_aide, suit_delete_docs))
        .api_route("/mcp/suits/manage", post_with(suit_manage_aide, suit_manage_docs))
        .api_route(
            "/mcp/suits/servers/list",
            get_with(servers_list_aide, servers_list_docs),
        )
        .api_route(
            "/mcp/suits/servers/manage",
            post_with(server_manage_aide, server_manage_docs),
        )
        .api_route("/mcp/suits/tools/list", get_with(tools_list_aide, tools_list_docs))
        .api_route(
            "/mcp/suits/tools/manage",
            post_with(component_manage_aide, component_manage_docs),
        )
        .api_route(
            "/mcp/suits/resources/list",
            get_with(resources_list_aide, resources_list_docs),
        )
        .api_route(
            "/mcp/suits/resources/manage",
            post_with(component_manage_aide, component_manage_docs),
        )
        .api_route(
            "/mcp/suits/prompts/list",
            get_with(prompts_list_aide, prompts_list_docs),
        )
        .api_route(
            "/mcp/suits/prompts/manage",
            post_with(component_manage_aide, component_manage_docs),
        )
        .with_state(state)
}

// Generate aide-compatible wrappers for basic operations
aide_wrapper_query!(
    suits::suits_list,
    SuitsListReq,
    SuitsListResp,
    "List all configuration suits with optional filtering"
);

aide_wrapper_query!(
    suits::suit_details,
    SuitDetailsReq,
    SuitDetailsResp,
    "Get details for a specific configuration suit"
);

// Generate aide-compatible wrappers for CRUD operations
aide_wrapper_payload!(
    suits::suit_create,
    SuitCreateReq,
    SuitResp,
    "Create a new configuration suit"
);

aide_wrapper_payload!(
    suits::suit_update,
    SuitUpdateReq,
    SuitResp,
    "Update an existing configuration suit"
);

aide_wrapper_payload!(
    suits::suit_delete,
    SuitDeleteReq,
    SuitManageResp,
    "Delete a configuration suit"
);

// Generate aide-compatible wrappers for management operations
aide_wrapper_payload!(
    suits::suit_manage,
    SuitManageReq,
    SuitManageResp,
    "Manage suit operations (activate/deactivate)"
);

// Generate aide-compatible wrappers for component list operations
aide_wrapper_query!(
    suits::servers_list,
    SuitComponentListReq,
    SuitServersListResp,
    "List servers in a configuration suit"
);

aide_wrapper_query!(
    suits::tools_list,
    SuitComponentListReq,
    SuitToolsListResp,
    "List tools in a configuration suit"
);

aide_wrapper_query!(
    suits::resources_list,
    SuitComponentListReq,
    SuitResourcesListResp,
    "List resources in a configuration suit"
);

aide_wrapper_query!(
    suits::prompts_list,
    SuitComponentListReq,
    SuitPromptsListResp,
    "List prompts in a configuration suit"
);

// Generate aide-compatible wrappers for server management
aide_wrapper_payload!(
    suits::server_manage,
    SuitComponentManageReq,
    SuitServerManageResp,
    "Manage server operations (enable/disable servers in configuration suits)"
);

// Generate aide-compatible wrappers for component management
aide_wrapper_payload!(
    suits::component_manage,
    SuitComponentManageReq,
    SuitServerManageResp,
    "Manage component operations (enable/disable tools, resources, prompts)"
);
