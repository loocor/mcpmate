//! Status-related types for MCPMate
//!
//! This module contains types related to status and state.

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

/// Enabled status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnabledStatus {
    /// Enabled
    Enabled,
    /// Disabled
    Disabled,
}

impl EnabledStatus {
    /// Convert to boolean
    pub fn as_bool(&self) -> bool {
        match self {
            EnabledStatus::Enabled => true,
            EnabledStatus::Disabled => false,
        }
    }

    /// Create from boolean
    pub fn from_bool(value: bool) -> Self {
        if value {
            EnabledStatus::Enabled
        } else {
            EnabledStatus::Disabled
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            EnabledStatus::Enabled => "enabled",
            EnabledStatus::Disabled => "disabled",
        }
    }
}

impl fmt::Display for EnabledStatus {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type for EnabledStatus parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEnabledStatusError;

impl fmt::Display for ParseEnabledStatusError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid enabled status")
    }
}

impl std::error::Error for ParseEnabledStatusError {}

impl FromStr for EnabledStatus {
    type Err = ParseEnabledStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "enabled" | "true" | "1" | "yes" | "on" => Ok(EnabledStatus::Enabled),
            "disabled" | "false" | "0" | "no" | "off" => Ok(EnabledStatus::Disabled),
            _ => Err(ParseEnabledStatusError),
        }
    }
}

impl Serialize for EnabledStatus {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(self.as_bool())
    }
}

struct EnabledStatusVisitor;

impl<'de> Visitor<'de> for EnabledStatusVisitor {
    type Value = EnabledStatus;

    fn expecting(
        &self,
        formatter: &mut fmt::Formatter,
    ) -> fmt::Result {
        formatter.write_str("a boolean or string representing an enabled status")
    }

    fn visit_bool<E>(
        self,
        value: bool,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(EnabledStatus::from_bool(value))
    }

    fn visit_str<E>(
        self,
        value: &str,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        EnabledStatus::from_str(value)
            .map_err(|_| E::custom(format!("invalid enabled status: {}", value)))
    }
}

impl<'de> Deserialize<'de> for EnabledStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        deserializer.deserialize_any(EnabledStatusVisitor)
    }
}

// SQLx type mapping for EnabledStatus
impl Type<Sqlite> for EnabledStatus {
    fn type_info() -> SqliteTypeInfo {
        <i64 as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for EnabledStatus {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        let value = match self {
            EnabledStatus::Enabled => 1i64,
            EnabledStatus::Disabled => 0i64,
        };
        <i64 as Encode<Sqlite>>::encode_by_ref(&value, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for EnabledStatus {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        // Try to decode as i64 (SQLite's INTEGER type)
        match <i64 as Decode<Sqlite>>::decode(value) {
            Ok(i) => Ok(if i == 0 {
                EnabledStatus::Disabled
            } else {
                EnabledStatus::Enabled
            }),
            Err(e) => Err(e),
        }
    }
}
