// MCP Proxy API models for Config Suit management
// Contains data models for Config Suit endpoints

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ==========================================
// STANDARDIZED REQUEST/RESPONSE MODELS
// Following server module patterns with JsonSchema annotations
// ==========================================

// Action Enums
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Available suit management actions")]
pub enum SuitAction {
    #[schemars(description = "Activate the configuration suit")]
    Activate,
    #[schemars(description = "Deactivate the configuration suit")]
    Deactivate,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(description = "Available component management actions")]
pub enum SuitComponentAction {
    #[schemars(description = "Enable the component")]
    Enable,
    #[schemars(description = "Disable the component")]
    Disable,
}

// Query Request Models
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for suits list operation")]
pub struct SuitsListReq {
    #[serde(default)]
    #[schemars(description = "Filter by suit status: active, inactive, all")]
    pub filter_type: Option<String>,

    #[serde(default)]
    #[schemars(description = "Filter by suit type: host_app, scenario, shared")]
    pub suit_type: Option<String>,

    #[serde(default)]
    #[schemars(description = "Page limit for pagination (max 100)")]
    pub limit: Option<usize>,

    #[serde(default)]
    #[schemars(description = "Page offset for pagination")]
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for suit details operation")]
pub struct SuitDetailsReq {
    #[schemars(description = "Unique suit identifier")]
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for suit component list (servers, tools, etc.)")]
pub struct SuitComponentListReq {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,

    #[serde(default)]
    #[schemars(description = "Show only enabled components")]
    pub enabled_only: Option<bool>,
}

// Payload Request Models
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for suit management operations")]
pub struct SuitManageReq {
    #[schemars(description = "Suit identifiers (single or multiple)")]
    pub ids: Vec<String>,

    #[schemars(description = "Management action to perform")]
    pub action: SuitAction,

    #[schemars(description = "Whether to trigger client configuration synchronization")]
    #[serde(default)]
    pub sync: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for component management operations (unified single and batch operations)")]
pub struct SuitComponentManageReq {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,

    #[schemars(description = "Component identifiers (single element for individual operations, multiple for batch)")]
    pub component_ids: Vec<String>,

    #[schemars(description = "Management action to perform on component(s)")]
    pub action: SuitComponentAction,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for suit deletion")]
pub struct SuitDeleteReq {
    #[schemars(description = "Unique suit identifier to delete")]
    pub id: String,
}

// Response Models (with Resp suffix)
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suits list operation")]
pub struct SuitsListData {
    #[schemars(description = "List of configuration suits")]
    pub suits: Vec<SuitData>,

    #[schemars(description = "Total number of suits matching filter")]
    pub total: usize,

    #[schemars(description = "ISO 8601 timestamp of response")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit details operation")]
pub struct SuitDetailsData {
    #[schemars(description = "Suit details")]
    pub suit: SuitData,

    #[schemars(description = "Number of enabled servers in suit")]
    pub servers_count: usize,

    #[schemars(description = "Number of enabled tools in suit")]
    pub tools_count: usize,

    #[schemars(description = "Number of enabled resources in suit")]
    pub resources_count: usize,

    #[schemars(description = "Number of enabled prompts in suit")]
    pub prompts_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Single suit operation result")]
pub struct SuitOperationResult {
    #[schemars(description = "Suit identifier")]
    pub id: String,

    #[schemars(description = "Suit name")]
    pub name: String,

    #[schemars(description = "Operation result")]
    pub result: String,

    #[schemars(description = "Current suit status after operation")]
    pub status: String,

    #[schemars(description = "Error message if operation failed")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit management operations")]
pub struct SuitManageData {
    #[schemars(description = "Number of successful operations")]
    pub success_count: usize,

    #[schemars(description = "Number of failed operations")]
    pub failed_count: usize,

    #[schemars(description = "List of operation results")]
    pub results: Vec<SuitOperationResult>,

    #[schemars(description = "ISO 8601 timestamp of operation")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit servers list operation")]
pub struct SuitServersListData {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,

    #[schemars(description = "Suit name")]
    pub suit_name: String,

    #[schemars(description = "List of servers in this suit")]
    pub servers: Vec<SuitServerResp>,

    #[schemars(description = "Total number of servers in suit")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit tools list operation")]
pub struct SuitToolsListData {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,

    #[schemars(description = "Suit name")]
    pub suit_name: String,

    #[schemars(description = "List of tools in this suit")]
    pub tools: Vec<SuitToolData>,

    #[schemars(description = "Total number of tools in suit")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit resources list operation")]
pub struct SuitResourcesListResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitResourcesListData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Data for suit resources list operation")]
pub struct SuitResourcesListData {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,

    #[schemars(description = "Suit name")]
    pub suit_name: String,

    #[schemars(description = "List of resources in this suit")]
    pub resources: Vec<SuitResourceData>,

    #[schemars(description = "Total number of resources in suit")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit prompts list operation")]
pub struct SuitPromptsListResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitPromptsListData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Data for suit prompts list operation")]
pub struct SuitPromptsListData {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,

    #[schemars(description = "Suit name")]
    pub suit_name: String,

    #[schemars(description = "List of prompts in this suit")]
    pub prompts: Vec<SuitPromptData>,

