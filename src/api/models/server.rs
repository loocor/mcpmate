// MCP Proxy API models for MCP server management
// Contains data models for MCP server endpoints

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// API Model
//

/// Instance status information
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceStatus {
    /// Instance ID
    pub id: String,
    /// Instance status
    pub status: String,
}

/// Server status response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatusResponse {
    /// Server name
    pub name: String,
    /// Server status summary
    pub status: String,
    /// List of instances
    pub instances: Vec<InstanceStatus>,
}

/// Instance Summary
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstanceSummary {
    /// Instance ID
    pub id: String,
    /// Instance status
    pub status: String,
    /// Started at time (ISO 8601 format)
    pub started_at: Option<String>,
    /// Connected at time (ISO 8601 format)
    pub connected_at: Option<String>,
}

/// Server Response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerResponse {
    /// Server name
    pub name: String,
    /// Is enabled in configuration
    pub enabled: bool,
    /// Summary of instances
    pub instances: Vec<ServerInstanceSummary>,
}

/// Server Instances Response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstancesResponse {
    /// Server name
    pub name: String,
    /// List of instances
    pub instances: Vec<ServerInstanceSummary>,
}

/// Server Instance Details
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstanceDetails {
    /// Connection attempts
    pub connection_attempts: u32,
    /// Last connected time (seconds since connection)
    pub last_connected_seconds: Option<u64>,
    /// Number of tools available
    pub tools_count: usize,
    /// Error message if status is Error
    pub error_message: Option<String>,
    /// Server type (stdio, sse, etc.)
    pub server_type: String,
    /// Process ID
    pub process_id: Option<u32>,
    /// CPU usage percentage of the instance process
    pub cpu_usage: Option<f32>,
    /// Memory usage in bytes of the instance process
    pub memory_usage: Option<u64>,
    /// Last health check time (ISO 8601 format)
    pub last_health_check: Option<String>,
}

/// Server Instance Response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstanceResponse {
    /// Instance ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Instance status
    pub status: String,
    /// Allowed operations
    pub allowed_operations: Vec<String>,
    /// Instance details
    pub details: ServerInstanceDetails,
}

/// Instance Health Response
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceHealthResponse {
    /// Instance ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Is instance healthy
    pub healthy: bool,
    /// Health check message
    pub message: String,
    /// Current status
    pub status: String,
    /// Last health check time (ISO 8601 format)
    pub checked_at: String,
    // TODO: Consider adding the following fields in the future:
    // - resource_metrics: Option<ResourceMetrics> - Resource usage metrics for the instance
    // - connection_stability: Option<f32> - Stability score based on connection history
    // - health_history: Option<Vec<HealthRecord>> - Recent health check history
}

/// Operation Request
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationRequest {
    /// Force the operation (optional, for disconnect)
    pub force: Option<bool>,
    /// Reset the connection (optional, for reconnect)
    pub reset: Option<bool>,
}

/// Operation Response
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationResponse {
    /// Instance ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Operation result (success or error message)
    pub result: String,
    /// New status after operation
    pub status: String,
    /// Allowed operations after this operation
    pub allowed_operations: Vec<String>,
}

/// Server list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerListResponse {
    /// List of servers
    pub servers: Vec<ServerResponse>,
}

/// Instance details
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceDetails {
    /// Instance ID
    pub id: String,
    /// Instance status
    pub status: String,
    /// Connection attempts
    pub connection_attempts: u32,
    /// Last connected time (seconds since connection)
    pub last_connected_seconds: Option<u64>,
    /// Number of tools available
    pub tools_count: usize,
    /// Error message if status is Error
    pub error_message: Option<String>,
}

/// Detailed server response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerDetailsResponse {
    /// Server name
    pub name: String,
    /// Server status summary
    pub status: String,
    /// Server type (stdio, sse, etc.)
    pub server_type: String,
    /// Is enabled in configuration
    pub is_enabled: bool,
    /// List of instances
    pub instances: Vec<InstanceDetails>,
}

/// Create server request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateServerRequest {
    /// Server name
    pub name: String,
    /// Server type (stdio, sse, streamable_http)
    pub kind: String,
    /// Command to execute (for stdio servers)
    pub command: Option<String>,
    /// URL (for sse and streamable_http servers)
    pub url: Option<String>,
    /// Arguments to pass to the command (for stdio servers)
    pub args: Option<Vec<String>>,
    /// Environment variables to set (for stdio servers)
    pub env: Option<HashMap<String, String>>,
    /// Whether to enable the server in the default config suit
    pub enabled: Option<bool>,
}

/// Update server request
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateServerRequest {
    /// Server type (stdio, sse, streamable_http)
    pub kind: Option<String>,
    /// Command to execute (for stdio servers)
    pub command: Option<String>,
    /// URL (for sse and streamable_http servers)
    pub url: Option<String>,
    /// Arguments to pass to the command (for stdio servers)
    pub args: Option<Vec<String>>,
    /// Environment variables to set (for stdio servers)
    pub env: Option<HashMap<String, String>>,
    /// Whether to enable the server in the default config suit
    pub enabled: Option<bool>,
}

/// Import servers request
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportServersRequest {
    /// Map of MCP server name to configuration
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, ImportServerConfig>,
}

/// Import server configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportServerConfig {
    /// Type of the server (stdio, sse, streamable_http)
    #[serde(rename = "type")]
    pub kind: String,
    /// Command to execute (for stdio servers)
    pub command: Option<String>,
    /// Arguments to pass to the command (for stdio servers)
    pub args: Option<Vec<String>>,
    /// URL (for sse and streamable_http servers)
    pub url: Option<String>,
    /// Environment variables to set (for stdio servers)
    pub env: Option<HashMap<String, String>>,
}

/// Import servers response
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportServersResponse {
    /// Number of servers imported
    pub imported_count: usize,
    /// List of imported server names
    pub imported_servers: Vec<String>,
    /// List of servers that failed to import
    pub failed_servers: Vec<String>,
}
