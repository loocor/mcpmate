//! Unified capability management module
//!
//! Complete MCP capability management including tools, resources, prompts, and unified query services.
//! Integrates the former protocol layer for a cohesive capability-centric architecture.

pub mod domain;
pub mod integration;
pub mod internal;
pub mod query;
pub mod unified;

// MCP capability implementations (formerly protocol layer)
pub mod naming;
pub mod prompts;
pub mod resolver;
pub mod resources;
pub mod server_mapping;
pub mod service;
pub mod tools;

// Main public interfaces
pub use domain::{
    AffinityKey, CapabilityError, CapabilityItem, CapabilityQuery, CapabilityResult, CapabilityType, ConnectionMode,
    DataSource, FreshnessRequirement, IsolationMode, QueryContext,
};

pub use unified::{ConnectionStats, InstanceMetadata, InstanceStatus, UnifiedConnectionManager};

pub use query::{MetricsCollector, UnifiedQueryService, UnifiedQueryServiceBuilder};

pub use integration::{UnifiedQueryAdapter, UnifiedQueryIntegration, migration::MigrationComparison};

// Re-export capability implementations for backward compatibility
pub use tools::{
    ToolMapping, ToolNameMapping, build_tool_mapping, find_tool_in_server, get_all_tools,
};

pub use resources::{
    ResourceMapping, ResourceTemplateMapping, build_resource_mapping, build_resource_template_mapping,
    get_resource_status, is_resource_enabled, read_upstream_resource, validate_resource_uri,
};

pub use prompts::{
    PromptMapping, PromptTemplateMapping, build_prompt_mapping, build_prompt_template_mapping, get_all_prompts,
    get_prompt_status, get_upstream_prompt, is_prompt_enabled, validate_prompt_name,
};

pub use server_mapping::{
    ServerInfo, ServerMappingManager, global_server_mapping_manager, initialize_server_mapping_manager,
};

pub use service::CapabilityService;
