// Transport layer module for core
// Provides abstractions for different transport types (stdio, sse, http, unified)

pub mod http;

// sse functions are merged into http module
pub mod stdio;
pub mod unified;

// Re-export TransportType from common
pub use crate::common::server::TransportType;

// Re-export main transport functions
pub use http::connect_http_server;
pub use http::connect_sse_server;
pub use unified::{connect_server, connect_server_simple};
