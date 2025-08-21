// MCP Proxy API models for system management
// Contains data models for system endpoints

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "System status response")]
pub struct StatusResponse {
    #[schemars(description = "System status - running, starting, stopping, etc.")]
    pub status: String,
    #[schemars(description = "System uptime in seconds")]
    pub uptime: u64,
    #[schemars(description = "Total number of servers")]
    pub total_servers: usize,
    #[schemars(description = "Number of connected servers")]
    pub connected_servers: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "System status response")]
pub struct SystemStatusResponse {
    #[schemars(description = "System version")]
    pub version: String,
    #[schemars(description = "System uptime in seconds")]
    pub uptime_seconds: u64,
    #[schemars(description = "Number of connected servers")]
    pub connected_servers: usize,
    #[schemars(description = "Number of total instances")]
    pub total_instances: usize,
    #[schemars(description = "Number of ready instances")]
    pub ready_instances: usize,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "System metrics response")]
pub struct SystemMetricsResponse {
    #[schemars(description = "System uptime in seconds")]
    pub uptime_seconds: u64,
    #[schemars(description = "Current timestamp in ISO 8601 format")]
    pub timestamp: String,
    #[schemars(description = "Number of connected servers")]
    pub connected_servers_count: usize,
    #[schemars(description = "Number of total server instances")]
    pub total_instances_count: usize,
    #[schemars(description = "Number of ready instances")]
    pub ready_instances_count: usize,
    #[schemars(description = "Number of error instances")]
    pub error_instances_count: usize,
    #[schemars(description = "Number of initializing instances")]
    pub initializing_instances_count: usize,
    #[schemars(description = "Number of busy instances")]
    pub busy_instances_count: usize,
    #[schemars(description = "Number of shutdown instances")]
    pub shutdown_instances_count: usize,
    #[schemars(description = "Total number of tools available")]
    pub total_tools_count: usize,
    #[schemars(description = "Number of unique tools available")]
    pub unique_tools_count: usize,
    #[schemars(description = "CPU usage percentage of the proxy process")]
    pub cpu_usage: Option<f32>,
    #[schemars(description = "Memory usage in bytes of the proxy process")]
    pub memory_usage: Option<u64>,
    #[schemars(description = "Overall system CPU usage percentage")]
    pub system_cpu_usage: Option<f32>,
    #[schemars(description = "Overall system memory usage in bytes")]
    pub system_memory_usage: Option<u64>,
    #[schemars(description = "Total system memory in bytes")]
    pub system_memory_total: Option<u64>,
    #[schemars(description = "Configuration application status")]
    pub config_application_status: Option<ConfigApplicationStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(description = "Configuration application status")]
pub struct ConfigApplicationStatus {
    #[schemars(description = "Whether a configuration application is currently in progress")]
    pub in_progress: bool,
    #[schemars(description = "Configuration suit ID being applied")]
    pub suit_id: Option<String>,
    #[schemars(description = "Current stage description")]
    pub current_stage: Option<String>,
    #[schemars(description = "Progress percentage (0-100)")]
    pub progress_percentage: Option<u8>,
    #[schemars(description = "Estimated remaining time in seconds")]
    pub estimated_remaining_seconds: Option<u32>,
    #[schemars(description = "Start time of the current application (ISO 8601 format)")]
    pub started_at: Option<String>,
    #[schemars(description = "Total number of servers being processed")]
    pub total_servers: Option<usize>,
    #[schemars(description = "Number of servers successfully started")]
    pub servers_started: Option<usize>,
    #[schemars(description = "Number of servers successfully stopped")]
    pub servers_stopped: Option<usize>,
    #[schemars(description = "Failed operations with error messages")]
    pub failed_operations: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(description = "Server connection status for detailed reporting")]
pub struct ServerConnectionStatus {
    #[schemars(description = "Server name")]
    pub server_name: String,
    #[schemars(description = "Connection status - connected, disconnected, connecting, error")]
    pub status: String,
    #[schemars(description = "Last connection attempt timestamp (ISO 8601 format)")]
    pub last_attempt: Option<String>,
    #[schemars(description = "Error message if connection failed")]
    pub error_message: Option<String>,
    #[schemars(description = "Number of tools available from this server")]
    pub tools_count: usize,
    #[schemars(description = "Whether this server is enabled in active configuration suits")]
    pub enabled_in_suits: bool,
}
