// MCP Proxy API models for tool management
// Contains data models for tool endpoints

use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Import SDK types
use rmcp::model::Tool as RmcpTool;

/// Tool configuration request
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolConfigRequest {
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Prefixed/qualified name for the tool (to avoid conflicts)
    pub prefixed_name: Option<String>,
}

/// Tool response (replacing both ToolResponse and ToolConfigResponse)
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Unique ID (UUID from config_suit_tool table)
    pub id: String,
    /// Name of the server that provides this tool
    pub server_name: String,
    /// Name of the tool
    pub tool_name: String,
    /// Prefixed/qualified name for the tool (to avoid conflicts)
    pub prefixed_name: Option<String>,
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Allowed operations on this tool
    pub allowed_operations: Vec<String>,
}

/// Tool status response
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolStatusResponse {
    /// Unique ID (UUID from config_suit_tool table)
    pub id: String,
    /// Name of the server that provides this tool
    pub server_name: String,
    /// Name of the tool
    pub tool_name: String,
    /// Result of the operation
    pub result: String,
    /// Current status of the tool
    pub status: String,
    /// Allowed operations on this tool
    pub allowed_operations: Vec<String>,
}

/// Tool list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolListResponse {
    /// List of tools
    pub tools: Vec<ToolResponse>,
}

// For backward compatibility during transition
// TODO: Remove after API handlers are updated
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolConfigResponse {
    /// Unique ID
    pub id: String,
    /// Name of the server that provides this tool
    pub server_name: String,
    /// Name of the tool
    pub tool_name: String,
    /// Prefixed/qualified name for the tool (to avoid conflicts)
    pub prefixed_name: Option<String>,
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Allowed operations on this tool
    pub allowed_operations: Vec<String>,
}

/// Server tools response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerToolsResponse {
    /// Name of the server
    pub server_name: String,
    /// Server connection status
    pub status: String,
    /// List of tools provided by this server
    pub tools: Vec<ToolResponse>,
}

/// MCP Tool Annotations
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolAnnotations {
    /// Human-readable title for the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// If true, the tool does not modify its environment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    /// If true, the tool may perform destructive updates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    /// If true, repeated calls with same args have no additional effect
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    /// If true, tool interacts with external entities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

/// MCP Tool Definition
#[derive(Debug, Serialize, Deserialize)]
pub struct McpTool {
    /// Unique identifier for the tool
    pub name: String,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema for the tool's parameters
    pub input_schema: serde_json::Value,
    /// Optional hints about tool behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
}

/// MCP Tool List Response
#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolListResponse {
    /// List of tools
    pub tools: Vec<McpTool>,
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// MCP Tool Info Response
#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolInfoResponse {
    /// Tool definition
    pub tool: McpTool,
    /// MCPMate-specific metadata
    pub metadata: ToolResponse,
}

/// MCPMate Tool Info (combines MCP standard and MCPMate-specific fields)
#[derive(Debug, Serialize, Deserialize)]
pub struct McpMateToolInfo {
    /// MCP standard tool fields (flattened)
    #[serde(flatten)]
    pub mcp_tool: RmcpTool,
    /// MCPMate-specific fields
    pub id: String,
    pub server_name: String,
    pub enabled: bool,
    pub allowed_operations: Vec<String>,
}

/// Conversion from McpMateToolInfo to RmcpTool
impl From<McpMateToolInfo> for RmcpTool {
    fn from(info: McpMateToolInfo) -> Self {
        info.mcp_tool
    }
}

/// Conversion from RmcpTool and metadata to McpMateToolInfo
impl From<(RmcpTool, String, String, bool)> for McpMateToolInfo {
    fn from((tool, id, server_name, enabled): (RmcpTool, String, String, bool)) -> Self {
        McpMateToolInfo {
            mcp_tool: tool,
            id,
            server_name,
            enabled,
            allowed_operations: vec![if enabled {
                "disable".to_string()
            } else {
                "enable".to_string()
            }],
        }
    }
}

/// Conversion from ToolResponse to McpMateToolInfo
impl TryFrom<ToolResponse> for McpMateToolInfo {
    type Error = String;

    fn try_from(response: ToolResponse) -> Result<Self, Self::Error> {
        // Create a basic RmcpTool with minimal information
        let name = response
            .prefixed_name
            .unwrap_or_else(|| response.tool_name.clone());
        let description = Some(format!(
            "Tool provided by server '{}'",
            response.server_name
        ));

        // Create a minimal schema (this would need to be replaced with actual schema)
        let input_schema = Arc::new(serde_json::Map::new());

        // Create the RmcpTool
        let mcp_tool = RmcpTool {
            name: name.into(),
            description: description.map(|s| s.into()),
            input_schema,
            annotations: None,
        };

        // Create and return the McpMateToolInfo
        Ok(McpMateToolInfo {
            mcp_tool,
            id: response.id,
            server_name: response.server_name,
            enabled: response.enabled,
            allowed_operations: response.allowed_operations,
        })
    }
}
