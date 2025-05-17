// MCP Proxy API models for notifications
// Contains data models for notification endpoints

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Tool change operation type
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ToolChangeOperation {
    /// Enable tools
    Enable,
    /// Disable tools
    Disable,
    /// Update tools configuration
    Update,
}

/// Scope of the tool change
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ToolChangeScope {
    /// All tools
    All,
    /// Tools from specific services
    Services,
    /// Specific tools
    Tools,
}

/// Tool identifier with service context
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ToolIdentifier {
    /// Tool name or ID
    pub name: String,

    /// Optional service ID (if not provided, will apply to all services with this tool)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_id: Option<String>,
}

/// Tool list changed notification request
#[derive(Debug, Deserialize)]
pub struct ToolsChangedRequest {
    /// Operation type
    pub operation: ToolChangeOperation,

    /// Scope of the change
    pub scope: ToolChangeScope,

    /// Service IDs to apply the change to (required when scope is Services)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_ids: Option<HashSet<String>>,

    /// Tool identifiers to apply the change to (required when scope is Tools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolIdentifier>>,

    /// Optional reason for the change
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Notification response
#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    /// Number of clients notified
    pub notified_clients: usize,
    /// Success message
    pub message: String,
    /// Details about the operation
    pub details: ToolsChangedDetails,
}

/// Details about the tools changed operation
#[derive(Debug, Serialize)]
pub struct ToolsChangedDetails {
    /// Operation performed
    pub operation: String,
    /// Scope of the change
    pub scope: String,
    /// Number of services affected
    pub services_affected: usize,
    /// Number of tools affected
    pub tools_affected: usize,
}
