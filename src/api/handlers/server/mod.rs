// MCPMate Proxy API handlers for MCP server management
// Contains handler functions for MCP server endpoints

// Re-export all public functions from submodules
pub use self::basic::{get_server, list_servers};
pub use self::crud::{create_server, import_servers, update_server};
pub use self::instance::list_instances;
pub use self::mgmt::{disable_server, enable_server};

// Submodules
mod basic;
mod crud;
mod instance;
mod mgmt;

// Common imports for all submodules
pub(crate) mod common {
    pub use axum::{
        extract::{Path, State},
        Json,
    };
    pub use std::sync::Arc;

    pub use crate::{
        api::{
            models::mcp::{
                CreateServerRequest, ImportServersRequest, ImportServersResponse,
                OperationResponse, ServerInstanceSummary, ServerInstancesResponse,
                ServerListResponse, ServerResponse, UpdateServerRequest,
            },
            routes::AppState,
        },
        conf::models::Server,
    };

    pub use crate::api::handlers::ApiError;
}
