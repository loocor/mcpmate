//! Proxy server module index: minimal declarations and re-exports

mod common;
mod gateway;
mod prompts;
mod resources;
mod tools;

pub use common::{UnifiedHttpServer, UnifiedHttpServerConfig, supports_capability};
pub use gateway::ProxyServer;
