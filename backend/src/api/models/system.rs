// MCP Proxy API models for system management
// Contains data models for system endpoints

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::macros::resp::api_resp;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "System status response")]
pub struct SystemStatusResp {
    #[schemars(description = "System status - running, starting, stopping, etc.")]
    pub status: String,
    #[schemars(description = "System uptime in seconds")]
    pub uptime: u64,
    #[schemars(description = "Total number of servers")]
    pub total_servers: usize,
    #[schemars(description = "Number of connected servers")]
    pub connected_servers: usize,
}

/// Active REST and MCP listener ports (from runtime configuration).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Runtime API and MCP proxy ports")]
pub struct SystemPortsResp {
    #[schemars(description = "REST API port")]
    pub api_port: u16,
    #[schemars(description = "MCP proxy port")]
    pub mcp_port: u16,
    #[schemars(description = "Base URL for the REST API")]
    pub api_url: String,
    #[schemars(description = "MCP Streamable HTTP endpoint URL")]
    pub mcp_http_url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "System metrics response")]
pub struct SystemMetricsResp {
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
    #[schemars(description = "Profile ID being applied")]
    pub profile_id: Option<String>,
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
    #[schemars(description = "Whether this server is enabled in active profile")]
    pub enabled_in_profile: bool,
}

// Management responses (shutdown/restart) now co-located under system models
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Response for management control actions (shutdown/restart)")]
pub struct ManagementActionResp {
    /// Action status label (e.g., "shutting_down", "restarted")
    pub status: String,
    /// Optional human-readable message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// MCP port when (re)started
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_port: Option<u16>,
    /// Transport mode hint (e.g., "uni")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
}

impl ManagementActionResp {
    pub fn shutting_down() -> Self {
        Self {
            status: "shutting_down".into(),
            message: Some("Proxy transport cancelled and instances disconnecting".into()),
            mcp_port: None,
            transport: None,
        }
    }

    pub fn restarted(
        mcp_port: u16,
        transport: &str,
    ) -> Self {
        Self {
            status: "restarted".into(),
            message: Some("Proxy transport restarted".into()),
            mcp_port: Some(mcp_port),
            transport: Some(transport.to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "System default client management mode payload")]
pub struct SystemDefaultClientModeData {
    #[schemars(description = "Default mode for unrecognized or unconfigured clients")]
    pub default_config_mode: String,
}

api_resp!(
    SystemDefaultClientModeResp,
    SystemDefaultClientModeData,
    "System default client management mode response"
);

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request to update system default client management mode")]
pub struct SystemDefaultClientModeReq {
    #[schemars(description = "Default mode: unify|hosted|transparent")]
    pub default_config_mode: String,
}
