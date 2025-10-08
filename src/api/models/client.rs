use crate::common::ClientCategory;
use crate::macros::resp::api_resp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[schemars(description = "Configuration file container type")]
pub enum ClientConfigType {
    #[schemars(description = "Object map container (default)")]
    Standard,
    #[schemars(description = "Array container")]
    Array,
}

/// Database row structure for client table
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClientRow {
    pub id: String,
    pub identifier: String,
    pub display_name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub category: Option<String>,
    pub enabled: bool,
    pub detected: bool,
    pub last_detected: Option<chrono::DateTime<chrono::Utc>>,
    pub install_path: Option<String>,
    pub config_path: Option<String>,
    pub version: Option<String>,
    pub detection_method: Option<String>,
    pub config_mode: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ClientRow {
    /// Get the category as a ClientCategory enum
    pub fn get_category(&self) -> ClientCategory {
        self.category
            .as_ref()
            .and_then(|c| ClientCategory::parse(c))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Detailed information about a clientlication")]
pub struct ClientInfo {
    #[schemars(description = "Unique client identifier (e.g., 'cursor', 'windsurf')")]
    pub identifier: String,
    #[schemars(description = "Display name of the clientlication")]
    pub display_name: String,
    #[schemars(description = "URL to client logo image")]
    pub logo_url: Option<String>,
    #[schemars(description = "Type of clientlication")]
    pub category: ClientCategory,
    #[schemars(description = "Whether client is enabled in MCPMate")]
    pub enabled: bool,
    #[schemars(description = "Whether MCPMate manages this client")]
    pub managed: bool,
    #[schemars(description = "Whether client is installed and detected")]
    pub detected: bool,
    #[schemars(description = "Installation path of the clientlication")]
    pub install_path: Option<String>,
    #[schemars(description = "Path to client configuration file")]
    pub config_path: String,
    #[schemars(description = "Whether configuration file exists")]
    pub config_exists: bool,
    #[schemars(description = "Whether MCP servers are configured")]
    pub has_mcp_config: bool,
    #[schemars(description = "Supported MCP transport protocols")]
    pub supported_transports: Vec<String>,
    #[schemars(description = "Short description of the client application")]
    #[serde(default)]
    pub description: Option<String>,
    #[schemars(description = "Homepage URL for the client application")]
    #[serde(default)]
    pub homepage_url: Option<String>,
    #[schemars(description = "Documentation URL for the client application")]
    #[serde(default)]
    pub docs_url: Option<String>,
    #[schemars(description = "Support or community URL for the client application")]
    #[serde(default)]
    pub support_url: Option<String>,
    #[schemars(description = "Configuration management mode")]
    pub config_mode: Option<String>,
    #[schemars(description = "Format type of configuration file")]
    pub config_type: Option<ClientConfigType>,
    #[schemars(description = "ISO 8601 timestamp of last detection")]
    pub last_detected: Option<String>,
    #[schemars(description = "ISO 8601 timestamp of last config modification")]
    pub last_modified: Option<String>,
    #[schemars(description = "Count of configured MCP servers")]
    pub mcp_servers_count: Option<u32>,
    #[schemars(description = "Template metadata summary for this client")]
    pub template: ClientTemplateMetadata,
}

#[derive(Debug, Clone, Serialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Configuration mode - hosted or transparent")]
pub enum ClientConfigMode {
    #[default]
    #[schemars(description = "MCPMate manages all server configurations ")]
    Hosted,
    #[schemars(description = "Merge with existing client configuration")]
    Transparent,
}

impl<'de> serde::Deserialize<'de> for ClientConfigMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_ascii_lowercase().as_str() {
            "hosted" => Ok(ClientConfigMode::Hosted),
            "transparent" => Ok(ClientConfigMode::Transparent),
            other => Err(serde::de::Error::custom(format!(
                "invalid mode '{}', allowed: hosted|transparent (case-insensitive)",
                other
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
#[schemars(description = "Selected configuration source - profile, servers, or default")]
pub enum ClientConfigSelected {
    #[schemars(description = "Use a profile by ID")]
    Profile {
        #[schemars(description = "Profile identifier")]
        profile_id: String,
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
#[schemars(description = "Response containing detected clientlications")]
pub struct ClientCheckData {
    #[schemars(description = "Array of clientlications with their detection status")]
    pub client: Vec<ClientInfo>,
    #[schemars(description = "Total count of clientlications")]
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
    #[schemars(description = "Diff output format when previewing (json/json5/toml/yaml)")]
    #[serde(default)]
    pub diff_format: Option<String>,
    #[schemars(description = "Original content before applying (if available in preview)")]
    #[serde(default)]
    pub diff_before: Option<String>,
    #[schemars(description = "Content after applying (if available in preview)")]
    #[serde(default)]
    pub diff_after: Option<String>,

    #[schemars(description = "Whether the write was scheduled due to a temporary lock")]
    #[serde(default)]
    pub scheduled: Option<bool>,
    #[schemars(description = "Reason for scheduling (e.g., 'db_locked')")]
    #[serde(default)]
    pub scheduled_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client management state response")]
pub struct ClientManageData {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Whether MCPMate manages this client")]
    pub managed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Single backup entry for a client configuration")]
pub struct ClientBackupEntry {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Backup file name")]
    pub backup: String,
    #[schemars(description = "Full backup file path")]
    pub path: String,
    #[schemars(description = "Backup file size in bytes")]
    pub size: u64,
    #[schemars(description = "ISO 8601 timestamp of backup creation")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Backup list payload")]
pub struct ClientBackupListData {
    #[schemars(description = "Collection of backups across clients")]
    pub backups: Vec<ClientBackupEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Backup mutation response (restore/delete)")]
pub struct ClientBackupActionData {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Backup file name")]
    pub backup: String,
    #[schemars(description = "Result message")]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Backup policy response body")]
pub struct ClientBackupPolicyData {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Backup policy label (system default: keep_n)")]
    pub policy: String,
    #[schemars(description = "Optional limit for keep_n policy (system default: 30)")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Configuration view response")]
pub struct ClientConfigData {
    #[schemars(description = "Path to configuration file")]
    pub config_path: String,
    #[schemars(description = "Whether configuration file exists")]
    pub config_exists: bool,
    #[schemars(description = "Warning messages related to reading configuration")]
    #[serde(default)]
    pub warnings: Vec<String>,
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
    #[schemars(description = "Import attempt summary (counts and errors)")]
    #[serde(default)]
    pub import_summary: Option<ClientImportSummary>,
    #[schemars(description = "Template metadata summary for this client")]
    pub template: ClientTemplateMetadata,
    #[schemars(description = "Supported transports derived from the template")]
    pub supported_transports: Vec<String>,
    #[schemars(description = "Whether MCPMate manages this client")]
    pub managed: bool,
    #[schemars(description = "Short description of the client application")]
    #[serde(default)]
    pub description: Option<String>,
    #[schemars(description = "Homepage URL for the client application")]
    #[serde(default)]
    pub homepage_url: Option<String>,
    #[schemars(description = "Documentation URL for the client application")]
    #[serde(default)]
    pub docs_url: Option<String>,
    #[schemars(description = "Support or community URL for the client application")]
    #[serde(default)]
    pub support_url: Option<String>,
    #[schemars(description = "Logo URL for the client application")]
    #[serde(default)]
    pub logo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[schemars(description = "Summary for servers imported from a client config")]
pub struct ClientImportSummary {
    #[schemars(description = "Whether import was attempted for this request")]
    pub attempted: bool,
    #[schemars(description = "Number of servers successfully imported")]
    pub imported_count: u32,
    #[schemars(description = "Number of servers skipped (e.g., duplicates)")]
    pub skipped_count: u32,
    #[schemars(description = "Number of servers failed to import")]
    pub failed_count: u32,
    #[schemars(description = "Optional per-server error messages for failures")]
    #[serde(default)]
    pub errors: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Import result for client configuration import")]
pub struct ClientConfigImportData {
    #[schemars(description = "Summary for the import attempt")]
    pub summary: ClientImportSummary,
    #[schemars(description = "Imported servers (when applied)")]
    #[serde(default)]
    pub imported_servers: Vec<ClientImportedServer>,
    #[schemars(description = "Profile id used for association (when applied)")]
    #[serde(default)]
    pub profile_id: Option<String>,
    #[schemars(description = "Whether capability sync was scheduled in background")]
    #[serde(default)]
    pub scheduled: Option<bool>,
    #[schemars(description = "Reason for scheduling (if available)")]
    #[serde(default)]
    pub scheduled_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Summary of the client template metadata")]
pub struct ClientTemplateMetadata {
    #[schemars(description = "Template output format (json/json5/toml/yaml)")]
    pub format: String,
    #[schemars(description = "Declared MCP protocol revision for the template")]
    pub protocol_revision: Option<String>,
    #[schemars(description = "Storage backend metadata")]
    pub storage: ClientTemplateStorageMetadata,
    #[schemars(description = "Container type resolved from the template")]
    pub container_type: ClientConfigType,
    #[schemars(description = "Merge strategy applied when writing configuration")]
    pub merge_strategy: String,
    #[schemars(description = "Whether original configuration segments are preserved")]
    pub keep_original_config: bool,
    #[schemars(description = "Managed config source (e.g., 'profile') if declared")]
    pub managed_source: Option<String>,
    #[schemars(description = "Short description of the client template")]
    #[serde(default)]
    pub description: Option<String>,
    #[schemars(description = "Homepage URL linked with the client template")]
    #[serde(default)]
    pub homepage_url: Option<String>,
    #[schemars(description = "Documentation URL linked with the client template")]
    #[serde(default)]
    pub docs_url: Option<String>,
    #[schemars(description = "Support or community URL linked with the client template")]
    #[serde(default)]
    pub support_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Storage metadata for a client template")]
pub struct ClientTemplateStorageMetadata {
    #[schemars(description = "Storage adapter kind (e.g. file/kv/custom)")]
    pub kind: String,
    #[schemars(description = "Optional path resolution strategy")]
    pub path_strategy: Option<String>,
}

// Note: former `ClientManagedEndpointMetadata` was removed; use `ClientTemplateMetadata.managed_source` instead.

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
    #[schemars(description = "Server type (stdio|sse|streamable_http)")]
    pub server_type: String,
    #[schemars(description = "Endpoint URL for HTTP/SSE servers")]
    #[serde(default)]
    pub url: Option<String>,
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
    ClientCheckResp,
    ClientCheckData,
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
api_resp!(
    ClientConfigImportResp,
    ClientConfigImportData,
    "Client configuration import response"
);
api_resp!(ClientManageResp, ClientManageData, "Client management toggle response");
api_resp!(
    ClientBackupListResp,
    ClientBackupListData,
    "Client configuration backup list response"
);
api_resp!(
    ClientBackupActionResp,
    ClientBackupActionData,
    "Client configuration backup mutation response"
);
api_resp!(
    ClientBackupPolicyResp,
    ClientBackupPolicyData,
    "Client configuration backup policy response"
);

// REQUEST STRUCTURES
// ==========================================

/// Request for client list/check operation
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClientCheckReq {
    #[serde(default)]
    #[schemars(description = "Whether to refresh the client list")]
    pub refresh: bool,
}

/// Request for client config details
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClientConfigReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
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

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Client management request payload")]
pub struct ClientManageReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Management action: enable or disable")]
    pub action: ClientManageAction,
}

#[derive(Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(description = "Management action enum")]
pub enum ClientManageAction {
    Enable,
    Disable,
}

impl<'de> serde::Deserialize<'de> for ClientManageAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_ascii_lowercase().as_str() {
            "enable" => Ok(ClientManageAction::Enable),
            "disable" => Ok(ClientManageAction::Disable),
            other => Err(serde::de::Error::custom(format!(
                "invalid action '{}', allowed: enable|disable (case-insensitive)",
                other
            ))),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
#[schemars(description = "Backup listing query")]
pub struct ClientBackupListReq {
    #[schemars(description = "Optional client identifier filter")]
    pub identifier: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Backup restore/delete request payload")]
pub struct ClientBackupOperateReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Backup file name")]
    pub backup: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Configuration restore request payload")]
pub struct ClientConfigRestoreReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Backup file name")]
    pub backup: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Backup policy query payload")]
pub struct ClientBackupPolicyReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Backup policy update payload")]
pub struct ClientBackupPolicySetReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Backup policy descriptor")]
    pub policy: ClientBackupPolicyPayload,
}

/// Request for client config import
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for client config import")]
pub struct ClientConfigImportReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[serde(default = "super::default_true")]
    #[schemars(description = "Preview only without applying changes (default: true)")]
    pub preview: bool,
    #[serde(default)]
    #[schemars(description = "Target profile id; default profile if omitted")]
    pub profile_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Default)]
#[schemars(description = "Backup policy descriptor for API payload")]
pub struct ClientBackupPolicyPayload {
    #[schemars(description = "Policy name: keep_last, keep_n, off")]
    pub policy: String,
    #[schemars(description = "Optional limit for keep_n policy (recommended: 30; new clients default to 30)")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, JsonSchema)]
#[schemars(description = "Detection results for a clientlication")]
pub struct ClientDetectedApp {
    #[schemars(description = "Installation path of the clientlication")]
    pub install_path: std::path::PathBuf,
    #[schemars(description = "Path to client configuration file")]
    pub config_path: std::path::PathBuf,
}
