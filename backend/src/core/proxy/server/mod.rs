//! Proxy server module index: minimal declarations and re-exports

mod common;
mod gateway;
mod prompts;
mod resources;
mod tools;

pub use common::{
    ClientContext, ClientIdentitySource, ClientTransport, ManagedClientContextResolver, ObservedClientInfo,
    SessionBinding, SessionBoundClientContextResolver, UnifiedHttpServer, UnifiedHttpServerConfig,
    resolve_initialize_context_parts, resolve_request_context_parts, supports_capability,
};
pub use gateway::ProxyServer;
