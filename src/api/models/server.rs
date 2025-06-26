// MCP Proxy API models for MCP server management
// Contains data models for MCP server endpoints

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::common::server::ServerType;

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
    /// Server ID (unique identifier)
    pub id: Option<String>,
    /// Server name
    pub name: String,
    /// Is enabled in configuration (combined global and suit status)
    pub enabled: bool,
    /// Is globally enabled (server_config.enabled)
    pub globally_enabled: bool,
    /// Is enabled in any active config suit (config_suit_server.enabled)
    pub enabled_in_suits: bool,
    /// Server type (stdio, sse, streamable_http)
    pub server_type: ServerType,
    /// Command to execute (for stdio servers)
    pub command: Option<String>,
    /// URL (for sse and streamable_http servers)
    pub url: Option<String>,
    /// Arguments to pass to the command (for stdio servers)
    pub args: Option<Vec<String>>,
    /// Environment variables to set (for stdio servers)
    pub env: Option<HashMap<String, String>>,
    /// Server metadata
    pub meta: Option<ServerMetaInfo>,
    /// When the configuration was created
    pub created_at: Option<String>,
    /// When the configuration was last updated
    pub updated_at: Option<String>,
    /// Summary of instances
    pub instances: Vec<ServerInstanceSummary>,
}

/// Server Metadata Information
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerMetaInfo {
    /// Description of the server
    pub description: Option<String>,
    /// Author of the server
    pub author: Option<String>,
    /// Website of the server
    pub website: Option<String>,
    /// Repository URL of the server
    pub repository: Option<String>,
    /// Category of the server
    pub category: Option<String>,
    /// Recommended scenario for the server
    pub recommended_scenario: Option<String>,
    /// Rating of the server (1-5)
    pub rating: Option<i32>,
}

/// Server Instances Response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInstancesResponse {
    /// Server name
    pub name: String,
    /// List of instances
    pub instances: Vec<ServerInstanceSummary>,
}

/// Resource metrics for an instance
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// CPU usage percentage of the instance process
    pub cpu_usage: Option<f32>,
    /// Memory usage in bytes of the instance process
    pub memory_usage: Option<u64>,
    /// Process ID of the instance
    pub process_id: Option<u32>,
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
    pub server_type: ServerType,
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
    /// Resource usage metrics for the instance
    pub resource_metrics: Option<ResourceMetrics>,
    /// Stability score based on connection history (0.0-1.0)
    pub connection_stability: Option<f32>,
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
    pub server_type: ServerType,
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
    /// Detailed error information for failed servers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<HashMap<String, String>>,
}
