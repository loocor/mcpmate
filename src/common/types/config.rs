//! Configuration-related types for MCPMate
//!
//! This module contains types related to configuration suits and settings.

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

/// Configuration suit type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        write!(f, "invalid configuration suit type")
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
        formatter.write_str("a string representing a configuration suit type")
    }

    fn visit_str<E>(
        self,
        value: &str,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ConfigSuitType::from_str(value)
            .map_err(|_| E::custom(format!("invalid configuration suit type: {}", value)))
    }
}

impl<'de> Deserialize<'de> for ConfigSuitType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
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
