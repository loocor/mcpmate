// Tool types module
// Contains type definitions for tool mapping and related functionality

use rmcp::model::Tool;

/// Tool mapping information
///
/// This struct represents the mapping between a tool name and the server/instance
/// that provides it. It is used to route tool calls to the appropriate upstream server.
#[derive(Debug, Clone)]
pub struct ToolMapping {
    /// Name of the server that provides this tool
    pub server_name: String,
    /// ID of the instance that provides this tool
    pub instance_id: String,
    /// Original tool definition
    pub tool: Tool,
    /// Original upstream tool name (without any modifications)
    pub upstream_tool_name: String,
}

/// Tool name mapping information
///
/// This struct represents the mapping between a client-facing tool name (which may include
/// a prefix) and the actual upstream tool name. It is used to handle tool name prefixing
/// and routing tool calls to the appropriate upstream server.
#[derive(Debug, Clone)]
pub struct ToolNameMapping {
    /// Client-facing tool name (with prefix if needed)
    pub client_tool_name: String,
    /// Server name
    pub server_name: String,
    /// Instance ID
    pub instance_id: String,
    /// Original upstream tool name (without any modifications)
    pub upstream_tool_name: String,
}