    #[schemars(description = "Total number of prompts in suit")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for component management operations")]
pub struct SuitServerManageData {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,

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

/// Config Suit response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuitData {
    /// Unique ID
    pub id: String,
    /// Name of the configuration suit
    pub name: String,
    /// Description of the configuration suit
    pub description: Option<String>,
    /// Type of the configuration suit (host_app, scenario, shared)
    pub suit_type: String,
    /// Whether multiple configuration suits can be selected simultaneously
    pub multi_select: bool,
    /// Priority of the configuration suit (higher priority wins in case of conflicts)
    pub priority: i32,
    /// Whether the configuration suit is currently active
    pub is_active: bool,
    /// Whether the configuration suit is the default one
    pub is_default: bool,
    /// Allowed operations on this configuration suit
    pub allowed_operations: Vec<String>,
}

/// Create Config Suit request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuitCreateReq {
    /// Name of the configuration suit
    pub name: String,
    /// Description of the configuration suit
    pub description: Option<String>,
    /// Type of the configuration suit (host_app, scenario, shared)
    pub suit_type: String,
    /// Whether multiple configuration suits can be selected simultaneously
    pub multi_select: Option<bool>,
    /// Priority of the configuration suit (higher priority wins in case of conflicts)
    pub priority: Option<i32>,
    /// Whether the configuration suit is currently active
    pub is_active: Option<bool>,
    /// Whether the configuration suit is the default one
    pub is_default: Option<bool>,
    /// Clone from existing configuration suit (optional)
    pub clone_from_id: Option<String>,
}

/// Update Config Suit request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuitUpdateReq {
    /// Configuration suit ID to update
    pub id: String,
    /// Name of the configuration suit
    pub name: Option<String>,
    /// Description of the configuration suit
    pub description: Option<String>,
    /// Type of the configuration suit (host_app, scenario, shared)
    pub suit_type: Option<String>,
    /// Whether multiple configuration suits can be selected simultaneously
    pub multi_select: Option<bool>,
    /// Priority of the configuration suit (higher priority wins in case of conflicts)
    pub priority: Option<i32>,
    /// Whether the configuration suit is currently active
    pub is_active: Option<bool>,
    /// Whether the configuration suit is the default one
    pub is_default: Option<bool>,
}

/// Operation response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Operation response details")]
pub struct SuitOperationData {
    /// Unique ID
    #[schemars(description = "Unique identifier of the configuration suit")]
    pub id: String,
    /// Name of the configuration suit
    #[schemars(description = "Name of the configuration suit")]
    pub name: String,
    /// Result of the operation
    #[schemars(description = "Result description of the operation")]
    pub result: String,
    /// Status after the operation
    #[schemars(description = "Current status after the operation")]
    pub status: String,
    /// Allowed operations after this operation
    #[schemars(description = "List of operations allowed on this configuration suit")]
    pub allowed_operations: Vec<String>,
}

/// Config Suit server response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuitServerResp {
    /// Server ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Whether the server is enabled in this configuration suit
    pub enabled: bool,
    /// Allowed operations on this server
    pub allowed_operations: Vec<String>,
}

/// Config Suit tool response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuitToolData {
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
    /// Whether the tool is enabled in this configuration suit
    pub enabled: bool,
    /// Allowed operations on this tool
    pub allowed_operations: Vec<String>,
}

/// Config Suit resource response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuitResourceData {
    /// Resource ID
    pub id: String,
    /// Server ID
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Resource URI (original URI from upstream server)
    pub resource_uri: String,
    /// Whether the resource is enabled in this configuration suit
    pub enabled: bool,
    /// Allowed operations on this resource
    pub allowed_operations: Vec<String>,
}

/// Config Suit prompt response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuitPromptData {
    /// Prompt ID
    pub id: String,
    /// Server ID
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Prompt name (original name from upstream server)
    pub prompt_name: String,
    /// Whether the prompt is enabled in this configuration suit
    pub enabled: bool,
    /// Allowed operations on this prompt
    pub allowed_operations: Vec<String>,
}

// ==========================================
// SPECIFIC API RESPONSE TYPES
// ==========================================

use crate::api::models::clients::ApiError;

/// Response for suits list operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suits list API response")]
pub struct SuitsListResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitsListData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit details operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit details API response")]
pub struct SuitDetailsResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitDetailsData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit management operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit management API response")]
pub struct SuitManageResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitManageData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for config suit create/update operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Config suit API response")]
pub struct SuitResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit servers list operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit servers list API response")]
pub struct SuitServersListResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitServersListData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit tools list operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit tools list API response")]
pub struct SuitToolsListResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitToolsListData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit component manage operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit component manage API response")]
pub struct SuitServerManageResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitServerManageData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

// ==========================================
// RESPONSE IMPLEMENTATION METHODS
// ==========================================

impl SuitsListResp {
    pub fn success(data: SuitsListData) -> Self {
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

impl SuitDetailsResp {
    pub fn success(data: SuitDetailsData) -> Self {
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

impl SuitManageResp {
    pub fn success(data: SuitManageData) -> Self {
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

impl SuitResp {
    pub fn success(data: SuitData) -> Self {
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

/// Response for suit operation API calls
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit operation API response")]
pub struct SuitOperationResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitOperationData>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

impl SuitOperationResp {
    pub fn success(data: SuitOperationData) -> Self {
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

impl SuitServersListResp {
    pub fn success(data: SuitServersListData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl SuitToolsListResp {
    pub fn success(data: SuitToolsListData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl SuitResourcesListResp {
    pub fn success(data: SuitResourcesListData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl SuitPromptsListResp {
    pub fn success(data: SuitPromptsListData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl SuitServerManageResp {
    pub fn success(data: SuitServerManageData) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}
