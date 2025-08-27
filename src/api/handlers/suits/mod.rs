// MCPMate Proxy API handlers for Config Suit management
// Contains handler functions for Config Suit endpoints

// Re-export all public functions from submodules
pub use self::{
    capabilities::{component_manage, prompts_list, resources_list, tools_list},
    helpers::{get_suit_or_error, get_tool_or_error, get_tool_with_details_or_error},
    mgmt::{suit_create, suit_delete, suit_details, suit_manage, suit_update, suits_list},
    server::{server_manage, servers_list},
};

// Submodules
mod capabilities;
pub mod helpers;
mod mgmt;
mod server;

// Common imports for all submodules
pub(crate) mod common {
    pub use std::sync::Arc;

    pub use axum::{
        Json,
        extract::{Query, State},
    };

    pub use crate::{
        api::{
            handlers::ApiError,
            models::{ResponseConverter, suits::SuitData},
            routes::AppState,
        },
        config::models::ConfigSuit,
    };

    /// Get database reference from AppState
    pub async fn get_database(state: &Arc<AppState>) -> Result<Arc<crate::config::database::Database>, ApiError> {
        match state.http_proxy.as_ref().and_then(|p| p.database.clone()) {
            Some(db) => Ok(db),
            None => Err(ApiError::InternalError("Database not available".to_string())),
        }
    }

    /// Convert ConfigSuit to ConfigSuitResponse using unified converter
    pub fn suit_to_response(suit: &ConfigSuit) -> SuitData {
        ResponseConverter::suit_to_response(suit)
    }
}
