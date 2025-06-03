// MCP Proxy module
// Contains functions and utilities for the MCP proxy server

// Module declarations
pub mod connection;
pub mod error;
pub mod events;
pub mod http;
pub mod loader;
pub mod models;
pub mod monitor;
pub mod proxy;
pub mod sse;
pub mod stdio;
pub mod suit;
pub mod tool;
pub mod transport;
pub mod types;
pub mod utils;

// Re-exports
pub use crate::common::server::TransportType;
pub use crate::core::http::pool::UpstreamConnectionPool;
pub use connection::UpstreamConnection;
pub use sse::connect_sse_server;
pub use suit::ConfigSuitMergeService;
pub use tool::{
    ToolMapping, ToolNameMapping, build_tool_mapping, call_upstream_tool, find_tool_in_server,
    get_all_tools,
};
pub use transport::connect_http_server;
pub use types::ConnectionStatus;
pub use utils::{get_connection_timeout, get_tools_timeout};
