// MCP Proxy API models for Config Suit management
// Contains data models for Config Suit endpoints

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

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
pub enum ComponentAction {
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
    #[schemars(description = "Unique suit identifier")]
    pub id: String,
    
    #[schemars(description = "Management action to perform")]
    pub action: SuitAction,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for suit batch management operations")]
pub struct SuitBatchManageReq {
    #[schemars(description = "List of suit identifiers")]
    pub ids: Vec<String>,
    
    #[schemars(description = "Management action to perform on all suits")]
    pub action: SuitAction,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for component management operations")]
pub struct SuitComponentManageReq {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,
    
    #[schemars(description = "Component identifier (server_id, tool_id, etc.)")]
    pub component_id: String,
    
    #[schemars(description = "Management action to perform on component")]
    pub action: ComponentAction,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for component batch management operations")]
pub struct SuitComponentBatchManageReq {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,
    
    #[schemars(description = "List of component identifiers")]
    pub component_ids: Vec<String>,
    
    #[schemars(description = "Management action to perform on all components")]
    pub action: ComponentAction,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Request for suit deletion")]
pub struct DeleteSuitReq {
    #[schemars(description = "Unique suit identifier to delete")]
    pub id: String,
}

// Response Models (with Resp suffix)
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suits list operation")]
pub struct SuitsListResp {
    #[schemars(description = "List of configuration suits")]
    pub suits: Vec<ConfigSuitResp>,
    
    #[schemars(description = "Total number of suits matching filter")]
    pub total: usize,
    
    #[schemars(description = "ISO 8601 timestamp of response")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit details operation")]
pub struct SuitDetailsResp {
    #[schemars(description = "Suit details")]
    pub suit: ConfigSuitResp,
    
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
#[schemars(description = "Response for suit management operations")]
pub struct SuitManageResp {
    #[schemars(description = "Suit identifier")]
    pub id: String,
    
    #[schemars(description = "Suit name")]
    pub name: String,
    
    #[schemars(description = "Operation result")]
    pub result: String,
    
    #[schemars(description = "Current suit status after operation")]
    pub status: String,
    
    #[schemars(description = "ISO 8601 timestamp of operation")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit batch management operations")]
pub struct SuitBatchManageResp {
    #[schemars(description = "Number of successful operations")]
    pub success_count: usize,
    
    #[schemars(description = "List of successfully processed suit IDs")]
    pub successful_ids: Vec<String>,
    
    #[schemars(description = "Map of failed suit IDs to error messages")]
    pub failed_operations: HashMap<String, String>,
    
    #[schemars(description = "ISO 8601 timestamp of operation")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit servers list operation")]
pub struct SuitServersListResp {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,
    
    #[schemars(description = "Suit name")]
    pub suit_name: String,
    
    #[schemars(description = "List of servers in this suit")]
    pub servers: Vec<ConfigSuitServerResp>,
    
    #[schemars(description = "Total number of servers in suit")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit tools list operation")]
pub struct SuitToolsListResp {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,
    
    #[schemars(description = "Suit name")]
    pub suit_name: String,
    
    #[schemars(description = "List of tools in this suit")]
    pub tools: Vec<ConfigSuitToolResp>,
    
    #[schemars(description = "Total number of tools in suit")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit resources list operation")]
pub struct SuitResourcesListResp {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,
    
    #[schemars(description = "Suit name")]
    pub suit_name: String,
    
    #[schemars(description = "List of resources in this suit")]
    pub resources: Vec<ConfigSuitResourceResp>,
    
    #[schemars(description = "Total number of resources in suit")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for suit prompts list operation")]
pub struct SuitPromptsListResp {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,
    
    #[schemars(description = "Suit name")]
    pub suit_name: String,
    
    #[schemars(description = "List of prompts in this suit")]
    pub prompts: Vec<ConfigSuitPromptResp>,
    
    #[schemars(description = "Total number of prompts in suit")]
    pub total: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Response for component management operations")]
pub struct SuitComponentManageResp {
    #[schemars(description = "Suit identifier")]
    pub suit_id: String,
    
    #[schemars(description = "Component identifier")]
    pub component_id: String,
    
    #[schemars(description = "Component type (server, tool, resource, prompt)")]
    pub component_type: String,
    
    #[schemars(description = "Operation result")]
    pub result: String,
    
    #[schemars(description = "Current component status after operation")]
    pub status: String,
    
