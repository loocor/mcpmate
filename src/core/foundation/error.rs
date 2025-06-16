//! Core Error Types
//!
//! unified error handling module, providing error type definitions for the entire core system

use thiserror::Error;

/// unified error type for the core system
#[derive(Error, Debug)]
pub enum CoreError {
    /// error when connecting to an upstream server
    #[error("Connection error for server '{server_name}': {message}")]
    ConnectionError {
        /// server name
        server_name: String,
        /// error message
        message: String,
        /// source error (if available)
        #[source]
        source: Option<anyhow::Error>,
    },

    /// connection timeout error
    #[error("Connection timeout for server '{server_name}' after {seconds}s")]
    ConnectionTimeout {
        /// server name
        server_name: String,
        /// timeout seconds
        seconds: u64,
    },

    /// error when listing tools from an upstream server
    #[error("Failed to list tools from server '{server_name}': {message}")]
    ToolsError {
        /// server name
        server_name: String,
        /// error message
        message: String,
        /// source error (if available)
        #[source]
        source: Option<anyhow::Error>,
    },

    /// tools request timeout error
    #[error("Tools request timeout for server '{server_name}' after {seconds}s")]
    ToolsTimeout {
        /// server name
        server_name: String,
        /// timeout seconds
        seconds: u64,
    },

    /// error when a server is not found in the connection pool
    #[error("Server '{server_name}' not found in connection pool")]
    ServerNotFound {
        /// server name
        server_name: String,
    },

    /// unsupported server type error
    #[error("Unsupported server type: {server_type}")]
    UnsupportedServerType {
        /// server type
        server_type: String,
    },

    /// invalid state transition error
    #[error("Server '{server_name}' is already {state}")]
    InvalidStateTransition {
        /// server name
        server_name: String,
        /// current state
        state: String,
    },

    /// error when a required configuration field is missing
    #[error("Missing configuration for server '{server_name}': {field}")]
    MissingConfig {
        /// server name
        server_name: String,
        /// missing field
        field: String,
    },

    /// generic error, containing a message
    #[error("{message}")]
    GenericError {
        /// error message
        message: String,
        /// source error (if available)
        #[source]
        source: Option<anyhow::Error>,
    },
}

impl CoreError {
    /// create a new connection error
    pub fn connection_error(
        server_name: &str,
        message: &str,
        source: Option<anyhow::Error>,
    ) -> Self {
        Self::ConnectionError {
            server_name: server_name.to_string(),
            message: message.to_string(),
            source,
        }
    }

    /// create a new connection timeout error
    pub fn connection_timeout(
        server_name: &str,
        seconds: u64,
    ) -> Self {
        Self::ConnectionTimeout {
            server_name: server_name.to_string(),
            seconds,
        }
    }

    /// create a new tools error
    pub fn tools_error(
        server_name: &str,
        message: &str,
        source: Option<anyhow::Error>,
    ) -> Self {
        Self::ToolsError {
            server_name: server_name.to_string(),
            message: message.to_string(),
            source,
        }
    }

    /// create a new tools timeout error
    pub fn tools_timeout(
        server_name: &str,
        seconds: u64,
    ) -> Self {
        Self::ToolsTimeout {
            server_name: server_name.to_string(),
            seconds,
        }
    }

    /// create a new server not found error
    pub fn server_not_found(server_name: &str) -> Self {
        Self::ServerNotFound {
            server_name: server_name.to_string(),
        }
    }

    /// create a new unsupported server type error
    pub fn unsupported_server_type(server_type: &str) -> Self {
        Self::UnsupportedServerType {
            server_type: server_type.to_string(),
        }
    }

    /// create a new invalid state transition error
    pub fn invalid_state_transition(
        server_name: &str,
        state: &str,
    ) -> Self {
        Self::InvalidStateTransition {
            server_name: server_name.to_string(),
            state: state.to_string(),
        }
    }

    /// create a new missing configuration error
    pub fn missing_config(
        server_name: &str,
        field: &str,
    ) -> Self {
        Self::MissingConfig {
            server_name: server_name.to_string(),
            field: field.to_string(),
        }
    }

    /// create a new generic error
    pub fn generic_error(
        message: &str,
        source: Option<anyhow::Error>,
    ) -> Self {
        Self::GenericError {
            message: message.to_string(),
            source,
        }
    }
}

/// Core operation result type
pub type CoreResult<T> = std::result::Result<T, CoreError>;

// to maintain backward compatibility, re-export the original name
pub use CoreError as ProxyError;
pub use CoreResult as Result;
