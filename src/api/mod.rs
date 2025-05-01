// MCP Proxy API module
// Contains the RESTful API implementation for the MCP Proxy server

pub mod handlers;
pub mod models;
pub mod routes;
pub mod server;

// Re-export main API server type
pub use server::ApiServer;
