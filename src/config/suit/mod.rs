// Config Suit operations module for MCPMate
// Contains CRUD operations for configuration suits, organized by functional domains

pub mod basic;
pub mod init;
pub mod mgmt;
pub mod prompt;
pub mod resource;
pub mod server;
pub mod tool;

// Basic query operations
pub use basic::{
    get_active_config_suits, get_all_config_suits, get_config_suit, get_config_suit_by_name,
    get_config_suits_by_type, get_default_config_suit,
};

// Management operations
pub use mgmt::{
    delete_config_suit, set_config_suit_active, set_config_suit_default, upsert_config_suit,
    upsert_config_suit_tx,
};

// Prompt association operations
pub use prompt::{
    add_prompt_to_config_suit, get_enabled_prompts_for_config_suit, get_prompts_for_config_suit,
    remove_prompt_from_config_suit, update_prompt_enabled_status,
};

// Server association operations
pub use server::{
    add_server_to_config_suit, get_config_suit_servers, remove_server_from_config_suit,
};

// Resource association operations
pub use resource::{
    add_resource_to_config_suit, get_enabled_resources_for_config_suit,
    get_resources_for_config_suit, remove_resource_from_config_suit,
    update_resource_enabled_status,
};

// Tool association operations
pub use tool::{
    ToolStatus,                   // Tool status information for API responses
    ToolStatusService,            // Unified tool status service to eliminate code duplication
    add_tool_to_config_suit,      // Add a tool to a config suit
    get_config_suit_tools,        // Get all tools for a config suit
    remove_tool_from_config_suit, // Remove a tool from a config suit
};
