// MCP Proxy tool module
// Contains functions for handling tool calls and routing them to upstream servers

// Module declarations
mod call;
mod mapping;
mod prefix;
mod types;

// Re-exports
pub use call::call_upstream_tool;
pub use mapping::{build_tool_mapping, find_tool_in_server, get_all_tools};
pub use prefix::{build_name_mapping, detect_common_prefix, get_all_with_prefix, parse_tool_name};
pub use types::{ToolMapping, ToolNameMapping};
