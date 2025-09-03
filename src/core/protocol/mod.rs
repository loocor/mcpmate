//! Protocol module
//!
//! Contains all MCP protocol handling functionality including Tool, Resource, and Prompt protocols

pub mod prompt;
pub mod resource;
pub mod tool;
pub mod resolver;

// Re-export commonly used types and functions from each protocol
pub use tool::{
    DatabaseToolService, ToolMapping, ToolNameMapping, build_tool_mapping, call_upstream_tool,
    ensure_unique_name, find_tool_in_server, generate_unique_name, get_all_tools,
    resolve_unique_name,
};

pub use resource::{
    ResourceMapping, ResourceTemplateMapping, build_resource_mapping,
    build_resource_template_mapping, get_all_resource_templates, get_all_resources,
    get_resource_status, is_resource_enabled, read_upstream_resource, validate_resource_uri,
};

pub use prompt::{
    PromptMapping, PromptTemplateMapping, build_prompt_mapping, build_prompt_template_mapping,
    get_all_prompts, get_prompt_status, get_upstream_prompt, is_prompt_enabled,
    validate_prompt_name,
};
