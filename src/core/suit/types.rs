//! Suit module domain type definitions
//!
//! Defines core data types used in configuration suit business logic

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Merged server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedServerConfig {
    /// Server ID
    pub server_id: String,
    /// Server name
    pub name: String,
    /// Server address
    pub address: String,
    /// List of enabled tools
    pub enabled_tools: Vec<String>,
    /// List of source suit IDs
    pub source_suits: Vec<String>,
}

/// Merged tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedToolConfig {
    /// Tool name
    pub tool_name: String,
    /// Whether the tool is enabled
    pub enabled: bool,
    /// List of associated server IDs
    pub server_ids: Vec<String>,
    /// Configuration parameters
    pub config: HashMap<String, serde_json::Value>,
    /// List of source suit IDs
    pub source_suits: Vec<String>,
}

/// Configuration merge result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuitMergeResult {
    /// List of merged server configurations
    pub servers: Vec<MergedServerConfig>,
    /// List of merged tool configurations
    pub tools: Vec<MergedToolConfig>,
    /// List of suit IDs that participated in the merge
    pub merged_suits: Vec<String>,
    /// Merge timestamp
    pub merged_at: chrono::DateTime<chrono::Utc>,
}

/// Tool enablement check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnabledResult {
    /// Tool name
    pub tool_name: String,
    /// Whether the tool is enabled
    pub enabled: bool,
    /// List of server IDs that have this tool enabled
    pub enabled_servers: Vec<String>,
    /// List of related configuration suit IDs
    pub related_suits: Vec<String>,
}

/// Configuration synchronization event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuitConfigChangedEvent {
    /// Event type
    pub event_type: SuitEventType,
    /// Changed suit ID
    pub suit_id: String,
    /// List of affected server IDs
    pub affected_servers: Vec<String>,
    /// List of affected tool names
    pub affected_tools: Vec<String>,
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Suit event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuitEventType {
    /// Suit created
    Created,
    /// Suit updated
    Updated,
    /// Suit deleted
    Deleted,
    /// Suit activated
    Activated,
    /// Suit deactivated
    Deactivated,
}
