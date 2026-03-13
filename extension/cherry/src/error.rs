use std::fmt;

/// Result type alias for this crate
pub type Result<T> = std::result::Result<T, CherryDbError>;

/// Error types for Cherry DB operations
#[derive(Debug)]
pub enum CherryDbError {
    /// Database operation failed
    DatabaseError(String),
    /// JSON parsing failed
    JsonError(String),
    /// UTF-16 decoding failed
    EncodingError(String),
    /// Configuration not found
    ConfigNotFound,
    /// Invalid database path
    InvalidPath(String),
    /// Server not found
    ServerNotFound(String),
    /// Invalid server configuration
    InvalidServer(String),
}

impl fmt::Display for CherryDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CherryDbError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            CherryDbError::JsonError(msg) => write!(f, "JSON error: {}", msg),
            CherryDbError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
            CherryDbError::ConfigNotFound => write!(f, "MCP configuration not found"),
            CherryDbError::InvalidPath(path) => write!(f, "Invalid database path: {}", path),
            CherryDbError::ServerNotFound(id) => write!(f, "Server not found: {}", id),
            CherryDbError::InvalidServer(msg) => write!(f, "Invalid server configuration: {}", msg),
        }
    }
}

impl std::error::Error for CherryDbError {}

impl From<serde_json::Error> for CherryDbError {
    fn from(err: serde_json::Error) -> Self {
        CherryDbError::JsonError(err.to_string())
    }
}
