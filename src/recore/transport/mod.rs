// Transport layer module for recore
// Provides abstractions for different transport types (stdio, sse, http, unified)

pub mod http;
pub mod sse;
pub mod stdio;
pub mod unified;

// Re-export TransportType from common
pub use crate::common::server::TransportType;

// Re-export main transport functions
pub use http::connect_http_server; // with cancellation token
pub use sse::connect_sse_server; // with cancellation token
pub use stdio::{
    connect_stdio_server_with_ct,            // with cancellation token
    connect_stdio_server_with_ct_and_db,     // with cancellation token and database pool
    connect_stdio_server_with_runtime_cache, // with cancellation token and runtime cache
};
pub use unified::{
    connect_server,        // with cancellation token and runtime cache
    connect_server_simple, // with cancellation token without runtime cache
};
