// MCP Proxy API models for MCP server management
// Contains data models for MCP server endpoints

use crate::common::server::ServerType;
use crate::macros::resp::api_resp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// API Request Models
//

// ==========================================
// COMMON REQUEST STRUCTURES
// ==========================================

/// Generic request with server ID
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request with server ID")]
pub struct ServerIdReq {
    #[schemars(description = "Server ID")]
    pub id: String,
}

/// Generic request with server and instance IDs
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request with server and instance IDs")]
pub struct ServerInstanceReq {
    #[schemars(description = "Server ID")]
    pub server: String,
    #[schemars(description = "Instance ID")]
    pub instance: String,
}

// ==========================================
// SPECIFIC REQUEST STRUCTURES
// ==========================================

/// Request for server list operation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for server list operation")]
pub struct ServerListReq {
    #[serde(default)]
    #[schemars(description = "Filter by enabled status")]
    pub enabled: Option<bool>,

    #[serde(default)]
    #[schemars(description = "Filter by server type: stdio|sse|streamable_http")]
    pub server_type: Option<String>,

    #[serde(default)]
    #[schemars(description = "Page limit for pagination")]
    pub limit: Option<u32>,

    #[serde(default)]
    #[schemars(description = "Page offset for pagination")]
    pub offset: Option<u32>,
}

/// Request for server details operation
pub type ServerDetailsReq = ServerIdReq;

/// Request for server deletion
pub type ServerDeleteReq = ServerIdReq;

/// Request for server management operations
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for server management operations")]
pub struct ServerManageReq {
    #[schemars(description = "Server ID")]
    pub id: String,

    #[schemars(description = "Server management action: enable|disable")]
    pub action: ServerManageAction,

    #[serde(default)]
    #[schemars(description = "Whether to sync client configuration")]
    pub sync: bool,
}

/// Server management action enum
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Server management action enum")]
pub enum ServerManageAction {
    #[schemars(description = "Enable the server")]
    Enable,
    #[schemars(description = "Disable the server")]
    Disable,
}

impl<'de> serde::Deserialize<'de> for ServerManageAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_ascii_lowercase().as_str() {
            "enable" => Ok(ServerManageAction::Enable),
            "disable" => Ok(ServerManageAction::Disable),
            other => Err(serde::de::Error::custom(format!(
                "invalid server action '{}', allowed: enable|disable (case-insensitive)",
                other
            ))),
        }
    }
}

/// Request for server capability inspection
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for server capability inspection")]
#[derive(Default)]
pub struct ServerCapabilityReq {
    #[schemars(description = "Server ID")]
    pub id: String,

    #[serde(default)]
    #[schemars(description = "Refresh strategy: auto|force|cache")]
    pub refresh: Option<ServerRefreshStrategy>,
}

/// Server refresh strategy enum
#[derive(Debug, Serialize, JsonSchema, Clone, Copy)]
#[schemars(description = "Server refresh strategy enum")]
pub enum ServerRefreshStrategy {
    #[schemars(description = "Auto refresh based on cache policy")]
    Auto,
    #[schemars(description = "Force refresh from server")]
    Force,
    #[schemars(description = "Use cached data only")]
    Cache,
}

impl<'de> serde::Deserialize<'de> for ServerRefreshStrategy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(ServerRefreshStrategy::Auto),
            "force" => Ok(ServerRefreshStrategy::Force),
            "cache" => Ok(ServerRefreshStrategy::Cache),
            other => Err(serde::de::Error::custom(format!(
                "invalid refresh strategy '{}', allowed: auto|force|cache (case-insensitive)",
                other
            ))),
        }
    }
}

/// Request for instance list operation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for instance list operation")]
pub struct InstanceListReq {
    #[serde(default)]
    #[schemars(description = "Server ID (optional, lists all if not provided)")]
    pub id: Option<String>,
}

/// Request for instance details operation
pub type InstanceDetailsReq = ServerInstanceReq;

/// Request for instance health check operation
pub type InstanceHealthReq = ServerInstanceReq;

