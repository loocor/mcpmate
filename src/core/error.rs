// MCP Proxy error module
// Contains custom error types for the MCP proxy server

use thiserror::Error;

/// Custom error type for MCP proxy operations
#[derive(Error, Debug)]
pub enum ProxyError {
    /// Error when connecting to an upstream server
    #[error("Connection error for server '{server_name}': {message}")]
    ConnectionError {
        /// Name of the server
        server_name: String,
        /// Error message
        message: String,
        /// Source error if available
        #[source]
        source: Option<anyhow::Error>,
    },

    /// Error when a connection times out
    #[error("Connection timeout for server '{server_name}' after {seconds}s")]
    ConnectionTimeout {
        /// Name of the server
        server_name: String,
        /// Timeout in seconds
        seconds: u64,
    },

    /// Error when listing tools from an upstream server
    #[error("Failed to list tools from server '{server_name}': {message}")]
    ToolsError {
        /// Name of the server
        server_name: String,
        /// Error message
        message: String,
        /// Source error if available
        #[source]
        source: Option<anyhow::Error>,
    },

    /// Error when a tools request times out
    #[error("Tools request timeout for server '{server_name}' after {seconds}s")]
    ToolsTimeout {
        /// Name of the server
        server_name: String,
        /// Timeout in seconds
        seconds: u64,
    },

    /// Error when a server is not found in the connection pool
    #[error("Server '{server_name}' not found in connection pool")]
    ServerNotFound {
        /// Name of the server
        server_name: String,
    },

    /// Error when a server type is not supported
    #[error("Unsupported server type: {server_type}")]
    UnsupportedServerType {
        /// Type of the server
        server_type: String,
    },

    /// Error when a server is already in the requested state
    #[error("Server '{server_name}' is already {state}")]
    InvalidStateTransition {
        /// Name of the server
        server_name: String,
        /// Current state
        state: String,
    },

    /// Error when a required configuration field is missing
    #[error("Missing configuration for server '{server_name}': {field}")]
    MissingConfig {
        /// Name of the server
        server_name: String,
        /// Missing field
        field: String,
    },

    /// Generic error with a message
    #[error("{message}")]
    GenericError {
        /// Error message
        message: String,
        /// Source error if available
        #[source]
        source: Option<anyhow::Error>,
    },
}

impl ProxyError {
    /// Create a new connection error
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

    /// Create a new connection timeout error
    pub fn connection_timeout(server_name: &str, seconds: u64) -> Self {
        Self::ConnectionTimeout {
            server_name: server_name.to_string(),
            seconds,
        }
    }

    /// Create a new tools error
    pub fn tools_error(server_name: &str, message: &str, source: Option<anyhow::Error>) -> Self {
        Self::ToolsError {
            server_name: server_name.to_string(),
            message: message.to_string(),
            source,
        }
    }

    /// Create a new tools timeout error
    pub fn tools_timeout(server_name: &str, seconds: u64) -> Self {
        Self::ToolsTimeout {
            server_name: server_name.to_string(),
            seconds,
        }
    }

    /// Create a new server not found error
    pub fn server_not_found(server_name: &str) -> Self {
        Self::ServerNotFound {
            server_name: server_name.to_string(),
        }
    }

    /// Create a new unsupported server type error
    pub fn unsupported_server_type(server_type: &str) -> Self {
        Self::UnsupportedServerType {
            server_type: server_type.to_string(),
        }
    }

    /// Create a new invalid state transition error
    pub fn invalid_state_transition(server_name: &str, state: &str) -> Self {
        Self::InvalidStateTransition {
            server_name: server_name.to_string(),
            state: state.to_string(),
        }
    }

    /// Create a new missing config error
    pub fn missing_config(server_name: &str, field: &str) -> Self {
        Self::MissingConfig {
            server_name: server_name.to_string(),
            field: field.to_string(),
        }
    }

    /// Create a new generic error
    pub fn generic_error(message: &str, source: Option<anyhow::Error>) -> Self {
        Self::GenericError {
            message: message.to_string(),
            source,
        }
    }
}

/// Result type for MCP proxy operations
pub type Result<T> = std::result::Result<T, ProxyError>;
