// MCP Proxy API models for MCP server management
// Contains data models for MCP server endpoints

use serde::{Deserialize, Serialize};

/// Server status response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatusResponse {
    /// Server name
    pub name: String,
    /// Server status
    pub status: String,
}

/// Server response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerResponse {
    /// Server name
    pub name: String,
    /// Server status
    pub status: String,
}

/// Server list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerListResponse {
    /// List of servers
    pub servers: Vec<ServerStatusResponse>,
}
