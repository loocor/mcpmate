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
    /// System uptime in seconds
    pub uptime_seconds: u64,
    /// Current timestamp in ISO 8601 format
    pub timestamp: String,
    /// Number of connected servers
    pub connected_servers_count: usize,
    /// Number of total server instances
    pub total_instances_count: usize,
    /// Number of ready instances
    pub ready_instances_count: usize,
    /// Number of error instances
    pub error_instances_count: usize,
    /// Number of initializing instances
    pub initializing_instances_count: usize,
    /// Number of busy instances
    pub busy_instances_count: usize,
    /// Number of shutdown instances
    pub shutdown_instances_count: usize,
    /// Total number of tools available
    pub total_tools_count: usize,
    /// Number of unique tools available
    pub unique_tools_count: usize,
    /// CPU usage percentage of the proxy process
    pub cpu_usage: Option<f32>,
    /// Memory usage in bytes of the proxy process
    pub memory_usage: Option<u64>,
    /// Overall system CPU usage percentage
    pub system_cpu_usage: Option<f32>,
    /// Overall system memory usage in bytes
    pub system_memory_usage: Option<u64>,
    /// Total system memory in bytes
    pub system_memory_total: Option<u64>,
}
