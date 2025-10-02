// MCP Proxy API models for Profile management
// Contains data models for Profile endpoints

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Import the unified response macro
use crate::macros::resp::api_resp;

// ==========================================
// COMMON REQUEST STRUCTURES
// ==========================================

/// Generic request with profile ID
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request with profile ID")]
pub struct ProfileIdReq {
    #[schemars(description = "Profile ID")]
    pub id: String,
}

// ==========================================
// STANDARDIZED REQUEST/RESPONSE MODELS
// Following server module patterns with JsonSchema annotations
// ==========================================

// Action Enums
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Available profile management actions")]
pub enum ProfileAction {
    #[schemars(description = "Activate the profile")]
    Activate,
    #[schemars(description = "Deactivate the profile")]
    Deactivate,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Available component management actions")]
pub enum ProfileComponentAction {
    #[schemars(description = "Enable the component")]
    Enable,
    #[schemars(description = "Disable the component")]
    Disable,
    #[schemars(description = "Remove the component")]
    Remove,
}

// Query Request Models
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for profile list operation")]
pub struct ProfileListReq {
    #[serde(default)]
    #[schemars(description = "Filter by profile status: active, inactive, all")]
    pub filter_type: Option<String>,

    #[serde(default)]
    #[schemars(description = "Filter by profile type: host_app, scenario, shared")]
    pub profile_type: Option<String>,

    #[serde(default)]
    #[schemars(description = "Page limit for pagination (max 100)")]
    pub limit: Option<usize>,

    #[serde(default)]
    #[schemars(description = "Page offset for pagination")]
    pub offset: Option<usize>,
}

/// Request for profile details operation
pub type ProfileDetailsReq = ProfileIdReq;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for profile component list (servers, tools, etc.)")]
pub struct ProfileComponentListReq {
    #[schemars(description = "Profile identifier")]
    pub profile_id: String,

    #[serde(default)]
    #[schemars(description = "Show only enabled components")]
    pub enabled_only: Option<bool>,
}

// Payload Request Models
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for profile management operations")]
pub struct ProfileManageReq {
    #[schemars(description = "Profile identifiers (single or multiple)")]
    pub ids: Vec<String>,

    #[schemars(description = "Management action to perform")]
    pub action: ProfileAction,

    #[schemars(description = "Whether to trigger client configuration synchronization")]
    #[serde(default)]
    pub sync: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for component management operations (unified single and batch operations)")]
pub struct ProfileComponentManageReq {
    #[schemars(description = "Profile identifier")]
    pub profile_id: String,

    #[schemars(description = "Component identifiers (single element for individual operations, multiple for batch)")]
    pub component_ids: Vec<String>,

    #[schemars(description = "Management action to perform on component(s)")]
    pub action: ProfileComponentAction,
}

/// Request for profile deletion
pub type ProfileDeleteReq = ProfileIdReq;

// Response Models (with Resp suffix)
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for profile list operation")]
pub struct ProfileListData {
    #[schemars(description = "List of profile")]
    pub profile: Vec<ProfileData>,

    #[schemars(description = "Total number of profile matching filter")]
    pub total: usize,

    #[schemars(description = "ISO 8601 timestamp of response")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for profile details operation")]
pub struct ProfileDetailsData {
    #[schemars(description = "Profile details")]
    pub profile: ProfileData,

    #[schemars(description = "Number of enabled servers in profile")]
    pub servers_count: usize,

    #[schemars(description = "Number of enabled tools in profile")]
    pub tools_count: usize,

    #[schemars(description = "Number of enabled resources in profile")]
    pub resources_count: usize,

    #[schemars(description = "Number of enabled prompts in profile")]
    pub prompts_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Single profile operation result")]
pub struct ProfileOperationResult {
    #[schemars(description = "Profile identifier")]
    pub id: String,

    #[schemars(description = "Profile name")]
    pub name: String,

    #[schemars(description = "Operation result")]
    pub result: String,

    #[schemars(description = "Current profile status after operation")]
    pub status: String,

