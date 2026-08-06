// Profile operations module for MCPMate
// Contains CRUD operations for profile, organized by functional domains

pub mod basic;
pub mod capability_ref;
pub mod constants;
pub mod init;
pub mod mgmt;
pub mod prompt;
pub mod resource;
pub mod resource_template;
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

// System-owned default anchor normalization
pub use mgmt::ensure_default_anchor_profile_id;
pub(crate) use mgmt::normalize_default_anchor_profile;

// Prompt association operations
#[cfg(test)]
pub(crate) use prompt::add_prompt_to_profile;
pub use prompt::{get_enabled_prompts_for_profile, get_prompts_for_profile};

// Server association operations
#[cfg(test)]
pub(crate) use server::add_server_to_profile;
pub use server::get_profile_servers;

// Resource association operations
#[cfg(test)]
pub(crate) use resource::add_resource_to_profile;
pub use resource::{get_enabled_resources_for_profile, get_resources_for_profile};

// Resource template association operations
#[cfg(test)]
pub(crate) use resource_template::add_resource_template_to_profile;
pub use resource_template::{
    build_enabled_resource_templates_query, get_enabled_resource_templates_for_profile,
    get_resource_templates_for_profile,
};

// Tool association operations
#[cfg(test)]
pub(crate) use tool::add_tool_to_profile;
pub use tool::{
    ToolStatus,        // Tool status information for API responses
    ToolStatusService, // Unified tool status service to eliminate code duplication
    get_profile_tools, // Get all tools for a profile
};
