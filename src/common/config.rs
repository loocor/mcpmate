//! Configuration-related types and constants for MCPMate
//!
//! This module contains types and constants related to configuration suits and settings.

use std::{fmt, str::FromStr};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Visitor},
};
use sqlx::{
    Decode, Encode, Sqlite, Type,
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
};

/// Configuration keys used in client configs
pub mod config_keys {
    /// Key for MCP tool key in config files
    pub const MCP_TOOL_KEY: &str = "MCPTool";
    /// Key for name in config files
    pub const NAME_KEY: &str = "name";
    /// Key for type in config files
    pub const TYPE_KEY: &str = "type";
    /// Key for transports in config files
    pub const TRANSPORTS_KEY: &str = "transports";
    /// Key for parameters in config files
    pub const PARAMETERS_KEY: &str = "parameters";
    /// Key for tool settings in config files
    pub const TOOL_SETTINGS_KEY: &str = "toolSettings";
    /// Key for tools in config files
    pub const TOOLS_KEY: &str = "tools";
    /// Key for MCPMate in config files
    pub const MCPMATE: &str = "MCPMate";
}

/// Default values used in configuration
pub mod defaults {
    /// Default server port
    pub const DEFAULT_PORT: u16 = 8033;
    /// Default server host
    pub const DEFAULT_HOST: &str = "127.0.0.1";
    /// Default cache TTL in seconds
    pub const DEFAULT_CACHE_TTL: u32 = 86400; // 24 hours
    /// Default requests limit
    pub const DEFAULT_REQUESTS_LIMIT: u32 = 100;
    /// Default runtime value
    pub const RUNTIME: &str = "node";
}

/// Configuration suit type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigSuitType {
    /// Host application specific configuration
    HostApp,
    /// Scenario specific configuration
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

impl fmt::Display for ConfigSuitType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type for ConfigSuitType parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseConfigSuitTypeError;

impl fmt::Display for ParseConfigSuitTypeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid config suit type")
    }
}

impl std::error::Error for ParseConfigSuitTypeError {}

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

impl Serialize for ConfigSuitType {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

struct ConfigSuitTypeVisitor;

impl<'de> Visitor<'de> for ConfigSuitTypeVisitor {
    type Value = ConfigSuitType;

    fn expecting(
        &self,
        formatter: &mut fmt::Formatter,
    ) -> fmt::Result {
        formatter.write_str("a string representing a config suit type")
    }

    fn visit_str<E>(
        self,
        value: &str,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ConfigSuitType::from_str(value)
            .map_err(|_| E::custom(format!("invalid config suit type: {value}")))
    }
}

impl<'de> Deserialize<'de> for ConfigSuitType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ConfigSuitTypeVisitor)
    }
}

// SQLx type mapping for ConfigSuitType
impl Type<Sqlite> for ConfigSuitType {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ConfigSuitType {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        <String as Encode<Sqlite>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for ConfigSuitType {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <String as Decode<Sqlite>>::decode(value)?;
        ConfigSuitType::from_str(&s).map_err(|e| Box::new(e) as BoxDynError)
    }
}
