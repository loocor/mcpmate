// MCP Proxy API routes for MCP server management
// Contains route definitions for MCP server endpoints

use super::AppState;
use crate::api::handlers::server;
use crate::api::models::{
    cache::{CacheDetailsReq, CacheDetailsResp, CacheResetResp},
    oauth::{
        OAuthInitiateResp, OAuthStatusResp, ServerOAuthConfigReq, ServerOAuthInitiateReq, ServerOAuthPrepareReq,
        ServerOAuthRevokeReq, ServerOAuthStatusReq,
    },
    server::{
        InstanceDetailsReq, InstanceDetailsResp, InstanceHealthReq, InstanceHealthResp, InstanceListReq,
        InstanceListResp, InstanceManageReq, ServerCapabilityReq, ServerCreateReq, ServerDeleteReq, ServerDetailsReq,
        ServerDetailsResp, ServerListReq, ServerListResp, ServerManageReq, ServerOperationResp, ServerPreviewReq,
        ServerPreviewResp, ServerPromptsResp, ServerResourceTemplatesResp, ServerResourcesResp, ServerToolsResp,
        ServerUpdateReq, ServersImportReq, ServersImportResp,
    },
};
use crate::{aide_wrapper, aide_wrapper_payload, aide_wrapper_query};
use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with},
};
use axum::routing::post as axum_post;
use std::sync::Arc;

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
        .api_route(
            "/mcp/servers/preview",
            post_with(preview_servers_aide, preview_servers_docs),
        )
        // Management operations - Payload with action enum
        .api_route("/mcp/servers/manage", post_with(manage_server_aide, manage_server_docs))
        // Cache endpoints - Query parameters with refresh enum
        .api_route(
            "/mcp/servers/cache/detail",
            get_with(server_cache_detail_aide, server_cache_detail_docs),
        )
        .api_route(
            "/mcp/servers/cache/reset",
            post_with(server_cache_reset_aide, server_cache_reset_docs),
        )
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
            "/mcp/servers/oauth/config",
            post_with(configure_oauth_aide, configure_oauth_docs),
        )
        .api_route(
            "/mcp/servers/oauth/prepare",
            post_with(prepare_oauth_aide, prepare_oauth_docs),
        )
        .api_route(
            "/mcp/servers/oauth/initiate",
            post_with(start_oauth_aide, start_oauth_docs),
        )
        .route("/mcp/servers/oauth/callback", axum_post(server::complete_oauth))
        .api_route(
            "/mcp/servers/oauth/status",
            get_with(oauth_status_aide, oauth_status_docs),
        )
        .api_route(
            "/mcp/servers/oauth/revoke",
            post_with(revoke_oauth_aide, revoke_oauth_docs),
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
    ServerListResp,
    "List all MCP servers with optional filtering"
);

aide_wrapper_query!(
    server::server_details,
    ServerDetailsReq,
    ServerDetailsResp,
    "Get details for a specific MCP server"
);

aide_wrapper_query!(
    server::instance_list,
    InstanceListReq,
    InstanceListResp,
    "List instances for servers"
);

// Generate aide-compatible wrappers for CRUD server handlers
aide_wrapper_payload!(
    server::create_server,
    ServerCreateReq,
    ServerDetailsResp,
    "Create a new MCP server"
);

aide_wrapper_payload!(
    server::import_servers,
    ServersImportReq,
    ServersImportResp,
    "Import servers from JSON configuration"
);

// Preview capabilities for arbitrary server configs (no side effects)
aide_wrapper_payload!(
    server::preview_servers,
    ServerPreviewReq,
    ServerPreviewResp,
    "Preview capabilities for server configs without importing"
);

// Generate aide-compatible wrappers for capability handlers
aide_wrapper_query!(
    server::server_tools,
    ServerCapabilityReq,
    ServerToolsResp,
    "List all tools for a specific server"
);

aide_wrapper_query!(
    server::server_resources,
    ServerCapabilityReq,
    ServerResourcesResp,
    "List all resources for a specific server"
);

aide_wrapper_query!(
    server::server_resource_templates,
    ServerCapabilityReq,
    ServerResourceTemplatesResp,
    "List all resource templates for a specific server"
);

aide_wrapper_query!(
    server::server_prompts,
    ServerCapabilityReq,
    ServerPromptsResp,
    "List all prompts for a specific server"
);

aide_wrapper_payload!(
    server::configure_oauth,
    ServerOAuthConfigReq,
    OAuthStatusResp,
    "Configure OAuth settings for a specific server"
);

aide_wrapper_payload!(
    server::prepare_oauth,
    ServerOAuthPrepareReq,
    OAuthStatusResp,
    "Automatically discover and prepare OAuth configuration for a specific server"
);

aide_wrapper_payload!(
    server::start_oauth,
    ServerOAuthInitiateReq,
    OAuthInitiateResp,
    "Start OAuth authorization for a specific server"
);

aide_wrapper_query!(
    server::oauth_status,
    ServerOAuthStatusReq,
    OAuthStatusResp,
    "Get OAuth status for a specific server"
);

aide_wrapper_payload!(
    server::revoke_oauth,
    ServerOAuthRevokeReq,
    OAuthStatusResp,
    "Revoke stored OAuth token for a specific server"
);

aide_wrapper_query!(
    server::server_cache_detail,
    CacheDetailsReq,
    CacheDetailsResp,
    "Inspect capability cache detail for MCP servers"
);

aide_wrapper!(
    server::server_cache_reset,
    CacheResetResp,
    "Reset the MCP server capability cache"
);

// Generate aide-compatible wrappers for management handlers
aide_wrapper_payload!(
    server::manage_server,
    ServerManageReq,
    ServerOperationResp,
    "Manage server (enable/disable with sync options)"
);

// Generate aide-compatible wrapper for instance management
aide_wrapper_payload!(
    server::manage_instance,
    InstanceManageReq,
    ServerOperationResp,
    "Manage instance operations (disconnect, reconnect, recover, cancel)"
);

// Generate aide-compatible wrappers for updated CRUD handlers
aide_wrapper_payload!(
    server::update_server,
    ServerUpdateReq,
    ServerDetailsResp,
    "Update server configuration"
);

aide_wrapper_payload!(
    server::delete_server,
    ServerDeleteReq,
    ServerOperationResp,
    "Delete a server"
);

// Generate aide-compatible wrappers for instance handlers with query parameters
aide_wrapper_query!(
    server::get_instance,
    InstanceDetailsReq,
    InstanceDetailsResp,
    "Get details for a specific server instance"
);

aide_wrapper_query!(
    server::check_health,
    InstanceHealthReq,
    InstanceHealthResp,
    "Check health status of a specific server instance"
);
