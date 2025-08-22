// MCP Proxy API models for notifications
// Contains data models for notification endpoints

use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, PartialEq, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
#[schemars(description = "Tool change operation type (default: update)")]
pub enum ToolChangeOperation {
    #[schemars(description = "Enable tools")]
    Enable,
    #[schemars(description = "Disable tools")]
    Disable,
    #[default]
    #[schemars(description = "Update tools configuration (default)")]
    Update,
}

#[derive(Debug, Deserialize, Clone, PartialEq, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
#[schemars(description = "Scope of the tool change (default: all)")]
pub enum ToolChangeScope {
    #[default]
    #[schemars(description = "All tools (default)")]
    All,
    #[schemars(description = "Tools from specific services")]
    Services,
    #[schemars(description = "Specific tools")]
    Tools,
}

#[derive(Debug, Deserialize, Clone, PartialEq, JsonSchema)]
#[schemars(description = "Tool identifier with service context")]
pub struct ToolIdentifier {
    #[schemars(description = "Tool name or ID")]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Optional service ID (if not provided, will apply to all services with this tool)")]
    pub service_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Tool list changed notification request")]
pub struct ToolsChangedReq {
    #[serde(default)]
    #[schemars(
        default = "ToolChangeOperation::default",
        description = "Operation type (default: update)"
    )]
    pub operation: ToolChangeOperation,

    #[serde(default)]
    #[schemars(
        default = "ToolChangeScope::default",
        description = "Scope of the change (default: all)"
    )]
    pub scope: ToolChangeScope,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Service IDs to apply the change to (required when scope is Services)")]
    pub service_ids: Option<HashSet<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Tool identifiers to apply the change to (required when scope is Tools)")]
    pub tools: Option<Vec<ToolIdentifier>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Optional reason for the change")]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Tools changed notification response")]
pub struct ToolsChangedResp {
    #[schemars(description = "Number of clients notified")]
    pub notified_clients: usize,
    #[schemars(description = "Success message")]
    pub message: String,
    #[schemars(description = "Details about the operation")]
    pub details: ToolsChangedDetails,
}

#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Details about the tools changed operation")]
pub struct ToolsChangedDetails {
    #[schemars(description = "Operation performed")]
    pub operation: String,
    #[schemars(description = "Scope of the change")]
    pub scope: String,
    #[schemars(description = "Number of services affected")]
    pub services_affected: usize,
    #[schemars(description = "Number of tools affected")]
    pub tools_affected: usize,
}

// ==========================================
// SPECIFIC API RESPONSE TYPES
// ==========================================

use crate::api::models::clients::ApiError;

/// Response for tools changed operations
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(description = "Tools changed API response")]
pub struct ToolsChangedApiResp {
    #[schemars(description = "Whether the operation was successful")]
    pub success: bool,
    #[schemars(description = "Response data when successful")]
    pub data: Option<ToolsChangedResp>,
    #[schemars(description = "Error information when failed")]
    pub error: Option<ApiError>,
}

impl ToolsChangedApiResp {
    /// Create a success response
    pub fn success(data: ToolsChangedResp) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(error: ApiError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}
