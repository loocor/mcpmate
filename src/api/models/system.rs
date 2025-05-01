// MCP Proxy API models for system management
// Contains data models for system endpoints

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// System status response
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    /// System status (running, starting, stopping, etc.)
    pub status: String,
    /// System uptime in seconds
    pub uptime: u64,
    /// Total number of servers
    pub total_servers: usize,
    /// Number of connected servers
    pub connected_servers: usize,
}

/// System metrics response
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    /// System uptime in seconds
    pub uptime: u64,
    /// Server statuses with instance details
    pub server_statuses: HashMap<String, Vec<(String, String)>>,
}