    #[schemars(description = "Error message if operation failed")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for profile management operations")]
pub struct ProfileManageData {
    #[schemars(description = "Number of successful operations")]
    pub success_count: usize,

    #[schemars(description = "Number of failed operations")]
    pub failed_count: usize,

    #[schemars(description = "List of operation results")]
    pub results: Vec<ProfileOperationResult>,

    #[schemars(description = "ISO 8601 timestamp of operation")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for profile servers list operation")]
pub struct ProfileServersListData {
    #[schemars(description = "Profile identifier")]
    pub profile_id: String,

    #[schemars(description = "Profile name")]
    pub profile_name: String,

    #[schemars(description = "List of servers in this profile")]
    pub servers: Vec<ProfileServerResp>,

    #[schemars(description = "Total number of servers in profile")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for profile tools list operation")]
pub struct ProfileToolsListData {
    #[schemars(description = "Profile identifier")]
    pub profile_id: String,

    #[schemars(description = "Profile name")]
    pub profile_name: String,

    #[schemars(description = "List of tools in this profile")]
    pub tools: Vec<ProfileToolData>,

    #[schemars(description = "Total number of tools in profile")]
    pub total: usize,
}

api_resp!(
    ProfileResourcesListResp,
    ProfileResourcesListData,
    "Response for profile resources list operation"
);

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Data for profile resources list operation")]
pub struct ProfileResourcesListData {
    #[schemars(description = "Profile identifier")]
    pub profile_id: String,

    #[schemars(description = "Profile name")]
    pub profile_name: String,

    #[schemars(description = "List of resources in this profile")]
    pub resources: Vec<ProfileResourceData>,

    #[schemars(description = "Total number of resources in profile")]
    pub total: usize,
}

api_resp!(
    ProfilePromptsListResp,
    ProfilePromptsListData,
    "Response for profile prompts list operation"
);

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Data for profile prompts list operation")]
pub struct ProfilePromptsListData {
    #[schemars(description = "Profile identifier")]
    pub profile_id: String,

    #[schemars(description = "Profile name")]
    pub profile_name: String,

    #[schemars(description = "List of prompts in this profile")]
    pub prompts: Vec<ProfilePromptData>,

    #[schemars(description = "Total number of prompts in profile")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for component management operations")]
pub struct ProfileServerManageData {
    #[schemars(description = "Profile identifier")]
    pub profile_id: String,

    #[schemars(description = "Operation results (single element for individual operations, multiple for batch)")]
    pub results: Vec<ComponentOperationResult>,

    #[schemars(description = "Operation summary")]
    pub summary: String,

    #[schemars(description = "Overall operation status")]
    pub status: String,

    #[schemars(description = "ISO 8601 timestamp of operation")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Individual component operation result")]
pub struct ComponentOperationResult {
    #[schemars(description = "Component identifier")]
    pub component_id: String,

    #[schemars(description = "Component type")]
    pub component_type: String,

    #[schemars(description = "Whether the operation succeeded")]
    pub success: bool,

    #[schemars(description = "Operation result message")]
    pub result: String,

