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
    SuitManageReq, SuitManageResp, SuitResp, SuitServerManageResp, SuitServersListResp, SuitToolsListResp,
    SuitUpdateReq, SuitsListReq, SuitsListResp,
};
use crate::{aide_wrapper_payload, aide_wrapper_query};

/// Create Config Suit management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/mcp/suits/list", get_with(suits_list_aide, suits_list_docs))
        .api_route("/mcp/suits/details", get_with(suit_details_aide, suit_details_docs))
        .api_route(
            "/mcp/suits/create",
            post_with(create_suit_standardized_aide, create_suit_standardized_docs),
        )
        .api_route(
            "/mcp/suits/update",
            post_with(update_suit_standardized_aide, update_suit_standardized_docs),
        )
        .api_route(
            "/mcp/suits/delete",
            delete_with(delete_suit_standardized_aide, delete_suit_standardized_docs),
        )
        .api_route("/mcp/suits/manage", post_with(manage_suit_aide, manage_suit_docs))
        .api_route(
            "/mcp/suits/servers/list",
            get_with(servers_list_aide, servers_list_docs),
        )
        .api_route(
            "/mcp/suits/servers/manage",
            post_with(manage_component_aide, manage_component_docs),
        )
        .api_route("/mcp/suits/tools/list", get_with(tools_list_aide, tools_list_docs))
        .api_route(
            "/mcp/suits/tools/manage",
            post_with(manage_component_aide, manage_component_docs),
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
    suits::create_suit_standardized,
    SuitCreateReq,
    SuitResp,
    "Create a new configuration suit"
);

aide_wrapper_payload!(
    suits::update_suit_standardized,
    SuitUpdateReq,
    SuitResp,
    "Update an existing configuration suit"
);

aide_wrapper_payload!(
    suits::delete_suit_standardized,
    SuitDeleteReq,
    SuitManageResp,
    "Delete a configuration suit"
);

// Generate aide-compatible wrappers for management operations
aide_wrapper_payload!(
    suits::manage_suit,
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

// Generate aide-compatible wrappers for component management
aide_wrapper_payload!(
    suits::manage_component,
    SuitComponentManageReq,
    SuitServerManageResp,
    "Manage component operations (enable/disable servers, tools, etc.)"
);
