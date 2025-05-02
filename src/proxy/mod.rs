// MCP Proxy module
// Contains functions and utilities for the MCP proxy server

// Module declarations
pub mod connection;
pub mod error;
pub mod pool;
pub mod server;
pub mod sse;
pub mod stdio;
pub mod tool;
pub mod types;
pub mod utils;

// Re-exports
pub use connection::UpstreamConnection;
pub use pool::UpstreamConnectionPool;
pub use server::ProxyServer;
pub use sse::connect_sse_server;
pub use stdio::connect_stdio_server;
pub use tool::{call_upstream_tool, get_all_tools};
pub use types::ConnectionStatus;
pub use utils::{get_connection_timeout, get_tools_timeout, prepare_command_env};
