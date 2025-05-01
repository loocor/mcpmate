// MCP Proxy types
// Contains shared type definitions for the MCP proxy server

use std::fmt;

/// Connection status for an upstream server
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Server is connected and operational
    Connected,
    /// Server is disconnected
    Disconnected,
    /// Server is in the process of connecting
    Connecting,
    /// Server connection failed with an error
    Failed(String),
    /// Server is manually disabled by user
    Disabled,
    /// Server is paused (temporarily disabled)
    Paused,
    /// Server is scheduled for reconnection
    Reconnecting,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Connected => write!(f, "Connected"),
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting"),
            ConnectionStatus::Failed(err) => write!(f, "Failed: {}", err),
            ConnectionStatus::Disabled => write!(f, "Disabled"),
            ConnectionStatus::Paused => write!(f, "Paused"),
            ConnectionStatus::Reconnecting => write!(f, "Reconnecting"),
        }
    }
}

impl ConnectionStatus {
    /// Check if the connection is in a state that allows connection attempts
    pub fn can_connect(&self) -> bool {
        match self {
            ConnectionStatus::Disconnected
            | ConnectionStatus::Failed(_)
            | ConnectionStatus::Reconnecting => true,
            _ => false,
        }
    }

    /// Check if the connection is in a state that should be monitored by health checks
    pub fn should_monitor(&self) -> bool {
        match self {
            ConnectionStatus::Connected | ConnectionStatus::Failed(_) => true,
            _ => false,
        }
    }
}
