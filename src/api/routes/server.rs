// MCP Proxy API routes for MCP server management
// Contains route definitions for MCP server endpoints

use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with},
};

use super::AppState;
use crate::api::handlers::{instance, server};
use crate::api::models::server::{
    CreateServerReq, DeleteServerReq, ImportServersApiResp, ImportServersReq, InstanceDetailsApiResp,
    InstanceDetailsReq, InstanceHealthApiResp, InstanceHealthReq, InstanceListApiResp, InstanceListReq,
    InstanceManageReq, OperationApiResp, ServerCapabilityReq, ServerDetailsApiResp, ServerDetailsReq,
    ServerListApiResp, ServerListReq, ServerManageReq, ServerPromptArgumentsApiResp, ServerPromptsApiResp,
    ServerResourceTemplatesApiResp, ServerResourcesApiResp, ServerToolsApiResp, UpdateServerReq,
};
use crate::{aide_wrapper_payload, aide_wrapper_query};

/// Create MCP server management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        // Basic server management - Query parameters
        .api_route("/mcp/servers/list", get_with(server_list_aide, server_list_docs))
        .api_route(
            "/mcp/servers/details",
            get_with(server_details_aide, server_details_docs),
        )
        // CRUD operations - Payload parameters
        .api_route("/mcp/servers/create", post_with(create_server_aide, create_server_docs))
        .api_route("/mcp/servers/update", post_with(update_server_aide, update_server_docs))
        .api_route(
            "/mcp/servers/delete",
            delete_with(delete_server_aide, delete_server_docs),
        )
        .api_route(
            "/mcp/servers/import",
            post_with(import_servers_aide, import_servers_docs),
        )
        // Management operations - Payload with action enum
        .api_route("/mcp/servers/manage", post_with(manage_server_aide, manage_server_docs))
        // Capability endpoints - Query parameters with refresh enum
        .api_route("/mcp/servers/tools", get_with(server_tools_aide, server_tools_docs))
        .api_route(
            "/mcp/servers/resources",
            get_with(server_resources_aide, server_resources_docs),
        )
        .api_route(
            "/mcp/servers/resources/templates",
            get_with(server_resource_templates_aide, server_resource_templates_docs),
        )
        .api_route(
            "/mcp/servers/prompts",
            get_with(server_prompts_aide, server_prompts_docs),
        )
        .api_route(
            "/mcp/servers/prompts/arguments",
            get_with(server_prompt_arguments_aide, server_prompt_arguments_docs),
        )
        // Instance management - Query parameters
        .api_route(
            "/mcp/servers/instances/list",
            get_with(instance_list_aide, instance_list_docs),
        )
        .api_route(
            "/mcp/servers/instances/details",
            get_with(get_instance_aide, get_instance_docs),
        )
        .api_route(
            "/mcp/servers/instances/health",
            get_with(check_health_aide, check_health_docs),
        )
        // Instance management operations - Payload with action enum
        .api_route(
            "/mcp/servers/instances/manage",
            post_with(manage_instance_aide, manage_instance_docs),
        )
        .with_state(state)
}

// Generate aide-compatible wrappers for basic server handlers
aide_wrapper_query!(
    server::server_list,
    ServerListReq,
    ServerListApiResp,
    "List all MCP servers with optional filtering"
);

aide_wrapper_query!(
    server::server_details,
    ServerDetailsReq,
    ServerDetailsApiResp,
    "Get details for a specific MCP server"
);

aide_wrapper_query!(
    server::instance_list,
    InstanceListReq,
    InstanceListApiResp,
    "List instances for servers"
);

// Generate aide-compatible wrappers for CRUD server handlers
aide_wrapper_payload!(
    server::create_server,
    CreateServerReq,
    ServerDetailsApiResp,
    "Create a new MCP server"
);

aide_wrapper_payload!(
    server::import_servers,
    ImportServersReq,
    ImportServersApiResp,
    "Import servers from JSON configuration"
);

// Generate aide-compatible wrappers for capability handlers
aide_wrapper_query!(
    server::server_tools,
    ServerCapabilityReq,
    ServerToolsApiResp,
    "List all tools for a specific server"
);

aide_wrapper_query!(
    server::server_resources,
    ServerCapabilityReq,
    ServerResourcesApiResp,
    "List all resources for a specific server"
);

aide_wrapper_query!(
    server::server_resource_templates,
    ServerCapabilityReq,
    ServerResourceTemplatesApiResp,
    "List all resource templates for a specific server"
);

aide_wrapper_query!(
    server::server_prompts,
    ServerCapabilityReq,
    ServerPromptsApiResp,
    "List all prompts for a specific server"
);

aide_wrapper_query!(
    server::server_prompt_arguments,
    ServerCapabilityReq,
    ServerPromptArgumentsApiResp,
    "List all prompt arguments for a specific server"
);

// Generate aide-compatible wrappers for management handlers
aide_wrapper_payload!(
    server::manage_server,
    ServerManageReq,
    OperationApiResp,
    "Manage server (enable/disable with sync options)"
);

// Generate aide-compatible wrapper for instance management
aide_wrapper_payload!(
    instance::manage_instance,
    InstanceManageReq,
    OperationApiResp,
    "Manage instance operations (disconnect, reconnect, recover, cancel)"
);

// Generate aide-compatible wrappers for updated CRUD handlers
aide_wrapper_payload!(
    server::update_server,
    UpdateServerReq,
    ServerDetailsApiResp,
    "Update server configuration"
);

aide_wrapper_payload!(
    server::delete_server,
    DeleteServerReq,
    OperationApiResp,
    "Delete a server"
);

// Generate aide-compatible wrappers for instance handlers with query parameters
aide_wrapper_query!(
    instance::get_instance,
    InstanceDetailsReq,
    InstanceDetailsApiResp,
    "Get details for a specific server instance"
);

aide_wrapper_query!(
    instance::check_health,
    InstanceHealthReq,
    InstanceHealthApiResp,
    "Check health status of a specific server instance"
);
