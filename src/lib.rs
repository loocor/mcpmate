pub mod api;
pub mod common;
pub mod config;
pub mod core;
pub mod interop;
pub mod macros;
pub mod runtime;
pub mod system;

// Re-export FFI types for easier access
#[cfg(feature = "interop")]
pub use interop::{MCPMateEngine, ServiceInfo, ServiceStatus, StartupProgress};
