//! Resource protocol module
//!
//! Contains resource management functionality for MCPMate

pub mod call;
pub mod mapping;
pub mod status;
pub mod types;

// Re-export commonly used types and functions
pub use call::{read_upstream_resource, validate_resource_uri};
pub use mapping::{
    build_resource_mapping, build_resource_template_mapping, get_all_resource_templates,
    get_all_resources,
};
pub use status::{get_resource_status, is_resource_enabled};
pub use types::{ResourceMapping, ResourceTemplateMapping};
