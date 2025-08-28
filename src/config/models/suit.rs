// Config Suit models for MCPMate
// Contains data models for configuration suits

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::common::config::ConfigSuitType;

/// Configuration suit model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConfigSuit {
    /// Unique ID
    pub id: Option<String>,
    /// Name of the configuration suit
    pub name: String,
    /// Description of the configuration suit
    pub description: Option<String>,
    /// Type of the configuration suit
    #[sqlx(rename = "type")]
    pub suit_type: ConfigSuitType,
    /// Whether multiple configuration suits can be selected simultaneously
    pub multi_select: bool,
    /// Priority of the configuration suit (higher priority wins in case of conflicts)
    pub priority: i32,
    /// Whether the configuration suit is currently active
    pub is_active: bool,
    /// Whether the configuration suit is the default one
    pub is_default: bool,
    /// When the configuration suit was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the configuration suit was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

impl ConfigSuit {
    /// Create a new configuration suit
    pub fn new(
        name: String,
        suit_type: ConfigSuitType,
    ) -> Self {
        Self {
            id: None,
            name,
            description: None,
            suit_type,
            multi_select: false,
            priority: 0,
            is_active: false,
            is_default: false,
            created_at: None,
            updated_at: None,
        }
    }

    /// Create a new configuration suit with description
    pub fn new_with_description(
        name: String,
        description: Option<String>,
        suit_type: ConfigSuitType,
    ) -> Self {
        Self {
            id: None,
            name,
            description,
            suit_type,
            multi_select: false,
            priority: 0,
            is_active: false,
            is_default: false,
            created_at: None,
            updated_at: None,
        }
    }

    /// Get the configuration suit type as string (for API compatibility)
    pub fn suit_type_string(&self) -> String {
        self.suit_type.to_string()
    }
}

/// Configuration suit server association model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConfigSuitServer {
    /// Unique ID
    pub id: Option<String>,
    /// Configuration suit ID
    pub suit_id: String,
    /// Server ID
    pub server_id: String,
    /// Whether the server is enabled in this configuration suit
    pub enabled: bool,
    /// When the association was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the association was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

/// Server tool mapping model - maintains global tool name mappings
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServerTool {
    /// Unique ID (generated with "stool" prefix)
    pub id: String,
    /// Server ID
    pub server_id: String,
    /// Server name (cached for performance)
    pub server_name: String,
    /// Tool name (original name from upstream server)
    pub tool_name: String,
    /// Unique name for external display and routing
    pub unique_name: String,
    /// Tool description (from MCP server)
    pub description: Option<String>,
    /// When the mapping was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the mapping was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

/// Configuration suit tool association model - references server_tools
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConfigSuitTool {
    /// Unique ID (generated with "cstool" prefix)
    pub id: String,
    /// Configuration suit ID
    pub suit_id: String,
    /// Server tool ID (references server_tools.id)
    pub server_tool_id: String,
    /// Whether the tool is enabled in this configuration suit
    pub enabled: bool,
    /// When the association was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the association was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

/// Configuration suit tool with server tool details (for JOIN queries)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConfigSuitToolWithDetails {
    /// Config suit tool ID
    pub id: String,
    /// Configuration suit ID
    pub suit_id: String,
    /// Server tool ID (references server_tools.id)
    pub server_tool_id: String,
    /// Whether the tool is enabled in this configuration suit
    pub enabled: bool,
    /// When the association was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the association was last updated
    pub updated_at: Option<DateTime<Utc>>,
    /// Server ID (from server_tools)
    pub server_id: String,
    /// Server name (from server_tools)
    pub server_name: String,
    /// Tool name (from server_tools)
    pub tool_name: String,
    /// Unique name (from server_tools)
    pub unique_name: String,
    /// Tool description (from server_tools)
    pub description: Option<String>,
}
