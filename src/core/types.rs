// Core types for MCPMan
// These types are shared across different transport modes

use rmcp::model::Tool;
use std::collections::HashMap;
use uuid::Uuid;

/// Status of a connection to an upstream server
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Connection is initializing
    Initializing,
    /// Connection is ready
    Ready,
    /// Connection is disconnected
    Disconnected,
    /// Connection failed
    Failed(String),
}

/// Resource usage information for a process
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// Process ID
    pub pid: Option<u32>,
    /// CPU usage in percentage (0-100)
    pub cpu_usage: Option<f32>,
    /// Memory usage in bytes
    pub memory_usage: Option<u64>,
}

/// Resource limits for a process
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum CPU usage in percentage (0-100)
    pub max_cpu: Option<f32>,
    /// Maximum memory usage in bytes
    pub max_memory: Option<u64>,
    /// Action to take when limits are exceeded
    pub action: ResourceLimitAction,
}

/// Action to take when resource limits are exceeded
#[derive(Debug, Clone)]
pub enum ResourceLimitAction {
    /// Log a warning
    Warn,
    /// Restart the process
    Restart,
    /// Terminate the process
    Terminate,
}

/// Tool information with server name
#[derive(Debug, Clone)]
pub struct ToolInfo {
    /// The tool
    pub tool: Tool,
    /// The server name
    pub server_name: String,
}

/// Tool mapping information
#[derive(Debug, Clone)]
pub struct ToolMapping {
    /// Map of tool name to server name
    pub tool_to_server: HashMap<String, String>,
    /// Map of server name to list of tools
    pub server_to_tools: HashMap<String, Vec<Tool>>,
}

/// Result of a tool call
#[derive(Debug, Clone)]
pub struct ToolCallResult {
    /// The result of the tool call
    pub result: serde_json::Value,
    /// The server name
    pub server_name: String,
    /// The instance ID
    pub instance_id: Uuid,
}
