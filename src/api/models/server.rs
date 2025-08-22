// MCP Proxy API models for MCP server management
// Contains data models for MCP server endpoints

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::common::server::ServerType;

// API Request Models
//

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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for server details operation")]
pub struct ServerDetailsReq {
    #[schemars(description = "Server ID")]
    pub id: String,
}

/// Request for server deletion
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for server deletion")]
pub struct ServerDeleteReq {
    #[schemars(description = "Server ID")]
    pub id: String,
}

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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server management action enum")]
pub enum ServerManageAction {
    #[schemars(description = "Enable the server")]
    Enable,
    #[schemars(description = "Disable the server")]
    Disable,
}

/// Request for server capability inspection
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for server capability inspection")]
pub struct ServerCapabilityReq {
    #[schemars(description = "Server ID")]
    pub id: String,

    #[serde(default)]
    #[schemars(description = "Refresh strategy: auto|force|cache")]
    pub refresh: Option<ServerRefreshStrategy>,
}

/// Server refresh strategy enum
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Server refresh strategy enum")]
pub enum ServerRefreshStrategy {
    #[schemars(description = "Auto refresh based on cache policy")]
    Auto,
    #[schemars(description = "Force refresh from server")]
    Force,
    #[schemars(description = "Use cached data only")]
    Cache,
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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for instance details operation")]
pub struct InstanceDetailsReq {
    #[schemars(description = "Server ID")]
    pub server: String,

    #[schemars(description = "Instance ID")]
    pub instance: String,
}

/// Request for instance health check operation
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Request for instance health check operation")]
pub struct InstanceHealthReq {
    #[schemars(description = "Server ID")]
    pub server: String,

    #[schemars(description = "Instance ID")]
    pub instance: String,
}

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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
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
    pub data: Vec<serde_json::Value>,
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
    pub data: Vec<serde_json::Value>,
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
    pub data: Vec<serde_json::Value>,
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
    pub data: Vec<serde_json::Value>,
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
    pub data: Vec<serde_json::Value>,
    /// Metadata about the response
    pub meta: ServerCapabilityMeta,
}

/// Server capability response metadata
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
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

/// Create server request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Create server request")]
pub struct ServerCreateReq {
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
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Update server request")]
pub struct ServerUpdateReq {
    /// Server ID
    pub id: String,
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

use crate::api::models::clients::ApiError;

/// Response for server details operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Server details API response")]
pub struct ServerDetailsResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ServerDetailsData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for server list operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Server list API response")]
pub struct ServerListResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ServerListData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for instance list operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Instance list API response")]
pub struct InstanceListResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<InstanceListData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for instance details operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Instance details API response")]
pub struct InstanceDetailsResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<InstanceDetailsData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for instance health operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Instance health API response")]
pub struct InstanceHealthResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<InstanceHealthData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for server tools operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Server tools API response")]
pub struct ServerToolsResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ServerToolsData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for server resources operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Server resources API response")]
pub struct ServerResourcesResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ServerResourcesData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for server prompts operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Server prompts API response")]
pub struct ServerPromptsResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ServerPromptsData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for import servers operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Import servers API response")]
pub struct ServersImportResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ServersImportData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for operation results
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Operation result API response")]
pub struct ServerOperationResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ServerOperationData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for server resource templates operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Server resource templates API response")]
pub struct ServerResourceTemplatesResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ServerResourceTemplatesData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for server prompt arguments operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Server prompt arguments API response")]
pub struct ServerPromptArgumentsResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ServerPromptArgumentsData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

// ==========================================
// RESPONSE IMPLEMENTATION METHODS
// ==========================================

impl ServerDetailsResp {
    pub fn success(data: ServerDetailsData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl ServerListResp {
    pub fn success(data: ServerListData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl InstanceListResp {
    pub fn success(data: InstanceListData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl InstanceDetailsResp {
    pub fn success(data: InstanceDetailsData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl InstanceHealthResp {
    pub fn success(data: InstanceHealthData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl ServerToolsResp {
    pub fn success(data: ServerToolsData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl ServerResourcesResp {
    pub fn success(data: ServerResourcesData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl ServerPromptsResp {
    pub fn success(data: ServerPromptsData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl ServersImportResp {
    pub fn success(data: ServersImportData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl ServerOperationResp {
    pub fn success(data: ServerOperationData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl ServerResourceTemplatesResp {
    pub fn success(data: ServerResourceTemplatesData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}

impl ServerPromptArgumentsResp {
    pub fn success(data: ServerPromptArgumentsData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(
        code: &str,
        message: &str,
    ) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
            }),
        }
    }
}
