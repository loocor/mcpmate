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

/// Transport format constants for client configuration
/// These are the string representations used in client config files
pub mod transport_formats {
    /// Standard input/output transport format
    pub const STDIO: &str = "stdio";
    /// Server-Sent Events transport format
    pub const SSE: &str = "sse";
    /// Streamable HTTP transport format (client-side representation)
    pub const STREAMABLE_HTTP: &str = "streamableHttp";
}

/// Transport priority order for hosted mode selection
pub const TRANSPORT_PRIORITY: &[&str] = &[
    transport_formats::STREAMABLE_HTTP,
    transport_formats::SSE,
    transport_formats::STDIO,
];

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
            ServerType::Stdio => transport_formats::STDIO,
            ServerType::Sse => transport_formats::SSE,
            ServerType::StreamableHttp => transport_formats::STREAMABLE_HTTP,
        }
    }

    /// Create from client format string
    pub fn from_client_format(s: &str) -> Result<Self, ParseServerTypeError> {
        match s {
            s if s == transport_formats::STDIO => Ok(ServerType::Stdio),
            s if s == transport_formats::SSE => Ok(ServerType::Sse),
            s if s == transport_formats::STREAMABLE_HTTP => Ok(ServerType::StreamableHttp),
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
        match s.to_lowercase().as_str() {
            "stdio" => Ok(ServerType::Stdio),
            "sse" => Ok(ServerType::Sse),
            "streamable_http" | "streamablehttp" => Ok(ServerType::StreamableHttp),
            _ => Err(ParseServerTypeError),
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
            TransportType::Stdio => transport_formats::STDIO,
            TransportType::Sse => transport_formats::SSE,
            TransportType::StreamableHttp => transport_formats::STREAMABLE_HTTP,
        }
    }

    /// Create from client format string
    pub fn from_client_format(s: &str) -> Result<Self, ParseTransportTypeError> {
        match s {
            s if s == transport_formats::STDIO => Ok(TransportType::Stdio),
            s if s == transport_formats::SSE => Ok(TransportType::Sse),
            s if s == transport_formats::STREAMABLE_HTTP => Ok(TransportType::StreamableHttp),
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
        match s {
            "StreamableHttp" | "streamable_http" | "streamablehttp" => Ok(TransportType::StreamableHttp),
            "Sse" | "sse" => Ok(TransportType::Sse),
            "Stdio" | "stdio" => Ok(TransportType::Stdio),
            _ => Err(ParseTransportTypeError),
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