/// Request for instance management operations
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for instance management operations")]
pub struct InstanceManageReq {
    #[schemars(description = "Server ID")]
    pub server: String,

    #[schemars(description = "Instance ID")]
    pub instance: String,

    #[schemars(description = "Management action")]
    pub action: InstanceAction,
}

/// Server instance management action enum
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Server instance management action enum")]
pub enum InstanceAction {
    #[schemars(description = "Disconnect normally")]
    Disconnect,
    #[schemars(description = "Force disconnect")]
    ForceDisconnect,
    #[schemars(description = "Reconnect")]
    Reconnect,
    #[schemars(description = "Reset and reconnect")]
    ResetReconnect,
    #[schemars(description = "Recover disabled instance")]
    Recover,
    #[schemars(description = "Cancel initializing instance")]
    Cancel,
}

impl<'de> serde::Deserialize<'de> for InstanceAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_ascii_lowercase().as_str() {
            "disconnect" => Ok(InstanceAction::Disconnect),
            "forcedisconnect" | "force_disconnect" | "force-disconnect" => Ok(InstanceAction::ForceDisconnect),
            "reconnect" => Ok(InstanceAction::Reconnect),
            "resetreconnect" | "reset_reconnect" | "reset-reconnect" => Ok(InstanceAction::ResetReconnect),
            "recover" => Ok(InstanceAction::Recover),
            "cancel" => Ok(InstanceAction::Cancel),
            other => Err(serde::de::Error::custom(format!(
                "invalid instance action '{}', allowed: disconnect|force_disconnect|reconnect|reset_reconnect|recover|cancel (case-insensitive)",
                other
            ))),
        }
    }
}

// API Model
//

/// Instance status information
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Instance status information")]
pub struct ServerInstanceStatus {
    /// Instance ID
    pub id: String,
    /// Instance status
    pub status: String,
}

/// Server status response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server status response")]
pub struct ServerStatusResp {
    /// Server name
    pub name: String,
    /// Server status summary
    pub status: String,
    /// List of instances
    pub instances: Vec<ServerInstanceStatus>,
}

/// Instance Summary
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Instance Summary")]
pub struct InstanceSummary {
    /// Instance ID
    pub id: String,
    /// Instance status
    pub status: String,
    /// Started at time (ISO 8601 format)
    pub started_at: Option<String>,
    /// Connected at time (ISO 8601 format)
    pub connected_at: Option<String>,
}

/// Server details response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server details response")]
pub struct ServerDetailsData {
    /// Server ID (unique identifier)
    pub id: Option<String>,
    /// Server name
    pub name: String,
    /// Is enabled in configuration (combined global and profile status)
    pub enabled: bool,
    /// Is globally enabled (server_config.enabled)
    pub globally_enabled: bool,
    /// Is enabled in any active profile (profile_server.enabled)
    pub enabled_in_profile: bool,
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
    pub instances: Vec<InstanceSummary>,
}

/// Server Metadata Information
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server metadata information")]
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

/// Server instance list response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server instance list response")]
pub struct InstanceListData {
    /// Server name
    pub name: String,
    /// List of instances
    pub instances: Vec<InstanceSummary>,
}

/// Server resource metrics for an instance
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server resource metrics for an instance")]
pub struct ServerResourceMetrics {
    /// CPU usage percentage of the instance process
    pub cpu_usage: Option<f32>,
    /// Memory usage in bytes of the instance process
    pub memory_usage: Option<u64>,
    /// Process ID of the instance
    pub process_id: Option<u32>,
}

