//! Recore - refactor core module
//!
//! this is the refactor version of the original core module, using a more clear hierarchical architecture:
//! - foundation: infrastructure layer
//! - events: event system (independent first-level module)
//! - connection: single connection management
//! - transport: transport layer
//! - pool: connection pool management
//! - protocol: protocol handling
//! - audit: audit middleware
//! - proxy: proxy core

// infrastructure layer - does not depend on other modules
pub mod foundation;

// event system - independent infrastructure module
pub mod events;

// connection and transport layer - depends on infrastructure and event system
pub mod connection;
pub mod transport;

// connection pool layer - depends on connection and transport
pub mod pool;

// protocol handling layer - depends on connection pool
pub mod protocol;

// audit middleware layer - depends on protocol layer
pub mod audit;

// proxy core layer - depends on all lower modules
pub mod proxy;

// re-export core interfaces, keeping compatibility with the original core module
// note: these exports will be gradually enabled after the actual implementation of the modules

// pub use foundation::{
//     error::RecoreError,
//     types::*,
// };

pub use events::{
    Event, EventBus, EventHandlers, EventReceiver, init as init_events,
    init_with_handlers as init_events_with_handlers, needs_transport_ready_wait,
    wait_for_transport_ready,
};

// pub use connection::{
//     lifecycle::ConnectionLifecycle,
//     status::ConnectionStatus,
// };

// pub use pool::{
//     manager::UpstreamConnectionPool,
// };

pub use protocol::{
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

    // tool protocol
    call_upstream_tool,
    ensure_unique_name,
    find_tool_in_server,
    generate_unique_name,
    get_all_tools,
    get_prompt_status,
    get_resource_status,

    // prompt protocol
    get_upstream_prompt,
    is_prompt_enabled,
    is_resource_enabled,

    // resource protocol
    read_upstream_resource,
    resolve_unique_name,
    validate_prompt_name,
    validate_resource_uri,
};

// pub use proxy::{
//     engine::ProxyEngine,
//     handler::ProxyHandler,
// };
