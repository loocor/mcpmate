// MCP Proxy API models module
// Contains data models for API requests and responses

pub mod notifs;
pub mod server;
pub mod suits;
pub mod system;

/// Generic success response
#[derive(serde::Serialize)]
pub struct SuccessResponse {
    /// Success message
    pub message: String,
}

/// Generic error response
#[derive(serde::Serialize)]
pub struct ErrorResponse {
    /// Error details
    pub error: ErrorDetails,
}

/// Error details
#[derive(serde::Serialize)]
pub struct ErrorDetails {
    /// Error message
    pub message: String,
    /// HTTP status code
    pub status: u16,
}
