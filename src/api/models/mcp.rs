// MCP Proxy API models for MCP server management
// Contains data models for MCP server endpoints

use serde::{Deserialize, Serialize};

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

/// Server response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerResponse {
    /// Server name
    pub name: String,
    /// Server status summary
    pub status: String,
    /// List of instances
    pub instances: Vec<InstanceStatus>,
}

/// Server list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerListResponse {
    /// List of servers
    pub servers: Vec<ServerStatusResponse>,
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

/// Server health response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerHealthResponse {
    /// Server name
    pub name: String,
    /// Is server healthy
    pub healthy: bool,
    /// Health check message
    pub message: String,
    /// Current status
    pub status: String,
    /// Last health check time (ISO 8601 format)
    pub checked_at: String,
    /// List of instances
    pub instances: Vec<InstanceStatus>,
}
