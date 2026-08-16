// Profile models for MCPMate
// Contains data models for profile

use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    Decode, Encode, FromRow, Sqlite, Type,
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
};

use crate::common::profile::{ProfileRole, ProfileType};

/// Runtime-isolated Profile authoring mode.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMode {
    #[default]
    Capability,
    Workflow,
}

impl ProfileMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Capability => "capability",
            Self::Workflow => "workflow",
        }
    }
}

impl fmt::Display for ProfileMode {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ProfileMode {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "capability" => Ok(Self::Capability),
            "workflow" => Ok(Self::Workflow),
            _ => Err("invalid Profile mode"),
        }
    }
}

impl Type<Sqlite> for ProfileMode {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ProfileMode {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        <String as Encode<Sqlite>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for ProfileMode {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <String as Decode<Sqlite>>::decode(value)?;
        ProfileMode::from_str(&value)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error).into())
    }
}

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
    /// Role of the profile within the system lifecycle
    pub role: ProfileRole,
    /// Priority of the profile (higher priority wins in case of conflicts)
    pub priority: i32,
    /// Whether the profile is currently active
    pub is_active: bool,
    /// Whether the profile is the default one
    pub is_default: bool,
    /// Monotonic generation for Profile authoring concurrency control.
    pub authoring_generation: i64,
    /// Authoring mode that determines whether this Profile is a capability set or workflow specification.
    pub profile_mode: ProfileMode,
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
            role: ProfileRole::User,
            priority: 0,
            is_active: false,
            is_default: false,
            authoring_generation: 0,
            profile_mode: ProfileMode::Capability,
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
            role: ProfileRole::User,
            priority: 0,
            is_active: false,
            is_default: false,
            authoring_generation: 0,
            profile_mode: ProfileMode::Capability,
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
    /// Policy applied when the server later exposes a new CapabilityRef.
    pub new_ref_policy: String,
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

/// Profile capability-level Tool relationship.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProfileTool {
    pub profile_id: String,
    pub ref_id: String,
    pub enabled: bool,
}

/// Profile Tool relationship with current display projection.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProfileToolWithDetails {
    pub profile_id: String,
    pub ref_id: String,
    pub enabled: bool,
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub unique_name: String,
    pub description: Option<String>,
    pub state: String,
    pub state_generation: i64,
}
