//! Server-related types and constants for MCPMate
//!
//! This module contains types and constants related to server configuration and transport.

use std::{fmt, str::FromStr};

use schemars::JsonSchema;
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

// Transport format constants moved to src/common/constants.rs::transport module
use crate::common::constants::transport;

/// Transport priority order for hosted mode selection
pub const TRANSPORT_PRIORITY: &[&str] = &[transport::STREAMABLE_HTTP, transport::SSE, transport::STDIO];

/// Server identity (id + name)
///
/// Pool/DB with `id` as the authority key; protocol layer/external display with `name`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerIdentity {
    pub id: String,
    pub name: String,
}

impl ServerIdentity {
    pub fn new(
        id: String,
        name: String,
    ) -> Self {
        Self { id, name }
    }
}

/// Server type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, JsonSchema)]
#[schemars(description = "Server type enum")]
pub enum ServerType {
    /// Standard input/output server
    Stdio,
    /// Server-Sent Events server
    Sse,
    /// Streamable HTTP server
    StreamableHttp,
}

impl ServerType {
    /// Convert to string (database format)
    pub fn as_str(&self) -> &'static str {
        match self {
            ServerType::Stdio => "stdio",
            ServerType::Sse => "sse",
            ServerType::StreamableHttp => "streamable_http",
        }
    }

    /// Get the client-side representation (for JSON configs)
    pub fn client_format(&self) -> &'static str {
        match self {
            ServerType::Stdio => transport::STDIO,
            ServerType::Sse => transport::SSE,
            ServerType::StreamableHttp => transport::STREAMABLE_HTTP,
        }
    }

    /// Create from client format string (case-insensitive)
    pub fn from_client_format(s: &str) -> Result<Self, ParseServerTypeError> {
        let lc = s.to_ascii_lowercase();
        match lc.as_str() {
            x if x == transport::STDIO => Ok(ServerType::Stdio),
            x if x == transport::SSE => Ok(ServerType::Sse),
            x if x == transport::STREAMABLE_HTTP => Ok(ServerType::StreamableHttp),
            _ => Err(ParseServerTypeError),
        }
    }
}

impl fmt::Display for ServerType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type for ServerType parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseServerTypeError;

impl fmt::Display for ParseServerTypeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid server type")
    }
}

impl std::error::Error for ParseServerTypeError {}

impl FromStr for ServerType {
    type Err = ParseServerTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lc = s.to_ascii_lowercase();
        match lc.as_str() {
            "stdio" => Ok(ServerType::Stdio),
            "sse" => Ok(ServerType::Sse),
            "streamable_http" => Ok(ServerType::StreamableHttp),
            _ => {
                tracing::error!(
                    "Invalid server type '{}'. Allowed: 'stdio'|'sse'|'streamable_http' (case-insensitive)",
                    s
                );
                Err(ParseServerTypeError)
            }
        }
    }
}

impl Serialize for ServerType {
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

struct ServerTypeVisitor;

impl<'de> Visitor<'de> for ServerTypeVisitor {
    type Value = ServerType;

    fn expecting(
        &self,
        formatter: &mut fmt::Formatter,
    ) -> fmt::Result {
        formatter.write_str("a string representing a server type")
    }

    fn visit_str<E>(
        self,
        value: &str,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ServerType::from_str(value).map_err(|_| E::custom(format!("invalid server type: {value}")))
    }
}

impl<'de> Deserialize<'de> for ServerType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ServerTypeVisitor)
    }
}

// SQLx type mapping for ServerType
impl Type<Sqlite> for ServerType {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ServerType {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        <String as Encode<Sqlite>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for ServerType {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <String as Decode<Sqlite>>::decode(value)?;
        ServerType::from_str(&s).map_err(|e| Box::new(e) as BoxDynError)
    }
}

/// Transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportType {
    /// Streamable HTTP transport
    StreamableHttp,
    /// Server-Sent Events transport
    Sse,
    /// Standard input/output transport
    Stdio,
}

impl TransportType {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            TransportType::StreamableHttp => "StreamableHttp",
            TransportType::Sse => "Sse",
            TransportType::Stdio => "Stdio",
        }
    }

    /// Get the client-side representation (for JSON configs)
    pub fn client_format(&self) -> &'static str {
        match self {
            TransportType::Stdio => transport::STDIO,
            TransportType::Sse => transport::SSE,
            TransportType::StreamableHttp => transport::STREAMABLE_HTTP,
        }
    }

    /// Create from client format string (case-insensitive)
    pub fn from_client_format(s: &str) -> Result<Self, ParseTransportTypeError> {
        let lc = s.to_ascii_lowercase();
        match lc.as_str() {
            x if x == transport::STDIO => Ok(TransportType::Stdio),
            x if x == transport::SSE => Ok(TransportType::Sse),
            x if x == transport::STREAMABLE_HTTP => Ok(TransportType::StreamableHttp),
            _ => Err(ParseTransportTypeError),
        }
    }
}

impl fmt::Display for TransportType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type for TransportType parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseTransportTypeError;

impl fmt::Display for ParseTransportTypeError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid transport type")
    }
}

impl std::error::Error for ParseTransportTypeError {}

impl FromStr for TransportType {
    type Err = ParseTransportTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lc = s.to_ascii_lowercase();
        match lc.as_str() {
            "streamablehttp" | "streamable_http" => Ok(TransportType::StreamableHttp),
            "sse" => Ok(TransportType::Sse),
            "stdio" => Ok(TransportType::Stdio),
            _ => {
                tracing::error!(
                    "Invalid transport type '{}'. Allowed: 'Stdio'|'Sse'|'StreamableHttp' (case-insensitive)",
                    s
                );
                Err(ParseTransportTypeError)
            }
        }
    }
}

impl Serialize for TransportType {
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

struct TransportTypeVisitor;

impl<'de> Visitor<'de> for TransportTypeVisitor {
    type Value = TransportType;

    fn expecting(
        &self,
        formatter: &mut fmt::Formatter,
    ) -> fmt::Result {
        formatter.write_str("a string representing a transport type")
    }

    fn visit_str<E>(
        self,
        value: &str,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        TransportType::from_str(value).map_err(|_| E::custom(format!("invalid transport type: {value}")))
    }
}

impl<'de> Deserialize<'de> for TransportType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TransportTypeVisitor)
    }
}

// SQLx type mapping for TransportType
impl Type<Sqlite> for TransportType {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for TransportType {
    fn encode_by_ref(
        &self,
        buf: &mut Vec<SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, BoxDynError> {
        <String as Encode<Sqlite>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for TransportType {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let s = <String as Decode<Sqlite>>::decode(value)?;
        TransportType::from_str(&s).map_err(|e| Box::new(e) as BoxDynError)
    }
}

impl Default for TransportType {
    fn default() -> Self {
        Self::Sse
    }
}
