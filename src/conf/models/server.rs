// Server models for MCPMate
// Contains data models for server configuration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::common::types::{EnabledStatus, ServerType, TransportType};

/// Server configuration model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Server {
    /// Unique ID
    pub id: Option<String>,
    /// Name of the server
    pub name: String,
    /// Type of the server (stdio, sse, streamable_http)
    pub server_type: ServerType,
    /// Command to execute (for stdio servers)
    pub command: Option<String>,
    /// URL (for sse and streamable_http servers)
    pub url: Option<String>,
    /// Transport type
    pub transport_type: Option<TransportType>,
    /// Whether the server is globally enabled
    pub enabled: EnabledStatus,
    /// When the configuration was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the configuration was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

impl Server {
    /// Get the server type (for backward compatibility)
    pub fn get_server_type(&self) -> ServerType {
        self.server_type
    }

    /// Get the transport type (for backward compatibility)
    pub fn get_transport_type(&self) -> Option<TransportType> {
        self.transport_type
    }

    /// Get the enabled status (for backward compatibility)
    pub fn get_enabled_status(&self) -> EnabledStatus {
        self.enabled
    }

    /// Set the server type
    pub fn set_server_type(
        &mut self,
        server_type: ServerType,
    ) {
        self.server_type = server_type;
    }

    /// Set the transport type
    pub fn set_transport_type(
        &mut self,
        transport_type: Option<TransportType>,
    ) {
        self.transport_type = transport_type;
    }

    /// Set the enabled status
    pub fn set_enabled_status(
        &mut self,
        enabled: EnabledStatus,
    ) {
        self.enabled = enabled;
    }

    /// Get server type as string (for API compatibility)
    pub fn server_type_string(&self) -> String {
        self.server_type.to_string()
    }

    /// Get transport type as string (for API compatibility)
    pub fn transport_type_string(&self) -> Option<String> {
        self.transport_type.map(|t| t.to_string())
    }

    /// Get enabled as boolean (for API compatibility)
    pub fn enabled_bool(&self) -> Option<bool> {
        Some(self.enabled.as_bool())
    }
}

impl Server {
    /// Create a new server configuration
    pub fn new(
        name: String,
        server_type: ServerType,
    ) -> Self {
        Self {
            id: None,
            name,
            server_type,
            command: None,
            url: None,
            transport_type: None,
            enabled: EnabledStatus::Enabled, // Default to enabled
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
            server_type: ServerType::Stdio,
            command,
            url: None,
            transport_type: Some(TransportType::Stdio),
            enabled: EnabledStatus::Enabled, // Default to enabled
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
            server_type: ServerType::Sse,
            command: None,
            url,
            transport_type: Some(TransportType::Sse),
            enabled: EnabledStatus::Enabled, // Default to enabled
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
            server_type: ServerType::StreamableHttp,
            command: None,
            url,
            transport_type: Some(TransportType::StreamableHttp),
            enabled: EnabledStatus::Enabled, // Default to enabled
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
