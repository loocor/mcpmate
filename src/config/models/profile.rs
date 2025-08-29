// Profile models for MCPMate
// Contains data models for profile

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::common::profile::ProfileType;

/// Profile model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Profile {
    /// Unique ID
    pub id: Option<String>,
    /// Name of the profile
    pub name: String,
    /// Description of the profile
    pub description: Option<String>,
    /// Type of the profile
    #[sqlx(rename = "type")]
    pub profile_type: ProfileType,
    /// Whether multiple profile can be selected simultaneously
    pub multi_select: bool,
    /// Priority of the profile (higher priority wins in case of conflicts)
    pub priority: i32,
    /// Whether the profile is currently active
    pub is_active: bool,
    /// Whether the profile is the default one
    pub is_default: bool,
    /// When the profile was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the profile was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

impl Profile {
    /// Create a new profile
    pub fn new(
        name: String,
        profile_type: ProfileType,
    ) -> Self {
        Self {
            id: None,
            name,
            description: None,
            profile_type,
            multi_select: false,
            priority: 0,
            is_active: false,
            is_default: false,
            created_at: None,
            updated_at: None,
        }
    }

    /// Create a new profile with description
    pub fn new_with_description(
        name: String,
        description: Option<String>,
        profile_type: ProfileType,
    ) -> Self {
        Self {
            id: None,
            name,
            description,
            profile_type,
            multi_select: false,
            priority: 0,
            is_active: false,
            is_default: false,
            created_at: None,
            updated_at: None,
        }
    }

    /// Get the profile type as string (for API compatibility)
    pub fn profile_type_string(&self) -> String {
        self.profile_type.to_string()
    }
}

/// Profile server association model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProfileServer {
    /// Unique ID
    pub id: Option<String>,
    /// Profile ID
    pub profile_id: String,
    /// Server ID
    pub server_id: String,
    /// Whether the server is enabled in this profile
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

/// Profile tool association model - references server_tools
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProfileTool {
    /// Unique ID (generated with "ptool" prefix)
    pub id: String,
    /// Profile ID
    pub profile_id: String,
    /// Server tool ID (references server_tools.id)
    pub server_tool_id: String,
    /// Whether the tool is enabled in this profile
    pub enabled: bool,
    /// When the association was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the association was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

/// Profile tool with server tool details (for JOIN queries)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProfileToolWithDetails {
    /// Profile tool ID
    pub id: String,
    /// Profile ID
    pub profile_id: String,
    /// Server tool ID (references server_tools.id)
    pub server_tool_id: String,
    /// Whether the tool is enabled in this profile
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
