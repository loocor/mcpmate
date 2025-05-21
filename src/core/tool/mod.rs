// MCP Proxy tool module
// Contains functions for handling tool calls and routing them to upstream servers

// Module declarations
mod call;
mod mapping;
mod types;

// Re-exports
pub use call::call_upstream_tool;
pub use mapping::{build_tool_mapping, find_tool_in_server, get_all_tools};
pub use types::{ToolMapping, ToolNameMapping};
