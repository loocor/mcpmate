// MCP Proxy API models for system management
// Contains data models for system endpoints

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// System status response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
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
    /// Configuration application status
    pub config_application_status: Option<ConfigApplicationStatus>,
}

/// Configuration application status
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ConfigApplicationStatus {
    /// Whether a configuration application is currently in progress
    pub in_progress: bool,
    /// Configuration suit ID being applied
    pub suit_id: Option<String>,
    /// Current stage description
    pub current_stage: Option<String>,
    /// Progress percentage (0-100)
    pub progress_percentage: Option<u8>,
    /// Estimated remaining time in seconds
    pub estimated_remaining_seconds: Option<u32>,
    /// Start time of the current application (ISO 8601 format)
    pub started_at: Option<String>,
    /// Total number of servers being processed
    pub total_servers: Option<usize>,
    /// Number of servers successfully started
    pub servers_started: Option<usize>,
    /// Number of servers successfully stopped
    pub servers_stopped: Option<usize>,
    /// Failed operations with error messages
    pub failed_operations: Option<HashMap<String, String>>,
}

/// Server connection status for detailed reporting
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct ServerConnectionStatus {
    /// Server name
    pub server_name: String,
    /// Connection status (connected, disconnected, connecting, error)
    pub status: String,
    /// Last connection attempt timestamp (ISO 8601 format)
    pub last_attempt: Option<String>,
    /// Error message if connection failed
    pub error_message: Option<String>,
    /// Number of tools available from this server
    pub tools_count: usize,
    /// Whether this server is enabled in active configuration suits
    pub enabled_in_suits: bool,
}
