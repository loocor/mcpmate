pub mod api;
pub mod common;
pub mod config;
pub mod core;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod macros;
pub mod runtime;
pub mod standalone;
pub mod system;

// Re-export FFI types for easier access
#[cfg(feature = "ffi")]
pub use ffi::{MCPMateEngine, ServiceInfo, ServiceStatus, StartupProgress};
