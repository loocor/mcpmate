// MCP Proxy API models for Config Suit management
// Contains data models for Config Suit endpoints

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Config Suit response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSuitResponse {
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
pub struct ConfigSuitListResponse {
    /// List of configuration suits
    pub suits: Vec<ConfigSuitResponse>,
}

/// Create Config Suit request
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateConfigSuitRequest {
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
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateConfigSuitRequest {
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
pub struct BatchOperationRequest {
    /// List of IDs to operate on
    pub ids: Vec<String>,
}

/// Operation response
#[derive(Debug, Serialize, Deserialize)]
pub struct SuitOperationResponse {
    /// Unique ID
    pub id: String,
    /// Name of the configuration suit
    pub name: String,
    /// Result of the operation
    pub result: String,
    /// Status after the operation
    pub status: String,
    /// Allowed operations after this operation
    pub allowed_operations: Vec<String>,
}

/// Batch operation response
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchOperationResponse {
    /// Number of successful operations
    pub success_count: usize,
    /// List of successful operations
    pub successful_ids: Vec<String>,
    /// List of failed operations
    pub failed_ids: HashMap<String, String>,
}

/// Config Suit server response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSuitServerResponse {
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
pub struct ConfigSuitServersResponse {
    /// Configuration suit ID
    pub suit_id: String,
    /// Configuration suit name
    pub suit_name: String,
    /// List of servers in this configuration suit
    pub servers: Vec<ConfigSuitServerResponse>,
}

/// Config Suit tool response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSuitToolResponse {
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
pub struct ConfigSuitToolsResponse {
    /// Configuration suit ID
    pub suit_id: String,
    /// Configuration suit name
    pub suit_name: String,
    /// List of tools in this configuration suit
    pub tools: Vec<ConfigSuitToolResponse>,
}

/// Config Suit resource response
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigSuitResourceResponse {
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
pub struct ConfigSuitResourcesResponse {
    /// Configuration suit ID
    pub suit_id: String,
    /// Configuration suit name
    pub suit_name: String,
    /// List of resources in this configuration suit
    pub resources: Vec<ConfigSuitResourceResponse>,
}
