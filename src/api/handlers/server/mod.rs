// MCPMate Proxy API handlers for MCP server management
// Contains handler functions for MCP server endpoints

// Re-export all public functions from submodules
pub use self::{
    basic::{get_server, list_servers},
    crud::{create_server, delete_server, import_servers, update_server},
    instance::list_instances,
    mgmt::{disable_server, enable_server},
    prompts::{get_prompt_arguments, list_prompts},
    resources::{list_resource_templates, list_resources},
    tools::{get_tool_detail, list_tools},
};

// Submodules
mod basic;
mod crud;
mod instance;
mod mgmt;

// Inspect functionality
mod prompts;
mod resources;
mod tools;

// Shared utilities
pub mod common;

// Common imports for all submodules
pub(crate) mod shared {
    pub use std::sync::Arc;

    pub use axum::{
        Json,
        extract::{Path, Query, State},
    };

    pub use crate::{
        api::{
            handlers::ApiError,
            models::server::{
                CreateServerRequest, ImportServersRequest, ImportServersResponse,
                OperationResponse, ServerInstancesResponse, ServerListResponse, ServerResponse,
                UpdateServerRequest,
            },
            routes::AppState,
        },
        config::models::Server,
    };
}
