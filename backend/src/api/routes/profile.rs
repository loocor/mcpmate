// MCP Proxy API routes for Profile management
// Contains route definitions for Profile endpoints

use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with},
};

use super::AppState;
use crate::api::handlers::profile;
use crate::api::models::profile::{
    ProfileComponentListReq, ProfileComponentManageReq, ProfileCreateReq, ProfileDeleteReq, ProfileDetailsReq,
    ProfileDetailsResp, ProfileListReq, ProfileListResp, ProfileManageReq, ProfileManageResp, ProfilePromptsListResp,
    ProfileResourceTemplatesListResp, ProfileResourcesListResp, ProfileResp, ProfileServerManageResp,
    ProfileServersListResp, ProfileToolsListResp, ProfileUpdateReq,
};
use crate::api::models::token_estimate::{CapabilityTokenLedgerResponse, TokenEstimateQuery, TokenEstimateResponse};
use crate::{aide_wrapper_payload, aide_wrapper_query};

/// Create Profile management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/mcp/profile/list", get_with(profile_list_aide, profile_list_docs))
        .api_route(
            "/mcp/profile/create",
            post_with(profile_create_aide, profile_create_docs),
        )
        .api_route(
            "/mcp/profile/details",
            get_with(profile_details_aide, profile_details_docs),
        )
        .api_route(
            "/mcp/profile/update",
            post_with(profile_update_aide, profile_update_docs),
        )
        .api_route(
            "/mcp/profile/delete",
            delete_with(profile_delete_aide, profile_delete_docs),
        )
        .api_route(
            "/mcp/profile/manage",
            post_with(profile_manage_aide, profile_manage_docs),
        )
        .api_route(
            "/mcp/profile/servers/list",
            get_with(servers_list_aide, servers_list_docs),
        )
        .api_route(
            "/mcp/profile/servers/manage",
            post_with(server_manage_aide, server_manage_docs),
        )
        .api_route("/mcp/profile/tools/list", get_with(tools_list_aide, tools_list_docs))
        .api_route(
            "/mcp/profile/tools/manage",
            post_with(component_manage_aide, component_manage_docs),
        )
        .api_route(
            "/mcp/profile/resources/list",
            get_with(resources_list_aide, resources_list_docs),
        )
        .api_route(
            "/mcp/profile/resources/manage",
            post_with(component_manage_aide, component_manage_docs),
        )
        .api_route(
            "/mcp/profile/resource-templates/list",
            get_with(resource_templates_list_aide, resource_templates_list_docs),
        )
        .api_route(
            "/mcp/profile/resource-templates/manage",
            post_with(component_manage_aide, component_manage_docs),
        )
        .api_route(
            "/mcp/profile/prompts/list",
            get_with(prompts_list_aide, prompts_list_docs),
        )
        .api_route(
            "/mcp/profile/prompts/manage",
            post_with(component_manage_aide, component_manage_docs),
        )
        .api_route(
            "/mcp/profile/token-estimate",
            get_with(token_estimate_aide, token_estimate_docs),
        )
        .api_route(
            "/mcp/profile/capability-token-ledger",
            get_with(capability_token_ledger_aide, capability_token_ledger_docs),
        )
        .with_state(state)
}

// Generate aide-compatible wrappers for basic operations
aide_wrapper_query!(
    profile::profile_list,
    ProfileListReq,
    ProfileListResp,
    "List all profile with optional filtering"
);

aide_wrapper_query!(
    profile::profile_details,
    ProfileDetailsReq,
    ProfileDetailsResp,
    "Get details for a specific profile"
);

// Generate aide-compatible wrappers for CRUD operations
aide_wrapper_payload!(
    profile::profile_create,
    ProfileCreateReq,
    ProfileResp,
    "Create a new profile"
);

aide_wrapper_payload!(
    profile::profile_update,
    ProfileUpdateReq,
    ProfileResp,
    "Update an existing profile"
);

aide_wrapper_payload!(
    profile::profile_delete,
    ProfileDeleteReq,
    ProfileManageResp,
    "Delete a profile"
);

// Generate aide-compatible wrappers for management operations
aide_wrapper_payload!(
    profile::profile_manage,
    ProfileManageReq,
    ProfileManageResp,
    "Manage profile operations (activate/deactivate)"
);

// Generate aide-compatible wrappers for component list operations
aide_wrapper_query!(
    profile::servers_list,
    ProfileComponentListReq,
    ProfileServersListResp,
    "List servers in a profile"
);

aide_wrapper_query!(
    profile::tools_list,
    ProfileComponentListReq,
    ProfileToolsListResp,
    "List tools in a profile"
);

aide_wrapper_query!(
    profile::resources_list,
    ProfileComponentListReq,
    ProfileResourcesListResp,
    "List resources in a profile"
);

aide_wrapper_query!(
    profile::resource_templates_list,
    ProfileComponentListReq,
    ProfileResourceTemplatesListResp,
    "List resource templates in a profile"
);

aide_wrapper_query!(
    profile::prompts_list,
    ProfileComponentListReq,
    ProfilePromptsListResp,
    "List prompts in a profile"
);

// Generate aide-compatible wrappers for server management
aide_wrapper_payload!(
    profile::server_manage,
    ProfileComponentManageReq,
    ProfileServerManageResp,
    "Manage server operations (enable/disable servers in profile)"
);

// Generate aide-compatible wrappers for component management
aide_wrapper_payload!(
    profile::component_manage,
    ProfileComponentManageReq,
    ProfileServerManageResp,
    "Manage component operations (enable/disable tools, resources, prompts)"
);

// Generate aide-compatible wrapper for token estimation
aide_wrapper_query!(
    profile::token_estimate,
    TokenEstimateQuery,
    TokenEstimateResponse,
    "Estimate token savings for a profile"
);

aide_wrapper_query!(
    profile::capability_token_ledger,
    TokenEstimateQuery,
    CapabilityTokenLedgerResponse,
    "Per-capability JSON payloads for client-side tokenizer (profile trimming)"
);