/// Server Instance Details
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server instance details")]
pub struct InstanceDetails {
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

/// Server instance details response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server instance details response")]
pub struct InstanceDetailsData {
    /// Instance ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Instance status
    pub status: String,
    /// Allowed operations
    pub allowed_operations: Vec<String>,
    /// Instance details
    pub details: InstanceDetails,
}

/// Server instance health response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server instance health response")]
pub struct InstanceHealthData {
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
    pub resource_metrics: Option<ServerResourceMetrics>,
    /// Stability score based on connection history (0.0-1.0)
    pub connection_stability: Option<f32>,
}

/// Server tools response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server tools response")]
pub struct ServerToolsData {
    /// List of tools
    pub items: Vec<serde_json::Value>,
    /// Response state
    pub state: String,
    /// Metadata about the response
    pub meta: ServerCapabilityMeta,
}

/// Server resources response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server resources data")]
pub struct ServerResourcesData {
    /// List of resources
    pub items: Vec<serde_json::Value>,
    /// Response state
    pub state: String,
    /// Metadata about the response
    pub meta: ServerCapabilityMeta,
}

/// Server resource templates response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server resource templates response data")]
pub struct ServerResourceTemplatesData {
    /// List of resource templates
    pub items: Vec<serde_json::Value>,
    /// Response state
    pub state: String,
    /// Metadata about the response
    pub meta: ServerCapabilityMeta,
}

/// Server prompts response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server prompts response")]
pub struct ServerPromptsData {
    /// List of prompts
    pub items: Vec<serde_json::Value>,
    /// Response state
    pub state: String,
    /// Metadata about the response
    pub meta: ServerCapabilityMeta,
}

/// Server prompt arguments response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server prompt arguments response")]
pub struct ServerPromptArgumentsData {
    /// List of prompt arguments
    pub items: Vec<serde_json::Value>,
    /// Metadata about the response
    pub meta: ServerCapabilityMeta,
}

/// Server capability response metadata
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[schemars(description = "Server capability response metadata")]
pub struct ServerCapabilityMeta {
    /// Whether data came from cache
    pub cache_hit: bool,
    /// Refresh strategy used
    pub strategy: String,
    /// Data source
    pub source: String,
}

/// Server operation request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server operation request")]
pub struct ServerOperationReq {
    /// Force the operation (optional, for disconnect)
    pub force: Option<bool>,
    /// Reset the connection (optional, for reconnect)
    pub reset: Option<bool>,
}

/// Server operation response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server operation response")]
pub struct ServerOperationData {
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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server list response")]
pub struct ServerListData {
    /// List of servers
    pub servers: Vec<ServerDetailsData>,
}

/// MCP Server Create Request
///
/// Request parameters for creating a new MCP server configuration. The server type must strictly use standard formats.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request parameters for creating a MCP server")]
pub struct ServerCreateReq {
    /// Server name
    ///
    /// Must be a unique identifier for identifying this server in the system
    #[schemars(description = "Server's unique name identifier")]
    pub name: String,

    /// Server type
    ///
    /// **Strict format requirements**: Only accepts the following three standard formats
    /// - `"stdio"`: Standard input/output server, started by command line
    /// - `"sse"`: Server-Sent Events server, connected by HTTP SSE
    /// - `"streamable_http"`: Streamable HTTP server, connected by HTTP stream
    ///
    /// **Note**: The system will reject any variant formats, such as "http", "streamableHttp", etc.
    #[schemars(description = "Server type, must be stdio, sse, or streamable_http")]
    #[schemars(regex(pattern = r"^(stdio|sse|streamable_http)$"))]
    pub server_type: String,

    /// Startup command (only used for stdio type)
    ///
    /// Required when the server type is "stdio", specify the command to start the server
    #[schemars(description = "Server startup command (required for stdio type)")]
    pub command: Option<String>,

    /// Server URL (only used for sse and streamable_http types)
    ///
    /// Required when the server type is "sse" or "streamable_http"
    #[schemars(description = "Server URL (required for sse and streamable_http types)")]
    pub url: Option<String>,

    /// Command arguments (only used for stdio type)
    #[schemars(description = "List of arguments passed to the command (optional for stdio type)")]
    pub args: Option<Vec<String>>,

    /// Environment variables (only used for stdio type)
    #[schemars(description = "Environment variables to set (optional for stdio type)")]
    pub env: Option<HashMap<String, String>>,

