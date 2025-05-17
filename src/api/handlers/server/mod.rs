// MCPMate Proxy API handlers for MCP server management
// Contains handler functions for MCP server endpoints

// Re-export all public functions from submodules
pub use self::{
    basic::{get_server, list_servers},
    crud::{create_server, import_servers, update_server},
    instance::list_instances,
    mgmt::{disable_server, enable_server},
};

// Submodules
mod basic;
mod crud;
mod instance;
mod mgmt;

// Common imports for all submodules
pub(crate) mod common {
    pub use std::sync::Arc;

    pub use axum::{
        Json,
        extract::{Path, State},
    };

    pub use crate::{
        api::{
            handlers::ApiError,
            models::mcp::{
                CreateServerRequest, ImportServersRequest, ImportServersResponse,
                OperationResponse, ServerInstanceSummary, ServerInstancesResponse,
                ServerListResponse, ServerResponse, UpdateServerRequest,
            },
            routes::AppState,
        },
        conf::models::Server,
    };
}
