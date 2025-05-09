// Database models for MCPMate
// Contains data models for database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Tool configuration model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ToolConfig {
    /// Unique ID
    pub id: Option<i64>,
    /// Name of the server that provides this tool
    pub server_name: String,
    /// Name of the tool
    pub tool_name: String,
    /// Alias name for the tool (user-defined)
    pub alias_name: Option<String>,
    /// Whether the tool is enabled
    pub enabled: bool,
    /// When the configuration was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the configuration was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

impl ToolConfig {
    /// Create a new tool configuration
    pub fn new(server_name: String, tool_name: String, enabled: bool) -> Self {
        Self {
            id: None,
            server_name,
            tool_name,
            alias_name: None,
            enabled,
            created_at: None,
            updated_at: None,
        }
    }

    /// Create a new tool configuration with alias
    pub fn new_with_alias(
        server_name: String,
        tool_name: String,
        alias_name: Option<String>,
        enabled: bool,
    ) -> Self {
        Self {
            id: None,
            server_name,
            tool_name,
            alias_name,
            enabled,
            created_at: None,
            updated_at: None,
        }
    }
}

/// Tool configuration update model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfigUpdate {
    /// Whether the tool is enabled
    pub enabled: bool,
    /// Alias name for the tool (user-defined)
    pub alias_name: Option<String>,
}
