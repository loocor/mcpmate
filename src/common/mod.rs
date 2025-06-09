//! Common utilities and shared functionality for MCPMate
//!
//! This module provides shared functionality used across different parts of MCPMate,
//! helping to eliminate code duplication and improve maintainability.

pub mod config;
pub mod connection;
pub mod env;
pub mod json;
pub mod paths;
pub mod server;
pub mod status;
pub mod types;

// Re-export commonly used items for convenience
pub use env::{EnvironmentManager, create_runtime_environment, prepare_command_environment};
pub use json::strip_comments;
pub use paths::{MCPMatePaths, get_bridge_path, global_paths};
pub use types::ClientCategory;
