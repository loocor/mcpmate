// MCP Proxy API models module
// Contains data models for API requests and responses

pub mod audit;
pub mod cache;
pub mod client;
pub mod inspector;
pub mod oauth;
pub mod profile;
pub mod resp;
pub mod runtime;
pub mod server;
pub mod system;
pub mod token_estimate;

// Re-export commonly used types for convenience
pub use resp::{ErrorDetails, ErrorResp, ResponseConverter, SuccessResp};

// Common default value functions for serde
pub fn default_all() -> String {
    "all".to_string()
}

pub fn default_true() -> bool {
    true
}

pub fn default_false() -> bool {
    false
}

pub fn default_true_option() -> Option<bool> {
    Some(true)
}

pub fn default_false_option() -> Option<bool> {
    Some(false)
}
