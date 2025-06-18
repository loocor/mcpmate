//! Interop (Interoperability) module
//!
//! This module provides cross-language interfaces for desktop application integration.
//! It exposes minimal functionality for service lifecycle management only.

#[cfg(feature = "interop")]
pub mod bridge;
#[cfg(feature = "interop")]
pub mod engine;
#[cfg(feature = "interop")]
pub mod types;

// Note: bridge module is used internally for language interoperability
#[cfg(feature = "interop")]
pub use engine::MCPMateEngine;
#[cfg(feature = "interop")]
pub use types::*;