    #[schemars(description = "Error message if operation failed")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ==========================================
// LEGACY MODELS (kept for backward compatibility)
// ==========================================

/// Profile response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProfileData {
    /// Unique ID
    pub id: String,
    /// Name of the profile
    pub name: String,
    /// Description of the profile
    pub description: Option<String>,
    /// Type of the profile (host_app, scenario, shared)
    pub profile_type: String,
    /// Role of the profile (user, default_anchor)
    pub role: String,
    /// Whether multiple profile can be selected simultaneously
    pub multi_select: bool,
    /// Priority of the profile (higher priority wins in case of conflicts)
    pub priority: i32,
    /// Whether the profile is currently active
    pub is_active: bool,
    /// Whether the profile is the default one
    pub is_default: bool,
    /// Allowed operations on this profile
    pub allowed_operations: Vec<String>,
}

/// Create Profile request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProfileCreateReq {
    /// Name of the profile
    pub name: String,
    /// Description of the profile
    pub description: Option<String>,
    /// Type of the profile (host_app, scenario, shared)
    pub profile_type: String,
    /// Whether multiple profile can be selected simultaneously
    pub multi_select: Option<bool>,
    /// Priority of the profile (higher priority wins in case of conflicts)
    pub priority: Option<i32>,
    /// Whether the profile is currently active
    pub is_active: Option<bool>,
    /// Whether the profile is the default one
    pub is_default: Option<bool>,
    /// Clone from existing profile (optional)
    pub clone_from_id: Option<String>,
}

/// Update Profile request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProfileUpdateReq {
    /// Profile ID to update
    pub id: String,
    /// Name of the profile
    pub name: Option<String>,
    /// Description of the profile
    pub description: Option<String>,
    /// Type of the profile (host_app, scenario, shared)
    pub profile_type: Option<String>,
    /// Whether multiple profile can be selected simultaneously
    pub multi_select: Option<bool>,
    /// Priority of the profile (higher priority wins in case of conflicts)
    pub priority: Option<i32>,
    /// Whether the profile is currently active
    pub is_active: Option<bool>,
    /// Whether the profile is the default one
    pub is_default: Option<bool>,
}

/// Operation response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Operation response details")]
pub struct ProfileOperationData {
    /// Unique ID
    #[schemars(description = "Unique identifier of the profile")]
    pub id: String,
    /// Name of the profile
    #[schemars(description = "Name of the profile")]
    pub name: String,
    /// Result of the operation
    #[schemars(description = "Result description of the operation")]
    pub result: String,
    /// Status after the operation
    #[schemars(description = "Current status after the operation")]
    pub status: String,
    /// Allowed operations after this operation
    #[schemars(description = "List of operations allowed on this profile")]
    pub allowed_operations: Vec<String>,
}

/// Profile server response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProfileServerResp {
    /// Server ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Whether the server is enabled in this profile
    pub enabled: bool,
    /// Allowed operations on this server
    pub allowed_operations: Vec<String>,
}

/// Profile tool response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProfileToolData {
    /// Tool ID
    pub id: String,
    /// Server ID
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Tool name (original name from upstream server)
    pub tool_name: String,
    /// Unique name for external display and routing
    pub unique_name: Option<String>,
    /// Whether the tool is enabled in this profile
    pub enabled: bool,
    /// Allowed operations on this tool
    pub allowed_operations: Vec<String>,
}

/// Profile resource response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProfileResourceData {
    /// Resource ID
    pub id: String,
    /// Server ID
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Resource URI (original URI from upstream server)
    pub resource_uri: String,
    /// Whether the resource is enabled in this profile
    pub enabled: bool,
    /// Allowed operations on this resource
    pub allowed_operations: Vec<String>,
}

/// Profile prompt response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ProfilePromptData {
    /// Prompt ID
    pub id: String,
    /// Server ID
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Prompt name (original name from upstream server)
    pub prompt_name: String,
    /// Whether the prompt is enabled in this profile
    pub enabled: bool,
    /// Allowed operations on this prompt
    pub allowed_operations: Vec<String>,
}

// ==========================================
// SPECIFIC API RESPONSE TYPES
// ==========================================

// Generate response structures using macro
api_resp!(ProfileListResp, ProfileListData, "Profile list API response");
api_resp!(ProfileDetailsResp, ProfileDetailsData, "Profile details API response");

api_resp!(ProfileManageResp, ProfileManageData, "Profile management API response");
api_resp!(ProfileResp, ProfileData, "Profile API response");
api_resp!(
    ProfileServersListResp,
    ProfileServersListData,
    "Profile servers list API response"
);
api_resp!(
    ProfileToolsListResp,
    ProfileToolsListData,
    "Profile tools list API response"
);
api_resp!(
    ProfileServerManageResp,
    ProfileServerManageData,
    "Profile component manage API response"
);
