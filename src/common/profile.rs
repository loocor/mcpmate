//! Profile-related types and constants for MCPMate
//!
//! This module contains types and constants related to profile and settings.

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

/// Profile type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileType {
    /// Host application specific configuration
    HostApp,
    /// Scenario specific configuration
    Scenario,
    /// Shared configuration
    Shared,
}

impl ProfileType {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileType::HostApp => "host_app",
            ProfileType::Scenario => "scenario",
            ProfileType::Shared => "shared",
        }
    }
}

impl fmt::Display for ProfileType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type for ProfileType parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProfileTypeError;

impl fmt::Display for ParseProfileTypeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid profile type")
    }
}

impl std::error::Error for ParseProfileTypeError {}

impl FromStr for ProfileType {
    type Err = ParseProfileTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "host_app" => Ok(ProfileType::HostApp),
            "scenario" => Ok(ProfileType::Scenario),
            "shared" => Ok(ProfileType::Shared),
            _ => Err(ParseProfileTypeError),
        }
    }
}

impl Serialize for ProfileType {
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

struct ProfileTypeVisitor;

impl<'de> Visitor<'de> for ProfileTypeVisitor {
    type Value = ProfileType;

    fn expecting(
        &self,
        formatter: &mut fmt::Formatter,
    ) -> fmt::Result {
        formatter.write_str("a string representing a profile type")
    }

    fn visit_str<E>(
        self,
        value: &str,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ProfileType::from_str(value).map_err(|_| E::custom(format!("invalid profile type: {value}")))
    }
}

impl<'de> Deserialize<'de> for ProfileType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ProfileTypeVisitor)
    }
}

// SQLx type mapping for ProfileType
impl Type<Sqlite> for ProfileType {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ProfileType {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        <String as Encode<Sqlite>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for ProfileType {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <String as Decode<Sqlite>>::decode(value)?;
        ProfileType::from_str(&s).map_err(|e| Box::new(e) as BoxDynError)
    }
}
