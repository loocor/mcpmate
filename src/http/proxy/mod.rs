// HTTP proxy server module for MCPMate
//
// This module contains the implementation of the HTTP proxy server that aggregates
// tools from multiple MCP servers and exposes them through various transport protocols.

use std::sync::{Arc, OnceLock};

// Module declarations
mod core;
mod handler;
mod mapping;
mod transport;

// Re-exports
pub use core::HttpProxyServer;

// Internal re-exports for use within the module
pub(crate) use mapping::get_tool_name_mapping;
pub(crate) use transport::{start_sse, start_streamable_http, start_unified};

// Global proxy server instance
static PROXY_SERVER: OnceLock<Arc<HttpProxyServer>> = OnceLock::new();

/// Set the global proxy server instance
pub fn set_proxy_server(server: Arc<HttpProxyServer>) {
    let _ = PROXY_SERVER.set(server);
}

/// Get the global proxy server instance
pub fn get_proxy_server() -> Option<Arc<HttpProxyServer>> {
    PROXY_SERVER.get().cloned()
}
