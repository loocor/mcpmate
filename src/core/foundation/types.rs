//! Core Core Types
//!
//! core type definitions for the core system

use std::fmt;

/// connection status of an upstream server
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// server is initializing or in the process of connecting
    Initializing,
    /// server is connected and ready to receive requests
    Ready,
    /// server is processing requests
    Busy,
    /// server encountered an error
    Error(ErrorDetails),
    /// server is closed or disconnected
    Shutdown,
}

/// detailed information about a connection error
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorDetails {
    /// error message
    pub message: String,
    /// error type
    pub error_type: ErrorType,
    /// number of consecutive failures
    pub failure_count: u32,
    /// first failure time (UNIX timestamp in seconds)
    pub first_failure_time: u64,
    /// last failure time (UNIX timestamp in seconds)
    pub last_failure_time: u64,
}

/// possible error types
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    /// temporary error that can be retried
    Temporary,
    /// permanent error that requires manual intervention
    Permanent,
    /// unknown error type
    Unknown,
}

/// Connection operations that can be performed on a connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionOperation {
    /// Connect to the server
    Connect,
    /// Disconnect from the server
    Disconnect,
    /// Force disconnect (immediate)
    ForceDisconnect,
    /// Reconnect to the server
    Reconnect,
    /// Cancel current operation
    Cancel,
    /// Reset and reconnect
    ResetReconnect,
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
    /// check if the connection is in an allowed connection attempt state
    pub fn can_connect(&self) -> bool {
        matches!(
            self,
            ConnectionStatus::Shutdown | ConnectionStatus::Error(_)
        )
    }

    /// check if the connection should be monitored by health check
    pub fn should_monitor(&self) -> bool {
        matches!(self, ConnectionStatus::Ready | ConnectionStatus::Error(_))
    }

    /// get the list of allowed operations in this state
    pub fn allowed_operations(&self) -> Vec<ConnectionOperation> {
        match self {
            Self::Initializing => {
                vec![ConnectionOperation::Cancel, ConnectionOperation::Disconnect]
            }
            Self::Ready => {
                vec![
                    ConnectionOperation::Disconnect,
                    ConnectionOperation::Reconnect,
                ]
            }
            Self::Busy => {
                vec![
                    ConnectionOperation::Disconnect,
                    ConnectionOperation::Reconnect,
                ]
            }
            Self::Error(_) => {
                vec![
                    ConnectionOperation::Connect,
                    ConnectionOperation::Disconnect,
                    ConnectionOperation::Reconnect,
                ]
            }
            Self::Shutdown => {
                vec![ConnectionOperation::Connect, ConnectionOperation::Reconnect]
            }
        }
    }

    /// check if a specific operation is allowed in the current state (type-safe version)
    pub fn can_perform_operation(
        &self,
        operation: ConnectionOperation,
    ) -> bool {
        use ConnectionOperation::*;

        match (self, operation) {
            // force disconnect is allowed in all states except Shutdown
            (Self::Shutdown, ForceDisconnect) => false,
            (_, ForceDisconnect) => true,

            // reset reconnect is allowed in all states
            (_, ResetReconnect) => true,

            // standard operation - check allowed operations list
            (_, op) => self.allowed_operations().contains(&op),
        }
    }

    /// check if force disconnect is allowed
    /// deprecated: use can_perform_operation(ConnectionOperation::ForceDisconnect) instead
    #[deprecated(
        since = "0.1.0",
        note = "Use can_perform_operation(ConnectionOperation::ForceDisconnect) instead"
    )]
    pub fn can_force_disconnect(&self) -> bool {
        self.can_perform_operation(ConnectionOperation::ForceDisconnect)
    }

    /// check if reset reconnect is allowed
    /// deprecated: use can_perform_operation(ConnectionOperation::ResetReconnect) instead
    #[deprecated(
        since = "0.1.0",
        note = "Use can_perform_operation(ConnectionOperation::ResetReconnect) instead"
    )]
    pub fn can_reset_reconnect(&self) -> bool {
        self.can_perform_operation(ConnectionOperation::ResetReconnect)
    }
}

impl std::fmt::Display for ConnectionOperation {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ConnectionOperation::Connect => write!(f, "connect"),
            ConnectionOperation::Disconnect => write!(f, "disconnect"),
            ConnectionOperation::ForceDisconnect => write!(f, "force_disconnect"),
            ConnectionOperation::Reconnect => write!(f, "reconnect"),
            ConnectionOperation::Cancel => write!(f, "cancel"),
            ConnectionOperation::ResetReconnect => write!(f, "reset_reconnect"),
        }
    }
}

impl std::str::FromStr for ConnectionOperation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "connect" => Ok(ConnectionOperation::Connect),
            "disconnect" => Ok(ConnectionOperation::Disconnect),
            "force_disconnect" | "force-disconnect" => Ok(ConnectionOperation::ForceDisconnect),
            "reconnect" => Ok(ConnectionOperation::Reconnect),
            "cancel" => Ok(ConnectionOperation::Cancel),
            "reset_reconnect" | "reset-reconnect" => Ok(ConnectionOperation::ResetReconnect),
            _ => Err(format!("Invalid connection operation: {}", s)),
        }
    }
}
