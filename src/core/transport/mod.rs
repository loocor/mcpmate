// Transport layer module for core
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
// stdio functions are now accessed through unified interface
pub use unified::{
    connect_server,        // unified interface with cancellation token and runtime cache
    connect_server_simple, // simplified interface without runtime cache
};
