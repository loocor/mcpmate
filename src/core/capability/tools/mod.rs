//! Tool protocol module
//!
//! This module contains functions for handling tool calls and routing them to upstream servers.
//! It also provides tool name management functionality.

// Module declarations
mod mapping;
mod types;

// Re-exports
pub use mapping::{build_tool_mapping, build_tool_mapping_filtered, find_tool_in_server, get_all_tools};
pub use types::{ToolMapping, ToolNameMapping};
