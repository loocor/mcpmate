//! Proxy server module index: minimal declarations and re-exports

mod common;
mod prompts;
mod resources;
mod server;
mod tools;

pub use common::{UnifiedHttpServer, UnifiedHttpServerConfig, supports_capability};
pub use server::ProxyServer;
