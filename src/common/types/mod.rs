//! Common types for MCPMate
//!
//! This module contains common types used throughout the application.
//! These types are designed to replace string constants with type-safe enums.

// Re-export all types for convenience
pub mod config;
pub mod connection;
pub mod server;
pub mod status;

pub use config::ConfigSuitType;
pub use connection::ConnectionOperation;
pub use server::{ServerType, TransportType};
pub use status::EnabledStatus;
