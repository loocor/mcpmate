//! Core - refactor core module
//!
//! this is the refactor version of the original core module, using a more clear hierarchical architecture:
//! - foundation: infrastructure layer
//! - models: data models (independent)
//! - events: event system (independent first-level module)
//! - transport: transport layer
//! - pool: connection pool management
//! - protocol: protocol handling
//! - proxy: proxy core

// infrastructure layer - does not depend on other modules
pub mod foundation;
pub mod instrumentation;

// data models - independent
pub mod models;

// event system - independent infrastructure module
pub mod events;

// transport layer - depends on infrastructure and event system
pub mod transport;

// connection pool layer - depends on transport
pub mod pool;

// protocol handling has been merged into capability module

// proxy core business logic - depends on protocol layer
pub mod proxy;

// profile configuration business logic - depends on foundation layer
pub mod profile;

// cache system - high-performance Redb-based caching
pub mod cache;

// AI system - for MCP configuration extraction
pub mod ai;

// Unified capability query system - for tools, resources, prompts queries
pub mod capability;
pub mod sandwich;

// proxy core business logic - depends on protocol layer
pub use proxy::{Args as ProxyArgs, ProxyServer};

// re-export core interfaces, keeping compatibility with the original core module
pub use events::{
    Event, EventBus, EventHandlers, EventReceiver, init as init_events,
    init_with_handlers as init_events_with_handlers, needs_transport_ready_wait, wait_for_transport_ready,
};

// Re-export capability functions for backward compatibility
pub use capability::{
    PromptMapping,
    PromptTemplateMapping,
    ResourceMapping,
    ResourceTemplateMapping,

    ToolMapping,
    ToolNameMapping,

    // build functions
    build_prompt_mapping,
    build_prompt_template_mapping,
    build_resource_mapping,
    build_resource_template_mapping,
    build_tool_mapping,

    // tool functions
    find_tool_in_server,

    // prompt functions
    get_all_prompts,
    get_all_tools,
    get_prompt_status,
    get_resource_status,
    get_upstream_prompt,
    is_prompt_enabled,
    is_resource_enabled,
    read_upstream_resource,
    validate_prompt_name,
    validate_resource_uri,
};

// AI module re-exports
pub use ai::{AiConfig, TextMcpExtractor, default_model_path, extract_mcp_config};
