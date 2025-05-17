// MCP Proxy module
// Contains functions and utilities for the MCP proxy server

// Module declarations
pub mod connection;
pub mod error;
pub mod loader;
pub mod models;
pub mod monitor;
pub mod sse;
pub mod stdio;
pub mod suit;
pub mod tool;
pub mod transport;
pub mod types;
pub mod utils;

// Re-exports
pub use connection::UpstreamConnection;
pub use sse::connect_sse_server;
pub use stdio::connect_stdio_server;
pub use suit::ConfigSuitMergeService;
pub use tool::{
    ToolMapping, ToolNameMapping, build_name_mapping, build_tool_mapping, call_upstream_tool,
    detect_common_prefix, find_tool_in_server, get_all_tools, get_all_with_prefix, parse_tool_name,
};
pub use transport::{TransportType, connect_http_server};
pub use types::ConnectionStatus;
pub use utils::{get_connection_timeout, get_tools_timeout, prepare_command_env};

pub use crate::http::pool::UpstreamConnectionPool;
