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

pub mod ai;
pub mod cache;
pub mod capability;
pub mod events;
pub mod foundation;
pub mod models;
pub mod pool;
pub mod profile;
pub mod proxy;
pub mod transport;

pub use ai::{AiConfig, TextMcpExtractor, default_model_path, extract_mcp_config};
pub use events::{
    Event, EventBus, EventHandlers, EventReceiver, init as init_events,
    init_with_handlers as init_events_with_handlers, needs_transport_ready_wait, wait_for_transport_ready,
};
pub use proxy::{Args as ProxyArgs, ProxyServer};
