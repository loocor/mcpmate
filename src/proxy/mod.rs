// MCP Proxy module
// Contains functions and utilities for the MCP proxy server

// Module declarations
pub mod connection;
pub mod pool;
pub mod server;
pub mod sse;
pub mod stdio;
pub mod utils;

pub use connection::UpstreamConnection;
pub use pool::UpstreamConnectionPool;
pub use server::ProxyServer;
pub use sse::connect_sse_server;
pub use stdio::connect_stdio_server;
pub use utils::{get_connection_timeout, get_tools_timeout, prepare_command_env};

/// Connection status for an upstream server
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Server is connected and operational
    Connected,
    /// Server is disconnected
    Disconnected,
    /// Server is in the process of connecting
    Connecting,
    /// Server connection failed with an error
    Failed(String),
}
