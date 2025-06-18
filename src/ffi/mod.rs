//! FFI (Foreign Function Interface) module
//!
//! This module provides C-compatible interfaces for Swift integration.
//! It exposes minimal functionality for service lifecycle management only.

#[cfg(feature = "ffi")]
pub mod bridge;
#[cfg(feature = "ffi")]
pub mod engine;
#[cfg(feature = "ffi")]
pub mod types;

// Note: bridge module is used internally by swift-bridge
#[cfg(feature = "ffi")]
pub use engine::MCPMateEngine;
#[cfg(feature = "ffi")]
pub use types::*;
