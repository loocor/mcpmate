// Server models for MCPMate
// Contains data models for server configuration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Server configuration model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Server {
    /// Unique ID
    pub id: Option<String>,
    /// Name of the server
    pub name: String,
    /// Type of the server (stdio, sse, streamable_http)
    pub server_type: String,
    /// Command to execute (for stdio servers)
    pub command: Option<String>,
    /// URL (for sse and streamable_http servers)
    pub url: Option<String>,
    /// Transport type
    pub transport_type: Option<String>,
    /// When the configuration was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the configuration was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

impl Server {
    /// Create a new server configuration
    pub fn new(
        name: String,
        server_type: String,
    ) -> Self {
        Self {
            id: None,
            name,
            server_type,
            command: None,
            url: None,
            transport_type: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Create a new stdio server configuration
    pub fn new_stdio(
        name: String,
        command: Option<String>,
    ) -> Self {
        Self {
            id: None,
            name,
            server_type: "stdio".to_string(),
            command,
            url: None,
            transport_type: Some("Stdio".to_string()),
            created_at: None,
            updated_at: None,
        }
    }

    /// Create a new SSE server configuration
    pub fn new_sse(
        name: String,
        url: Option<String>,
    ) -> Self {
        Self {
            id: None,
            name,
            server_type: "sse".to_string(),
            command: None,
            url,
            transport_type: Some("Sse".to_string()),
            created_at: None,
            updated_at: None,
        }
    }

    /// Create a new Streamable HTTP server configuration
    pub fn new_streamable_http(
        name: String,
        url: Option<String>,
    ) -> Self {
        Self {
            id: None,
            name,
            server_type: "streamable_http".to_string(),
            command: None,
            url,
            transport_type: Some("StreamableHttp".to_string()),
            created_at: None,
            updated_at: None,
        }
    }
}

/// Server argument model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServerArg {
    /// Unique ID
    pub id: Option<String>,
    /// Server ID
    pub server_id: String,
    /// Argument index
    pub arg_index: i32,
    /// Argument value
    pub arg_value: String,
}

/// Server environment variable model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServerEnv {
    /// Unique ID
    pub id: Option<String>,
    /// Server ID
    pub server_id: String,
    /// Environment variable key
    pub env_key: String,
    /// Environment variable value
    pub env_value: String,
}

/// Server metadata model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServerMeta {
    /// Unique ID
    pub id: Option<String>,
    /// Server ID
    pub server_id: String,
    /// Description of the server
    pub description: Option<String>,
    /// Author of the server
    pub author: Option<String>,
    /// Website of the server
    pub website: Option<String>,
    /// Repository URL of the server
    pub repository: Option<String>,
    /// Category of the server
    pub category: Option<String>,
    /// Recommended scenario for the server
    pub recommended_scenario: Option<String>,
    /// Rating of the server (1-5)
    pub rating: Option<i32>,
    /// When the metadata was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the metadata was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

impl ServerMeta {
    /// Create new server metadata
    pub fn new(server_id: String) -> Self {
        Self {
            id: None,
            server_id,
            description: None,
            author: None,
            website: None,
            repository: None,
            category: None,
            recommended_scenario: None,
            rating: None,
            created_at: None,
            updated_at: None,
        }
    }
}
