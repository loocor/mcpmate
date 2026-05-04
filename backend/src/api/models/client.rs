use crate::clients::models::{
    CapabilitySource, UnifyDirectCapabilityIds, UnifyDirectExposureConfig, UnifyDirectExposureDiagnostics,
    UnifyDirectExposureIntent,
};
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
    #[schemars(description = "Persisted fine-grained transport rules for this client")]
    #[serde(default)]
    pub transports: Option<std::collections::HashMap<String, ClientFormatRuleData>>,
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
    #[schemars(description = "Configuration management mode: unify, hosted, or transparent")]
    pub config_mode: Option<String>,
    #[schemars(description = "Preferred or resolved transport: auto|streamable_http|sse|stdio")]
    #[serde(default)]
    pub transport: Option<String>,
    #[schemars(description = "Detected client version string")]
    #[serde(default)]
    pub client_version: Option<String>,
    #[schemars(
        description = "Hosted-mode capability source for client-scoped runtime policy (activated, profiles, or custom)."
    )]
    #[serde(default)]
    pub capability_source: CapabilitySource,
    #[schemars(description = "Selected shared profile ids when using profiles mode")]
    #[serde(default)]
    pub selected_profile_ids: Vec<String>,
    #[schemars(description = "Client-private custom profile id when using custom mode")]
    #[serde(default)]
    pub custom_profile_id: Option<String>,
    #[schemars(description = "Whether the current custom capability profile is missing or no longer resolvable")]
    #[serde(default)]
    pub custom_profile_missing: bool,
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
    #[schemars(description = "Approval status of the client (pending/approved/suspended)")]
    #[serde(default)]
    pub approval_status: Option<String>,
    #[schemars(description = "External client attachment state (attached/detached/not_applicable)")]
    #[serde(default)]
    pub attachment_state: Option<String>,
    #[schemars(description = "Runtime governance kind (passive or active)")]
    #[serde(default)]
    pub governance_kind: Option<String>,
    #[schemars(description = "Runtime connection mode (local_config_detected, remote_http, or manual)")]
    #[serde(default)]
    pub connection_mode: Option<String>,
    #[schemars(description = "Whether current governance state is inherited from default policy")]
    #[serde(default)]
    pub governed_by_default_policy: bool,
    #[schemars(description = "Whether this client has a real writable local configuration target")]
    #[serde(default)]
    pub writable_config: bool,
    #[schemars(description = "Whether the client is pending approval")]
    #[serde(default)]
    pub pending_approval: bool,
    #[schemars(description = "Effective config file parsing rules currently used for this client")]
    #[serde(default)]
    pub config_file_parse_effective: Option<ClientConfigFileParseData>,
    #[schemars(description = "Client-specific config file parsing override, if any")]
    #[serde(default)]
    pub config_file_parse_override: Option<ClientConfigFileParseData>,
    #[schemars(description = "Whether this client currently falls back to the stored default parsing rules")]
    #[serde(default)]
    pub uses_template_parse_default: bool,
}

#[derive(Debug, Clone, Serialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Configuration mode - unify, hosted, or transparent")]
pub enum ClientConfigMode {
    #[schemars(description = "Session-scoped Unify Mode using only builtin MCP control-plane tools")]
    #[default]
    Unify,
    #[schemars(description = "MCPMate manages the client through a durable hosted endpoint")]
    Hosted,
    #[schemars(
        description = "Write selected servers into the existing client configuration without hosted runtime controls"
    )]
    Transparent,
}

