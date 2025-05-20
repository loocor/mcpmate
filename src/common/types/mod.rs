//! Common types for MCPMate
//!
//! This module contains common types used throughout the application.
//! These types are designed to replace string constants with type-safe enums.

// Re-export all types for convenience
pub mod server;
pub mod config;
pub mod status;

pub use server::{ServerType, TransportType};
pub use config::ConfigSuitType;
pub use status::EnabledStatus;
