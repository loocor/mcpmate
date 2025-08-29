//! Profile module domain type definitions
//!
//! Defines core data types used in profile business logic

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
    /// List of source profile IDs
    pub source_profile: Vec<String>,
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
    /// List of source profile IDs
    pub source_profile: Vec<String>,
}

/// Configuration merge result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMergeResult {
    /// List of merged server configurations
    pub servers: Vec<MergedServerConfig>,
    /// List of merged tool configurations
    pub tools: Vec<MergedToolConfig>,
    /// List of profile IDs that participated in the merge
    pub merged_profile: Vec<String>,
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
    /// List of related profile IDs
    pub related_profile: Vec<String>,
}

/// Configuration synchronization event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfigChangedEvent {
    /// Event type
    pub event_type: ProfileEventType,
    /// Changed profile ID
    pub profile_id: String,
    /// List of affected server IDs
    pub affected_servers: Vec<String>,
    /// List of affected tool names
    pub affected_tools: Vec<String>,
    /// Event timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Profile event type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfileEventType {
    /// Profile created
    Created,
    /// Profile updated
    Updated,
    /// Profile deleted
    Deleted,
    /// Profile activated
    Activated,
    /// Profile deactivated
    Deactivated,
}
