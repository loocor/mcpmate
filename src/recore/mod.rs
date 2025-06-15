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

// pub use events::{
//     EventBus,
//     EventHandler,
//     EventWaiter,
// };

// pub use connection::{
//     lifecycle::ConnectionLifecycle,
//     status::ConnectionStatus,
// };

// pub use pool::{
//     manager::UpstreamConnectionPool,
// };

// pub use protocol::{
//     tool::ToolMapping,
//     resource::ResourceMapping,
//     prompt::PromptMapping,
// };

// pub use proxy::{
//     engine::ProxyEngine,
//     handler::ProxyHandler,
// };
