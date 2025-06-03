//! Common utilities and shared functionality for MCPMate
//!
//! This module provides shared functionality used across different parts of MCPMate,
//! helping to eliminate code duplication and improve maintainability.

pub mod env;
pub mod json;
pub mod paths;
pub mod types;

// Re-export commonly used items for convenience
pub use env::{EnvironmentManager, create_runtime_environment, prepare_command_environment};
pub use json::strip_comments;
pub use paths::{MCPMatePaths, get_mcpmate_dir, global_paths};
