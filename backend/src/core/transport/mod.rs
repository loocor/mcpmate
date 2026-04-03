// Transport layer module for core
// Provides abstractions for different transport types (stdio, sse, http, unified)

pub mod http;
#[cfg(test)]
mod http_auth_tests;

// sse functions are merged into http module
pub mod client;
pub mod stdio;
pub mod unified;

// Unified alias for upstream client service with our handler type
pub type ClientService = rmcp::service::RunningService<rmcp::RoleClient, client::UpstreamClientHandler>;

// Re-export TransportType from common
pub use crate::common::server::TransportType;

// Re-export main transport functions
pub use http::connect_http_server;
pub use http::connect_http_server_with_client;
pub use http::connect_http_server_with_client_timeouts;
pub use unified::{connect_server, connect_server_simple};