impl<'de> serde::Deserialize<'de> for ClientConfigMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_ascii_lowercase().as_str() {
            "unify" => Ok(ClientConfigMode::Unify),
            "hosted" => Ok(ClientConfigMode::Hosted),
            "transparent" => Ok(ClientConfigMode::Transparent),
            other => Err(serde::de::Error::custom(format!(
                "invalid mode '{}', allowed: unify|hosted|transparent (case-insensitive)",
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
#[schemars(description = "Delete a client record")]
pub struct ClientDeleteReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Deleted client record summary")]
pub struct ClientDeleteData {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Whether the client record was deleted")]
    pub deleted: bool,
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
    #[schemars(description = "Optional limit for keep_n policy (system default: 5)")]
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
    #[schemars(description = "Structured degraded reasons when fallback paths were triggered")]
    #[serde(default)]
    pub degraded_reasons: Vec<String>,
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
    #[schemars(description = "Persisted fine-grained transport rules for this client")]
    #[serde(default)]
    pub transports: Option<std::collections::HashMap<String, ClientFormatRuleData>>,
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
    #[schemars(description = "Capability source for client-scoped runtime policy")]
    #[serde(default)]
    pub capability_source: CapabilitySource,
    #[schemars(description = "Selected shared profile ids when using profiles mode")]
    #[serde(default)]
    pub selected_profile_ids: Vec<String>,
    #[schemars(description = "Client-private custom profile id when using custom mode")]
    #[serde(default)]
    pub custom_profile_id: Option<String>,
    #[schemars(description = "Whether the current custom capability profile is missing or no longer resolvable")]
    #[serde(default)]
    pub custom_profile_missing: bool,
    #[schemars(description = "Approval status of the client (pending/approved/suspended)")]
    #[serde(default)]
    pub approval_status: Option<String>,
    #[schemars(description = "External client attachment state (attached/detached/not_applicable)")]
    #[serde(default)]
    pub attachment_state: Option<String>,
    #[schemars(description = "Runtime governance kind (passive or active)")]
    #[serde(default)]
    pub governance_kind: Option<String>,
    #[schemars(description = "Runtime connection mode (local_config_detected, remote_http, or manual)")]
    #[serde(default)]
    pub connection_mode: Option<String>,
    #[schemars(description = "Whether current governance state is inherited from default policy")]
    #[serde(default)]
    pub governed_by_default_policy: bool,
    #[schemars(description = "Whether this client has a real writable local configuration target")]
    #[serde(default)]
    pub writable_config: bool,
    #[schemars(description = "Effective config file parsing rules currently used for this client")]
    #[serde(default)]
    pub config_file_parse_effective: Option<ClientConfigFileParseData>,
    #[schemars(description = "Client-specific config file parsing override, if any")]
    #[serde(default)]
    pub config_file_parse_override: Option<ClientConfigFileParseData>,
    #[schemars(description = "Whether this client currently falls back to the stored default parsing rules")]
    #[serde(default)]
    pub uses_template_parse_default: bool,
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
    #[schemars(description = "Detailed reason for each skipped server")]
    #[serde(default)]
    pub skipped_servers: Vec<crate::api::models::server::SkippedServerData>,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[schemars(description = "Structured parsing rules for reading MCP servers from a client config file")]
pub struct ClientConfigFileParseData {
    #[schemars(description = "Config file format (json/json5/toml/yaml)")]
    pub format: String,
    #[schemars(description = "Container type used to store MCP server entries")]
    pub container_type: ClientConfigType,
    #[schemars(description = "Ordered dot-paths where MCP server entries may be stored")]
    pub container_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq)]
#[schemars(description = "Fine-grained transport format rule data")]
pub struct ClientFormatRuleData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_field: Option<String>,
    #[serde(default)]
    pub include_type: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_fields: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
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
    #[schemars(
        description = "Server type reported by the client import (stdio|streamable_http, legacy sse may be normalized during import)"
    )]
    pub server_type: String,
    #[schemars(description = "Endpoint URL for HTTP-based servers, including legacy SSE-compatible endpoints")]
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
api_resp!(ClientDeleteResp, ClientDeleteData, "Client deletion response");

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client config file parse rule inspection request")]
pub struct ClientConfigFileParseInspectReq {
    #[schemars(description = "Absolute or user-relative path to the client config file")]
    pub config_path: String,
    #[schemars(description = "Optional parse rule draft to validate against the selected file")]
    #[serde(default)]
    pub config_file_parse: Option<ClientConfigFileParseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Stored-client config file parse rule inspection request")]
pub struct ClientConfigFileParseInspectExistingReq {
    #[schemars(description = "Client identifier whose stored config_path should be inspected")]
    pub identifier: String,
    #[schemars(description = "Optional parse rule draft to validate against the selected file")]
    #[serde(default)]
    pub config_file_parse: Option<ClientConfigFileParseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Validation summary for a client config file parse rule")]
pub struct ClientConfigFileParseValidationData {
    pub matches: bool,
    pub format_matches: bool,
    pub container_found: bool,
    pub server_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client config file parse inspection response body")]
pub struct ClientConfigFileParseInspectData {
    pub normalized_path: String,
    #[serde(default)]
    pub detected_format: Option<String>,
    #[serde(default)]
    pub inferred_parse: Option<ClientConfigFileParseData>,
    #[serde(default)]
    pub validation: Option<ClientConfigFileParseValidationData>,
    #[serde(default)]
    pub preview: serde_json::Value,
    #[serde(default)]
    pub preview_text: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

api_resp!(
    ClientConfigFileParseInspectResp,
    ClientConfigFileParseInspectData,
    "Client config file parse inspection response"
);
api_resp!(
    ClientConfigFileParseInspectExistingResp,
    ClientConfigFileParseInspectData,
    "Stored client config file parse inspection response"
);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client capability configuration payload")]
pub struct ClientCapabilityConfigData {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Capability source for client-scoped runtime policy")]
    pub capability_source: CapabilitySource,
    #[schemars(description = "Selected shared profile ids when using profiles mode")]
    #[serde(default)]
    pub selected_profile_ids: Vec<String>,
    #[schemars(description = "Client-private custom profile id when using custom mode")]
    #[serde(default)]
    pub custom_profile_id: Option<String>,
    #[schemars(description = "Whether the current custom capability profile is missing or no longer resolvable")]
    #[serde(default)]
    pub custom_profile_missing: bool,
    #[schemars(description = "Unify-only direct exposure state and diagnostics")]
    #[serde(default)]
    pub unify_direct_exposure: ClientUnifyDirectExposureData,
}
api_resp!(
    ClientCapabilityConfigResp,
    ClientCapabilityConfigData,
    "Client capability configuration response"
);

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Client settings update request (partial)")]
pub struct ClientSettingsUpdateReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Management mode: unify|hosted|transparent")]
    #[serde(default)]
    pub config_mode: Option<String>,
    #[schemars(
        description = "Transport protocol: auto|sse|stdio|streamable_http (sse remains for legacy client compatibility)"
    )]
    #[serde(default)]
    pub transport: Option<String>,
    #[schemars(description = "Client version string")]
    #[serde(default)]
    pub client_version: Option<String>,
    #[schemars(description = "Optional display name for active runtime client records")]
    #[serde(default)]
    pub display_name: Option<String>,
    #[schemars(description = "Runtime connection mode: local_config_detected|remote_http|manual")]
    #[serde(default)]
    pub connection_mode: Option<String>,
    #[schemars(description = "Runtime config path when the client uses a local config target")]
    #[serde(default)]
    pub config_path: Option<String>,
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
    #[schemars(description = "Client-specific config file parsing override")]
    #[serde(default)]
    pub config_file_parse: Option<ClientConfigFileParseData>,
    #[schemars(
        description = "Clear any stored config file parsing override and fall back to the stored default rules"
    )]
    #[serde(default)]
    pub clear_config_file_parse: bool,
    #[schemars(description = "Fine-grained transport rules to persist for runtime rendering")]
    #[serde(default)]
    pub transports: Option<std::collections::HashMap<String, ClientFormatRuleData>>,
    #[schemars(description = "Clear persisted fine-grained transport rules")]
    #[serde(default)]
    pub clear_transports: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client settings update response body")]
pub struct ClientSettingsUpdateData {
    pub identifier: String,
    pub display_name: String,
    #[serde(default)]
    pub config_mode: Option<String>,
    pub transport: String,
    #[serde(default)]
    pub client_version: Option<String>,
    #[serde(default)]
    pub connection_mode: Option<String>,
    #[serde(default)]
    pub config_path: Option<String>,
    #[serde(default)]
    pub transports: Option<std::collections::HashMap<String, ClientFormatRuleData>>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub homepage_url: Option<String>,
    #[serde(default)]
    pub docs_url: Option<String>,
    #[serde(default)]
    pub support_url: Option<String>,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub config_file_parse_effective: Option<ClientConfigFileParseData>,
    #[serde(default)]
    pub config_file_parse_override: Option<ClientConfigFileParseData>,
    #[serde(default)]
    pub uses_template_parse_default: bool,
    #[serde(default)]
    pub setting_sources: ClientSettingsSourceData,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
#[schemars(description = "Source markers for derived values in active client settings writes")]
pub struct ClientSettingsSourceData {
    #[schemars(description = "Source for display_name: provided|stored|default")]
    pub display_name: String,
    #[schemars(description = "Source for approval_status: provided|stored|default")]
    pub approval_status: String,
    #[schemars(description = "Source for connection_mode: provided|derived|stored")]
    pub connection_mode: String,
}

api_resp!(
    ClientSettingsUpdateResp,
    ClientSettingsUpdateData,
    "Client settings update response"
);

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Client capability configuration update request")]
pub struct ClientCapabilityConfigReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Capability source for client-scoped runtime policy")]
    pub capability_source: CapabilitySource,
    #[schemars(description = "Selected shared profile ids when using profiles mode")]
    #[serde(default)]
    pub selected_profile_ids: Vec<String>,
    #[schemars(description = "Optional Unify direct exposure state update")]
    #[serde(default)]
    pub unify_direct_exposure: Option<ClientUnifyDirectExposureReq>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(description = "Unify direct exposure state returned by client capability config APIs")]
pub struct ClientUnifyDirectExposureData {
    #[serde(flatten)]
    pub intent: UnifyDirectExposureIntent,
    #[serde(default)]
    pub diagnostics: UnifyDirectExposureDiagnostics,
    #[serde(default)]
    pub resolved_capabilities: UnifyDirectExposureConfig,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Optional Unify direct exposure state update")]
pub struct ClientUnifyDirectExposureReq {
    #[schemars(description = "Route mode for Unify direct exposure")]
    #[serde(default)]
    pub route_mode: crate::clients::models::UnifyRouteMode,
    #[schemars(description = "Selected eligible server ids for direct exposure")]
    #[serde(default)]
    pub server_ids: Vec<String>,
    #[schemars(description = "Selected direct capability ids for capability-level direct exposure")]
    #[serde(default)]
    pub capability_ids: UnifyDirectCapabilityIds,
}

impl From<ClientUnifyDirectExposureReq> for UnifyDirectExposureIntent {
    fn from(value: ClientUnifyDirectExposureReq) -> Self {
        Self {
            route_mode: value.route_mode,
            server_ids: value.server_ids,
            capability_ids: value.capability_ids,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Client approval request")]
pub struct ClientApproveReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Optional template ID to bind during approval")]
    #[serde(default)]
    pub template_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client approval response body")]
pub struct ClientApproveData {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "New approval status after operation")]
    pub approval_status: String,
}

api_resp!(ClientApproveResp, ClientApproveData, "Client approval response");

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Client suspend request")]
pub struct ClientSuspendReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client suspend response body")]
pub struct ClientSuspendData {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "New approval status after operation")]
    pub approval_status: String,
}

api_resp!(ClientSuspendResp, ClientSuspendData, "Client suspend response");

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
    #[schemars(description = "Configuration mode - unify, hosted, or transparent (default: unify)")]
    pub mode: ClientConfigMode,
    #[serde(default = "super::default_true")]
    #[schemars(description = "Whether to only preview changes without applying them (default: true)")]
    pub preview: bool,
    #[serde(default)]
    #[schemars(description = "Selected configuration source (default: default)")]
    pub selected_config: ClientConfigSelected,
    #[serde(default)]
    #[schemars(description = "Optional backup policy to persist before applying configuration")]
    pub backup_policy: Option<ClientBackupPolicyPayload>,
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
    #[schemars(description = "Optional limit for keep_n policy (recommended: 5; new clients default to 5)")]
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

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Client approval request")]
pub struct ApprovalRequest {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Client approval response")]
pub struct ApprovalResponse {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "Updated approval status")]
    pub status: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Detach MCPMate from an external client configuration")]
pub struct ClientDetachReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client detach response body")]
pub struct ClientDetachData {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "New attachment state after operation")]
    pub attachment_state: String,
    #[schemars(description = "Whether the external client config file was changed")]
    pub changed: bool,
}

