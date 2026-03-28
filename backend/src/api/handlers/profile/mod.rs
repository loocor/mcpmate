// MCPMate Proxy API handlers for Profile management
// Contains handler functions for Profile endpoints

// Re-export all public functions from submodules
pub use self::{
    capabilities::{component_manage, prompts_list, resource_templates_list, resources_list, tools_list},
    capability_token_ledger::capability_token_ledger,
    helpers::{get_profile_or_error, get_tool_or_error, get_tool_with_details_or_error},
    mgmt::{profile_create, profile_delete, profile_details, profile_list, profile_manage, profile_update},
    server::{server_manage, servers_list},
    token_estimate::token_estimate,
};

// Submodules
mod capabilities;
mod capability_token_ledger;
pub mod helpers;
mod mgmt;
mod server;
mod token_estimate;
mod unified_capability_query;

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
            models::{ResponseConverter, profile::ProfileData},
            routes::AppState,
        },
        config::models::Profile,
    };

    /// Get database reference from AppState
    pub async fn get_database(state: &Arc<AppState>) -> Result<Arc<crate::config::database::Database>, ApiError> {
        match state.http_proxy.as_ref().and_then(|p| p.database.clone()) {
            Some(db) => Ok(db),
            None => Err(ApiError::InternalError("Database not available".to_string())),
        }
    }

    /// Convert Profile to ProfileResponse using unified converter
    pub fn profile_to_response(profile: &Profile) -> ProfileData {
        ResponseConverter::profile_to_response(profile)
    }
}
