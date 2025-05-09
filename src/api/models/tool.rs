// MCP Proxy API models for tool management
// Contains data models for tool endpoints

use serde::{Deserialize, Serialize};

/// Tool configuration request
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolConfigRequest {
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Alias name for the tool (user-defined)
    pub alias_name: Option<String>,
}

/// Tool configuration response
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolConfigResponse {
    /// Unique ID
    pub id: i64,
    /// Name of the server that provides this tool
    pub server_name: String,
    /// Name of the tool
    pub tool_name: String,
    /// Alias name for the tool (user-defined)
    pub alias_name: Option<String>,
    /// Display name (alias_name if set, otherwise tool_name)
    pub display_name: String,
    /// Whether the tool is enabled
    pub enabled: bool,
    /// When the configuration was created
    pub created_at: Option<String>,
    /// When the configuration was last updated
    pub updated_at: Option<String>,
    /// Allowed operations on this tool
    pub allowed_operations: Vec<String>,
}

/// Tool status response
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolStatusResponse {
    /// Unique ID
    pub id: i64,
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

/// Tool response
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResponse {
    /// Unique ID
    pub id: i64,
    /// Name of the server that provides this tool
    pub server_name: String,
    /// Name of the tool
    pub tool_name: String,
    /// Alias name for the tool (user-defined)
    pub alias_name: Option<String>,
    /// Display name (alias_name if set, otherwise tool_name)
    pub display_name: String,
    /// Whether the tool is enabled
    pub enabled: bool,
    /// When the configuration was created
    pub created_at: Option<String>,
    /// When the configuration was last updated
    pub updated_at: Option<String>,
}

/// Tool list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolListResponse {
    /// List of tools
    pub tools: Vec<ToolResponse>,
}