    /// Whether to enable the server in the default profile
    #[schemars(description = "Whether to enable this server in the default profile")]
    pub enabled: Option<bool>,
}

/// MCP Server Update Request
///
/// Request parameters for updating an existing MCP server configuration. If updating the server type, it must strictly use standard formats.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request parameters for updating MCP server")]
pub struct ServerUpdateReq {
    /// Server ID
    /// Unique identifier of the server to be updated
    #[schemars(description = "ID of the server to update")]
    pub id: String,

    /// Server type (optional update)
    ///
    /// **Strict format requirements**: If provided, only accepts the following three standard formats
    /// - `"stdio"`: Standard input/output server
    /// - `"sse"`: Server-Sent Events server
    /// - `"streamable_http"`: Streamable HTTP server
    ///
    /// **Important**: Any non-standard format will be rejected and return a 400 error
    #[schemars(description = "Server type, if provided must be stdio, sse, or streamable_http")]
    #[schemars(regex(pattern = r"^(stdio|sse|streamable_http)$"))]
    pub kind: Option<String>,

    /// Launch command (optional update)
    #[schemars(description = "Server launch command")]
    pub command: Option<String>,

    /// Server URL (optional update)
    #[schemars(description = "Server URL")]
    pub url: Option<String>,

    /// Command arguments (optional update)
    #[schemars(description = "List of arguments passed to the command")]
    pub args: Option<Vec<String>>,

    /// Environment variables (optional update)
    #[schemars(description = "Environment variables to set")]
    pub env: Option<HashMap<String, String>>,

    /// Whether to enable the server (optional update)
    #[schemars(description = "Whether to enable this server")]
    pub enabled: Option<bool>,
}

/// Import servers request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Import servers request")]
pub struct ServersImportReq {
    /// Map of MCP server name to configuration
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, ServersImportConfig>,
}

/// Import server configuration
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Import server configuration")]
pub struct ServersImportConfig {
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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Import servers response")]
pub struct ServersImportData {
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

// ==========================================
// SPECIFIC API RESPONSE TYPES
// ==========================================

// Generate response structures using macro
api_resp!(ServerDetailsResp, ServerDetailsData, "Server details API response");
api_resp!(ServerListResp, ServerListData, "Server list API response");
api_resp!(InstanceListResp, InstanceListData, "Instance list API response");
api_resp!(
    InstanceDetailsResp,
    InstanceDetailsData,
    "Instance details API response"
);
api_resp!(InstanceHealthResp, InstanceHealthData, "Instance health API response");
api_resp!(ServerToolsResp, ServerToolsData, "Server tools API response");
api_resp!(
    ServerResourcesResp,
    ServerResourcesData,
    "Server resources API response"
);
api_resp!(ServerPromptsResp, ServerPromptsData, "Server prompts API response");
api_resp!(ServersImportResp, ServersImportData, "Import servers API response");
api_resp!(
    ServerOperationResp,
    ServerOperationData,
    "Operation result API response"
);
api_resp!(
    ServerResourceTemplatesResp,
    ServerResourceTemplatesData,
    "Server resource templates API response"
);
api_resp!(
    ServerPromptArgumentsResp,
    ServerPromptArgumentsData,
    "Server prompt arguments API response"
);

// ================= Preview Capabilities =================

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Single server preview item request")]
pub struct ServerPreviewItemReq {
    pub name: String,
    pub kind: String, // stdio|sse|streamable_http
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Preview capabilities request")]
pub struct ServerPreviewReq {
    pub servers: Vec<ServerPreviewItemReq>,
    #[serde(default)]
    pub include_details: Option<bool>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Preview item response with capabilities snapshot")]
pub struct ServerPreviewItemData {
    pub name: String,
    pub ok: bool,
    #[serde(default)]
    pub error: Option<String>,
    pub tools: ServerToolsData,
    pub resources: ServerResourcesData,
    pub resource_templates: ServerResourceTemplatesData,
    pub prompts: ServerPromptsData,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Preview capabilities response data")]
pub struct ServerPreviewData {
    pub items: Vec<ServerPreviewItemData>,
}

api_resp!(
    ServerPreviewResp,
    ServerPreviewData,
    "Server capability preview response"
);
