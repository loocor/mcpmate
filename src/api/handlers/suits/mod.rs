// MCPMate Proxy API handlers for Config Suit management
// Contains handler functions for Config Suit endpoints

// Re-export all public functions from submodules
pub use self::{
    basic::{get_suit, list_suits},
    crud::{create_suit, delete_suit, update_suit},
    helpers::{
        check_resource_belongs_to_suit, check_tool_belongs_to_suit, get_resource_by_id,
        get_resource_or_error, get_server_or_error, get_suit_or_error, get_tool_or_error,
    },
    mgmt::{activate_suit, batch_activate_suits, batch_deactivate_suits, deactivate_suit},
    prompt::{
        batch_disable_prompts, batch_enable_prompts, disable_prompt, enable_prompt, list_prompts,
    },
    resource::{
        batch_disable_resources, batch_enable_resources, disable_resource, enable_resource,
        list_resources,
    },
    server::{
        batch_disable_servers, batch_enable_servers, disable_server, enable_server, list_servers,
    },
    tool::{batch_disable_tools, batch_enable_tools, disable_tool, enable_tool, list_tools},
};

// Submodules
mod basic;
mod crud;
pub mod helpers;
mod mgmt;
mod prompt;
mod resource;
mod server;
mod tool;

// Common imports for all submodules
pub(crate) mod common {
    pub use std::sync::Arc;

    pub use axum::{
        Json,
        extract::{Path, Query, State},
    };

    pub use crate::{
        api::{
            handlers::ApiError,
            models::suits::{
                BatchOperationRequest, BatchOperationResponse, ConfigSuitListResponse,
                ConfigSuitPromptResponse, ConfigSuitPromptsResponse, ConfigSuitResourceResponse,
                ConfigSuitResourcesResponse, ConfigSuitResponse, ConfigSuitServerResponse,
                ConfigSuitServersResponse, ConfigSuitToolResponse, ConfigSuitToolsResponse,
                CreateConfigSuitRequest, SuitOperationResponse, UpdateConfigSuitRequest,
            },
            routes::AppState,
        },
        common::config::ConfigSuitType,
        config::models::{ConfigSuit, ConfigSuitServer, ConfigSuitTool},
    };

    /// Get database reference from AppState
    pub async fn get_database(
        state: &Arc<AppState>
    ) -> Result<Arc<crate::config::database::Database>, ApiError> {
        match state.http_proxy.as_ref().and_then(|p| p.database.clone()) {
            Some(db) => Ok(db),
            None => Err(ApiError::InternalError(
                "Database not available".to_string(),
            )),
        }
    }

    /// Convert ConfigSuit to ConfigSuitResponse
    pub fn suit_to_response(suit: &ConfigSuit) -> ConfigSuitResponse {
        let mut allowed_operations = Vec::new();

        // Add allowed operations based on current state
        if suit.is_active {
            allowed_operations.push("deactivate".to_string());
        } else {
            allowed_operations.push("activate".to_string());
        }

        // Always allow update and delete
        allowed_operations.push("update".to_string());
        allowed_operations.push("delete".to_string());

        ConfigSuitResponse {
            id: suit.id.clone().unwrap_or_default(),
            name: suit.name.clone(),
            description: suit.description.clone(),
            suit_type: suit.suit_type_string(),
            multi_select: suit.multi_select,
            priority: suit.priority,
            is_active: suit.is_active,
            is_default: suit.is_default,
            allowed_operations,
        }
    }
}
