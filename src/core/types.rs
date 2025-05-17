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
    Error(ErrorDetails),
    /// Server is shut down or disconnected
    Shutdown,
}

/// Detailed error information for connection errors
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorDetails {
    /// Error message
    pub message: String,
    /// Error type
    pub error_type: ErrorType,
    /// Number of consecutive failures
    pub failure_count: u32,
    /// First failure time (as seconds since UNIX epoch)
    pub first_failure_time: u64,
    /// Last failure time (as seconds since UNIX epoch)
    pub last_failure_time: u64,
}

/// Types of errors that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    /// Temporary error that can be retried
    Temporary,
    /// Permanent error that requires manual intervention
    Permanent,
    /// Unknown error type
    Unknown,
}

impl fmt::Display for ErrorType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            ErrorType::Temporary => write!(f, "Temporary"),
            ErrorType::Permanent => write!(f, "Permanent"),
            ErrorType::Unknown => write!(f, "Unknown"),
        }
    }
}

impl fmt::Display for ErrorDetails {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.error_type)
    }
}

impl fmt::Display for ConnectionStatus {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            ConnectionStatus::Initializing => write!(f, "Initializing"),
            ConnectionStatus::Ready => write!(f, "Ready"),
            ConnectionStatus::Busy => write!(f, "Busy"),
            ConnectionStatus::Error(err) => write!(f, "Error: {err}"),
            ConnectionStatus::Shutdown => write!(f, "Shutdown"),
        }
    }
}

impl ConnectionStatus {
    /// Check if the connection is in a state that allows connection attempts
    pub fn can_connect(&self) -> bool {
        matches!(
            self,
            ConnectionStatus::Shutdown | ConnectionStatus::Error(_)
        )
    }

    /// Check if the connection is in a state that should be monitored by health checks
    pub fn should_monitor(&self) -> bool {
        matches!(self, ConnectionStatus::Ready | ConnectionStatus::Error(_))
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
    pub fn can_perform_operation(
        &self,
        operation: &str,
    ) -> bool {
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
