// MCP Proxy types
// Contains shared type definitions for the MCP proxy server

use std::fmt;

/// Connection status for an upstream server
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Server is initializing or in the process of connecting
    Initializing,
    /// Server is connected and ready to receive requests
    Ready,
    /// Server is processing a request
    Busy,
    /// Server encountered an error
    Error(String),
    /// Server is shut down or disconnected
    Shutdown,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Initializing => write!(f, "Initializing"),
            ConnectionStatus::Ready => write!(f, "Ready"),
            ConnectionStatus::Busy => write!(f, "Busy"),
            ConnectionStatus::Error(err) => write!(f, "Error: {}", err),
            ConnectionStatus::Shutdown => write!(f, "Shutdown"),
        }
    }
}

impl ConnectionStatus {
    /// Check if the connection is in a state that allows connection attempts
    pub fn can_connect(&self) -> bool {
        match self {
            ConnectionStatus::Shutdown | ConnectionStatus::Error(_) => true,
            _ => false,
        }
    }

    /// Check if the connection is in a state that should be monitored by health checks
    pub fn should_monitor(&self) -> bool {
        match self {
            ConnectionStatus::Ready | ConnectionStatus::Error(_) => true,
            _ => false,
        }
    }

    /// Get the allowed operations for this status
    pub fn allowed_operations(&self) -> Vec<&'static str> {
        let mut ops = vec!["disconnect", "reconnect"]; // Most states share these operations

        match self {
            Self::Initializing => {
                ops.push("cancel"); // Can cancel initialization
            }
            Self::Ready => {
                // No special operations
            }
            Self::Busy => {
                // No special operations
            }
            Self::Error(_) => {
                // No special operations
            }
            Self::Shutdown => {
                ops.clear(); // Clear shared operations
                ops.push("reconnect"); // Only reconnect is allowed
            }
        }

        ops
    }

    /// Check if a specific operation is allowed in the current state
    pub fn can_perform_operation(&self, operation: &str) -> bool {
        self.allowed_operations().contains(&operation)
    }

    /// Check if force disconnect is allowed
    pub fn can_force_disconnect(&self) -> bool {
        // Force disconnect is allowed in all states except Shutdown
        !matches!(self, Self::Shutdown)
    }

    /// Check if reset reconnect is allowed
    pub fn can_reset_reconnect(&self) -> bool {
        // Reset reconnect is allowed in all states
        true
    }
}
