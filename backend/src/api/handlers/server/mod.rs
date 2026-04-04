// MCPMate Proxy API handlers for MCP server management
// Contains handler functions for MCP server endpoints

// Re-export all public functions from submodules
pub use self::{
    basic::{instance_list, server_details, server_list},
    capability::{server_cache_detail, server_cache_reset},
    crud::{create_server, delete_server, import_servers, update_server},
    instance::{check_health, get_instance, manage_instance},
    mgmt::{disable_server, enable_server, manage_server},
    oauth::{complete_oauth, configure_oauth, oauth_status, prepare_oauth, revoke_oauth, start_oauth},
    preview::preview_servers,
    prompts::server_prompts,
    resources::{server_resource_templates, server_resources},
    tools::server_tools,
};

// Submodules
mod basic;
mod crud;
mod mgmt;
mod oauth;

// Inspect functionality
mod preview;
mod prompts;
mod resources;
mod tools;

// Shared utilities
pub mod capability;
pub mod common;
pub mod instance;

// Common imports for all submodules
pub(crate) mod shared {
    pub use std::sync::Arc;

    pub use axum::{
        Json,
        extract::{Path, Query, State},
    };

    pub use crate::{
        api::{handlers::ApiError, models::server::ServerOperationData, routes::AppState},
        config::models::Server,
    };
}