    #[schemars(description = "ISO 8601 timestamp of operation")]
    pub timestamp: String,
}

// ==========================================
// LEGACY MODELS (kept for backward compatibility)
// ==========================================

/// Config Suit response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConfigSuitResp {
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

/// Config Suit list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSuitListResp {
    /// List of configuration suits
    pub suits: Vec<ConfigSuitResp>,
}

/// Create Config Suit request
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateConfigSuitReq {
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
pub struct UpdateConfigSuitReq {
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

/// Batch operation request
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchOperationReq {
    /// List of IDs to operate on
    pub ids: Vec<String>,
}

/// Operation response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Operation response details")]
pub struct SuitOperationResp {
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

/// Batch operation response
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchOperationResp {
    /// Number of successful operations
    pub success_count: usize,
    /// List of successful operations
    pub successful_ids: Vec<String>,
    /// List of failed operations
    pub failed_ids: HashMap<String, String>,
}

/// Config Suit server response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConfigSuitServerResp {
    /// Server ID
    pub id: String,
    /// Server name
    pub name: String,
    /// Whether the server is enabled in this configuration suit
    pub enabled: bool,
    /// Allowed operations on this server
    pub allowed_operations: Vec<String>,
}

/// Config Suit servers list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSuitServersResp {
    /// Configuration suit ID
    pub suit_id: String,
    /// Configuration suit name
    pub suit_name: String,
    /// List of servers in this configuration suit
    pub servers: Vec<ConfigSuitServerResp>,
}

/// Config Suit tool response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConfigSuitToolResp {
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

/// Config Suit tools list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSuitToolsResp {
    /// Configuration suit ID
    pub suit_id: String,
    /// Configuration suit name
    pub suit_name: String,
    /// List of tools in this configuration suit
    pub tools: Vec<ConfigSuitToolResp>,
}

/// Config Suit resource response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConfigSuitResourceResp {
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

/// Config Suit resources list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSuitResourcesResp {
    /// Configuration suit ID
    pub suit_id: String,
    /// Configuration suit name
    pub suit_name: String,
    /// List of resources in this configuration suit
    pub resources: Vec<ConfigSuitResourceResp>,
}

/// Config Suit prompt response
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ConfigSuitPromptResp {
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

/// Config Suit prompts list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSuitPromptsResp {
    /// Configuration suit ID
    pub suit_id: String,
    /// Configuration suit name
    pub suit_name: String,
    /// List of prompts in this configuration suit
    pub prompts: Vec<ConfigSuitPromptResp>,
}

// ==========================================
// SPECIFIC API RESPONSE TYPES
// ==========================================

use crate::api::models::clients::ApiError;

/// Response for suits list operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suits list API response")]
pub struct SuitsListApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitsListResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit details operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit details API response")]
pub struct SuitDetailsApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitDetailsResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit management operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit management API response")]
pub struct SuitManageApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitManageResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for config suit create/update operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Config suit API response")]
pub struct ConfigSuitApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ConfigSuitResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit batch manage operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit batch manage API response")]
pub struct SuitBatchManageApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitBatchManageResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit servers list operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit servers list API response")]
pub struct SuitServersListApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitServersListResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit tools list operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit tools list API response")]
pub struct SuitToolsListApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitToolsListResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

/// Response for suit component manage operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit component manage API response")]
pub struct SuitComponentManageApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitComponentManageResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

// ==========================================
// RESPONSE IMPLEMENTATION METHODS
// ==========================================

impl SuitsListApiResp {
    pub fn success(data: SuitsListResp) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
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

impl SuitDetailsApiResp {
    pub fn success(data: SuitDetailsResp) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
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

impl SuitManageApiResp {
    pub fn success(data: SuitManageResp) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
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

impl ConfigSuitApiResp {
    pub fn success(data: ConfigSuitResp) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
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

impl SuitBatchManageApiResp {
    pub fn success(data: SuitBatchManageResp) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(error: ApiError) -> Self {
        Self { success: false, data: None, error: Some(error) }
    }
}

/// Response for suit operation API calls
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Suit operation API response")]
pub struct SuitOperationApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<SuitOperationResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

impl SuitOperationApiResp {
    pub fn success(data: SuitOperationResp) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
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

impl SuitServersListApiResp {
    pub fn success(data: SuitServersListResp) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(error: ApiError) -> Self {
        Self { success: false, data: None, error: Some(error) }
    }
}

impl SuitToolsListApiResp {
    pub fn success(data: SuitToolsListResp) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(error: ApiError) -> Self {
        Self { success: false, data: None, error: Some(error) }
    }
}

impl SuitComponentManageApiResp {
    pub fn success(data: SuitComponentManageResp) -> Self {
        Self { success: true, data: Some(data), error: None }
    }
    pub fn error(error: ApiError) -> Self {
        Self { success: false, data: None, error: Some(error) }
    }
}
