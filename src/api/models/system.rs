// MCP Proxy API models for system management
// Contains data models for system endpoints

use serde::{Deserialize, Serialize};

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

/// System status response
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    /// System version
    pub version: String,
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// Number of connected servers
    pub connected_servers: usize,
    /// Number of total instances
    pub total_instances: usize,
    /// Number of ready instances
    pub ready_instances: usize,
}

/// System metrics response
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetricsResponse {
    /// CPU usage percentage
    pub cpu_usage: f32,
    /// Memory usage in MB
    pub memory_usage_mb: f32,
    /// Number of requests processed
    pub requests_processed: u64,
    /// Average response time in ms
    pub avg_response_time_ms: f32,
}