api_resp!(ClientDetachResp, ClientDetachData, "Client detach response");

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Re-attach MCPMate to an external client configuration")]
pub struct ClientAttachReq {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Client attach response body")]
pub struct ClientAttachData {
    #[schemars(description = "Client identifier")]
    pub identifier: String,
    #[schemars(description = "New attachment state after operation")]
    pub attachment_state: String,
    #[schemars(description = "Whether the external client config file was updated")]
    pub changed: bool,
}

api_resp!(ClientAttachResp, ClientAttachData, "Client attach response");

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Onboarding policy update request")]
pub struct OnboardingPolicyRequest {
    #[schemars(description = "Policy: auto_manage, require_approval, or manual")]
    pub policy: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Onboarding policy response")]
pub struct OnboardingPolicyResponse {
    #[schemars(description = "Current onboarding policy")]
    pub policy: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "First contact behavior update request")]
pub struct FirstContactBehaviorRequest {
    #[schemars(
        description = "Behavior: deny, review, or allow (legacy pending_review / allow_then_review accepted as review)"
    )]
    pub behavior: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "First contact behavior payload")]
pub struct FirstContactBehaviorData {
    #[schemars(description = "Current first contact behavior")]
    pub behavior: String,
}

api_resp!(
    FirstContactBehaviorResp,
    FirstContactBehaviorData,
    "First contact behavior API response"
);

