use crate::common::ClientCategory;
use crate::config::client::models::ClientConfigType;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx;

// Import the unified response macro
use crate::macros::resp::api_resp;

/// Database row structure for client_apps table
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClientAppRow {
    pub id: String,
    pub identifier: String,
    pub display_name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub category: Option<String>,
    pub enabled: bool,
    pub detected: bool,
    pub last_detected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub install_path: Option<String>,
    pub config_path: Option<String>,
    pub version: Option<String>,
    pub detection_method: Option<String>,
    pub config_mode: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ClientAppRow {
    /// Get the category as a ClientCategory enum
    pub fn get_category(&self) -> ClientCategory {
        self.category
            .as_ref()
            .and_then(|c| ClientCategory::parse(c))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Detailed information about a client application")]
pub struct ClientInfo {
    #[schemars(description = "Unique client identifier (e.g., 'cursor', 'windsurf')")]
    pub identifier: String,
    #[schemars(description = "Display name of the client application")]
    pub display_name: String,
    #[schemars(description = "URL to client logo image")]
    pub logo_url: Option<String>,
    #[schemars(description = "Type of client application")]
    pub category: ClientCategory,
    #[schemars(description = "Whether client is enabled in MCPMate")]
    pub enabled: bool,
    #[schemars(description = "Whether client is installed and detected")]
    pub detected: bool,
    #[schemars(description = "Installation path of the client application")]
    pub install_path: Option<String>,
    #[schemars(description = "Path to client configuration file")]
    pub config_path: String,
    #[schemars(description = "Whether configuration file exists")]
    pub config_exists: bool,
    #[schemars(description = "Whether MCP servers are configured")]
    pub has_mcp_config: bool,
    #[schemars(description = "Supported MCP transport protocols")]
    pub supported_transports: Vec<String>,
    #[schemars(description = "Supported MCP runtime environments")]
    pub supported_runtimes: Vec<String>,
    #[schemars(description = "Configuration management mode")]
    pub config_mode: Option<String>,
    #[schemars(description = "Format type of configuration file")]
    pub config_type: Option<ClientConfigType>,
    #[schemars(description = "ISO 8601 timestamp of last detection")]
    pub last_detected_at: Option<String>,
    #[schemars(description = "ISO 8601 timestamp of last config modification")]
    pub last_modified: Option<String>,
    #[schemars(description = "Count of configured MCP servers")]
    pub mcp_servers_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Configuration mode - hosted or transparent")]
pub enum ClientConfigMode {
    #[default]
    #[schemars(description = "MCPMate manages all server configurations ")]
    Hosted,
    #[schemars(description = "Merge with existing client configuration")]
    Transparent,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
#[schemars(description = "Selected configuration source - suit, servers, or default")]
pub enum ClientConfigSelected {
    #[schemars(description = "Use a configuration suit by ID")]
    Suit {
        #[schemars(description = "Configuration suit identifier")]
        suit_id: String,
    },
    #[schemars(description = "Use specific servers by their IDs")]
    Servers {
        #[schemars(description = "List of server identifiers")]
        server_ids: Vec<String>,
    },
    #[default]
    #[schemars(description = "Use default configuration (all enabled servers)")]
    Default,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Response containing detected client applications")]
pub struct ClientsCheckData {
    #[schemars(description = "Array of client applications with their detection status")]
    pub clients: Vec<ClientInfo>,
    #[schemars(description = "Total count of client applications")]
    pub total: usize,
    #[schemars(description = "ISO 8601 timestamp of last update")]
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Configuration management response")]
pub struct ClientConfigUpdateData {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Preview of configuration changes")]
    pub preview: serde_json::Value,
    #[schemars(description = "Whether changes were actually applied")]
    pub applied: bool,
    #[schemars(description = "Path to backup file if created")]
    pub backup_path: Option<String>,
    #[schemars(description = "Warning messages from the operation")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Configuration view response")]
pub struct ClientConfigData {
    #[schemars(description = "Path to configuration file")]
    pub config_path: String,
    #[schemars(description = "Whether configuration file exists")]
    pub config_exists: bool,
    #[schemars(description = "Configuration file content")]
    pub content: serde_json::Value,
    #[schemars(description = "Whether MCP servers are configured")]
    pub has_mcp_config: bool,
    #[schemars(description = "Number of configured MCP servers")]
    pub mcp_servers_count: u32,
    #[schemars(description = "ISO 8601 timestamp of last modification")]
    pub last_modified: Option<String>,
    #[schemars(description = "Configuration file format type")]
    pub config_type: Option<ClientConfigType>,
    #[schemars(description = "List of imported server configurations")]
    pub imported_servers: Option<Vec<ClientImportedServer>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Information about an imported server")]
pub struct ClientImportedServer {
    #[schemars(description = "Server name identifier")]
    pub name: String,
    #[schemars(description = "Command to execute the server")]
    pub command: String,
    #[schemars(description = "Command line arguments")]
    pub args: Vec<String>,
    #[schemars(description = "Environment variables")]
    pub env: std::collections::HashMap<String, String>,
    #[schemars(description = "Transport protocol type")]
    pub transport_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "API error structure")]
pub struct ApiError {
    #[schemars(description = "Error code identifier")]
    pub code: String,
    #[schemars(description = "Human-readable error message")]
    pub message: String,
    #[schemars(description = "Additional error details")]
    pub details: Option<serde_json::Value>,
}

// ==========================================
// SPECIFIC API RESPONSE TYPES
// ==========================================

// Generate response structures using macro
api_resp!(
    ClientsCheckResp,
    ClientsCheckData,
    "Client applications detection response"
);
api_resp!(
    ClientConfigResp,
    ClientConfigData,
    "Client configuration details response"
);
api_resp!(
    ClientConfigUpdateResp,
    ClientConfigUpdateData,
    "Client configuration update response"
);

// REQUEST STRUCTURES
// ==========================================

/// Request for client list/check operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClientsCheckReq {
    #[serde(default)]
    #[schemars(description = "Whether to refresh the client list")]
    pub refresh: bool,
}

/// Request for client config details
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClientConfigReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[serde(default)]
    #[schemars(description = "Whether to import servers from the configuration")]
    pub import: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for client config update")]
pub struct ClientConfigUpdateReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[serde(default)]
    #[schemars(description = "Configuration mode - hosted or transparent (default: hosted)")]
    pub mode: ClientConfigMode,
    #[serde(default = "super::default_true")]
    #[schemars(description = "Whether to only preview changes without applying them (default: true)")]
    pub preview: bool,
    #[serde(default)]
    #[schemars(description = "Selected configuration source (default: default)")]
    pub selected_config: ClientConfigSelected,
}

#[derive(Debug, Clone, JsonSchema)]
#[schemars(description = "Detection results for a client application")]
pub struct ClientDetectedApp {
    #[schemars(description = "Installation path of the client application")]
    pub install_path: std::path::PathBuf,
    #[schemars(description = "Path to client configuration file")]
    pub config_path: std::path::PathBuf,
}
