// MCP Proxy API models module
// Contains data models for API requests and responses

pub mod clients;
pub mod notifs;
pub mod resp;
pub mod server;
pub mod suits;
pub mod system;

// Re-export commonly used types for convenience
pub use resp::{ErrorDetails, ErrorResponse, ResponseConverter, SuccessResponse};
