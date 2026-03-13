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

/// Role of a profile in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ProfileRole {
    /// User-managed profile with no special guarantees
    #[default]
    User,
    /// System default anchor profile that must always remain active
    DefaultAnchor,
}

impl ProfileRole {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileRole::User => "user",
            ProfileRole::DefaultAnchor => "default_anchor",
        }
    }

    /// Returns true when this role denotes the default anchor profile
    pub fn is_default_anchor(&self) -> bool {
        matches!(self, ProfileRole::DefaultAnchor)
    }
}

impl fmt::Display for ProfileRole {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type for ProfileRole parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProfileRoleError;

impl fmt::Display for ParseProfileRoleError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid profile role")
    }
}

impl std::error::Error for ParseProfileRoleError {}

impl FromStr for ProfileRole {
    type Err = ParseProfileRoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(ProfileRole::User),
            "default_anchor" => Ok(ProfileRole::DefaultAnchor),
            _ => Err(ParseProfileRoleError),
        }
    }
}

impl Serialize for ProfileRole {
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

struct ProfileRoleVisitor;

impl<'de> Visitor<'de> for ProfileRoleVisitor {
    type Value = ProfileRole;

    fn expecting(
        &self,
        formatter: &mut fmt::Formatter,
    ) -> fmt::Result {
        formatter.write_str("a string representing a profile role")
    }

    fn visit_str<E>(
        self,
        value: &str,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ProfileRole::from_str(value).map_err(|_| E::custom(format!("invalid profile role: {value}")))
    }
}

impl<'de> Deserialize<'de> for ProfileRole {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ProfileRoleVisitor)
    }
}

// SQLx type mapping for ProfileRole
impl Type<Sqlite> for ProfileRole {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ProfileRole {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        <String as Encode<Sqlite>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for ProfileRole {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <String as Decode<Sqlite>>::decode(value)?;
        ProfileRole::from_str(&s).map_err(|e| Box::new(e) as BoxDynError)
    }
}
