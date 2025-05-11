// MCP Proxy API models for tool management
// Contains data models for tool endpoints

use serde::{Deserialize, Serialize};

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
