//! Tool protocol module
//!
//! This module contains functions for handling tool calls and routing them to upstream servers.
//! It also provides tool name management functionality.

// Module declarations
mod call;
mod mapping;
mod naming;
mod types;

// Re-exports
pub use call::call_upstream_tool;
pub use mapping::{build_tool_mapping, find_tool_in_server, get_all_tools};
pub use naming::{ensure_unique_name, generate_unique_name, resolve_unique_name};
pub use types::{ToolMapping, ToolNameMapping};
