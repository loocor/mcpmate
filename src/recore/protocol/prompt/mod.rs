//! Prompt protocol module
//!
//! Contains prompt management functionality for MCPMate

pub mod call;
pub mod mapping;
pub mod status;
pub mod types;

// Re-export commonly used types and functions
pub use call::{get_upstream_prompt, validate_prompt_name};
pub use mapping::{build_prompt_mapping, build_prompt_template_mapping};
pub use status::{get_prompt_status, is_prompt_enabled};
pub use types::{PromptMapping, PromptTemplateMapping};
