// Config Suit models for MCPMate
// Contains data models for configuration suits

use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Configuration suit type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConfigSuitType {
    /// Host application configuration
    HostApp,
    /// Scenario-based configuration
    Scenario,
    /// Shared configuration
    Shared,
}

impl ConfigSuitType {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigSuitType::HostApp => "host_app",
            ConfigSuitType::Scenario => "scenario",
            ConfigSuitType::Shared => "shared",
        }
    }
}

/// Error type for ConfigSuitType parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseConfigSuitTypeError;

impl FromStr for ConfigSuitType {
    type Err = ParseConfigSuitTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "host_app" => Ok(ConfigSuitType::HostApp),
            "scenario" => Ok(ConfigSuitType::Scenario),
            "shared" => Ok(ConfigSuitType::Shared),
            _ => Err(ParseConfigSuitTypeError),
        }
    }
}

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
    pub suit_type: String,
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
            suit_type: suit_type.as_str().to_string(),
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
            suit_type: suit_type.as_str().to_string(),
            multi_select: false,
            priority: 0,
            is_active: false,
            is_default: false,
            created_at: None,
            updated_at: None,
        }
    }

    /// Get the configuration suit type
    pub fn get_type(&self) -> Option<ConfigSuitType> {
        ConfigSuitType::from_str(&self.suit_type).ok()
    }
}

/// Configuration suit server association model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConfigSuitServer {
    /// Unique ID
    pub id: Option<String>,
    /// Configuration suit ID
    pub config_suit_id: String,
    /// Server ID
    pub server_id: String,
    /// Whether the server is enabled in this configuration suit
    pub enabled: bool,
    /// When the association was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the association was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

/// Configuration suit tool association model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConfigSuitTool {
    /// Unique ID (UUID)
    pub id: Option<String>,
    /// Configuration suit ID
    pub config_suit_id: String,
    /// Server ID
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Tool name
    pub tool_name: String,
    /// Prefixed/qualified name for the tool (to avoid conflicts)
    pub prefixed_name: Option<String>,
    /// Whether the tool is enabled in this configuration suit
    pub enabled: bool,
    /// When the association was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the association was last updated
    pub updated_at: Option<DateTime<Utc>>,
}
