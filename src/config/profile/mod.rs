// Profile operations module for MCPMate
// Contains CRUD operations for profile, organized by functional domains

pub mod basic;
pub mod constants;
pub mod init;
pub mod mgmt;
pub mod prompt;
pub mod resource;
pub mod server;
pub mod tool;

// Basic query operations
pub use basic::{
    get_active_profile, get_all_profile, get_default_profile, get_default_profiles, get_profile, get_profile_by_name,
    get_profile_by_role, get_profile_by_type,
};

pub use constants::{
    DEFAULT_ANCHOR_INITIAL_NAME, DEFAULT_ANCHOR_ROLE, DEFAULT_PROFILE_DESCRIPTION, is_default_anchor_profile,
};

// Management operations
pub use mgmt::{
    delete_profile, ensure_default_anchor_profile_id, set_profile_active, set_profile_default, update_profile,
    upsert_profile,
};

// Prompt association operations
pub use prompt::{
    add_prompt_to_profile, get_enabled_prompts_for_profile, get_prompts_for_profile, remove_prompt_from_profile,
    update_prompt_enabled_status,
};

// Server association operations
pub use server::{
    ServerCapabilityAction, add_server_to_profile, get_profile_servers, remove_server_from_profile,
    sync_server_capabilities,
};

// Resource association operations
pub use resource::{
    add_resource_to_profile, get_enabled_resources_for_profile, get_resources_for_profile,
    remove_resource_from_profile, update_resource_enabled_status,
};

// Tool association operations
pub use tool::{
    ToolStatus,               // Tool status information for API responses
    ToolStatusService,        // Unified tool status service to eliminate code duplication
    add_tool_to_profile,      // Add a tool to a profile
    get_profile_tools,        // Get all tools for a profile
    remove_tool_from_profile, // Remove a tool from a profile
    update_tool_enabled_status,
};
