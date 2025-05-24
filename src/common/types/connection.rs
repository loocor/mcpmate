//! Connection-related types for MCPMate
//!
//! This module contains types related to connection operations and management.

use std::{fmt, str::FromStr};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, Visitor},
};

/// Connection operations that can be performed on server instances
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionOperation {
    /// Disconnect from server
    Disconnect,
    /// Force disconnect from server (ignores current state)
    ForceDisconnect,
    /// Reconnect to server (disconnect + connect with backoff)
    Reconnect,
    /// Cancel current connection/operation
    Cancel,
    /// Reset reconnection counter and reconnect
    ResetReconnect,
}

impl ConnectionOperation {
    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionOperation::Disconnect => "disconnect",
            ConnectionOperation::ForceDisconnect => "force_disconnect",
            ConnectionOperation::Reconnect => "reconnect",
            ConnectionOperation::Cancel => "cancel",
            ConnectionOperation::ResetReconnect => "reset_reconnect",
        }
    }

    /// Check if this operation requires special handling
    pub fn is_force_operation(&self) -> bool {
        matches!(self, ConnectionOperation::ForceDisconnect)
    }

    /// Check if this operation involves reconnection
    pub fn involves_reconnection(&self) -> bool {
        matches!(
            self,
            ConnectionOperation::Reconnect | ConnectionOperation::ResetReconnect
        )
    }
}

impl fmt::Display for ConnectionOperation {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error type for ConnectionOperation parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseConnectionOperationError;

impl fmt::Display for ParseConnectionOperationError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "invalid connection operation")
    }
}

impl std::error::Error for ParseConnectionOperationError {}

impl FromStr for ConnectionOperation {
    type Err = ParseConnectionOperationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "disconnect" => Ok(ConnectionOperation::Disconnect),
            "force_disconnect" => Ok(ConnectionOperation::ForceDisconnect),
            "reconnect" => Ok(ConnectionOperation::Reconnect),
            "cancel" => Ok(ConnectionOperation::Cancel),
            "reset_reconnect" => Ok(ConnectionOperation::ResetReconnect),
            _ => Err(ParseConnectionOperationError),
        }
    }
}

impl Serialize for ConnectionOperation {
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

struct ConnectionOperationVisitor;

impl<'de> Visitor<'de> for ConnectionOperationVisitor {
    type Value = ConnectionOperation;

    fn expecting(
        &self,
        formatter: &mut fmt::Formatter,
    ) -> fmt::Result {
        formatter.write_str("a string representing a connection operation")
    }

    fn visit_str<E>(
        self,
        value: &str,
    ) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        ConnectionOperation::from_str(value)
            .map_err(|_| E::custom(format!("invalid connection operation: {value}")))
    }
}

impl<'de> Deserialize<'de> for ConnectionOperation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ConnectionOperationVisitor)
    }
}
