// Server models for MCPMate
// Contains data models for server configuration

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};

use crate::common::{
    server::{ServerType, TransportType},
    status::EnabledStatus,
};
use crate::macros::entity::DatabaseEntity;

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
    /// Capabilities list string (e.g., "tools,prompts,resources")
    pub capabilities: Option<String>,
    /// Whether the server is globally enabled
    pub enabled: EnabledStatus,
    /// When the configuration was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the configuration was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl DatabaseEntity for Server {
    fn table_name() -> &'static str {
        "server_config"
    }

    fn get_id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(
        &mut self,
        id: String,
    ) {
        self.id = Some(id);
    }

    fn get_created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    fn set_created_at(
        &mut self,
        time: DateTime<Utc>,
    ) {
        self.created_at = Some(time);
    }

    fn get_updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }

    fn set_updated_at(
        &mut self,
        time: DateTime<Utc>,
    ) {
        self.updated_at = Some(time);
    }

    async fn find_by(
        pool: &Pool<Sqlite>,
        conditions: &str, // Example: "name = 'my_server' AND server_type = 'Stdio'"
    ) -> Result<Vec<Self>> {
        // Ensure Self: Sized + Send + Unpin + for<'r> FromRow<'r, sqlx::sqlite::SqliteRow> is met
        let query_string = format!("SELECT * FROM {} WHERE {}", Self::table_name(), conditions);
        let servers = sqlx::query_as(&query_string).fetch_all(pool).await?;
        Ok(servers)
    }
}

impl Server {
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
            capabilities: None,
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
            capabilities: None,
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
            capabilities: None,
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
            capabilities: None,
            enabled: EnabledStatus::Enabled, // Default to enabled
            created_at: None,
            updated_at: None,
        }
    }

    /// Helper: check capability by enum token (single public entrypoint)
    pub fn has_capability(
        &self,
        token: crate::common::capability::CapabilityToken,
    ) -> bool {
        if let Some(ref caps) = self.capabilities {
            let token_lower = token.as_str().to_ascii_lowercase();
            caps.split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .any(|t| t == token_lower)
        } else {
            false
        }
    }

    /// Helper: set capabilities from tokens
    pub fn set_capabilities_from_tokens<T: AsRef<str>>(
        &mut self,
        tokens: &[T],
    ) {
        let list: Vec<String> = tokens
            .iter()
            .map(|t| t.as_ref().trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        if list.is_empty() {
            self.capabilities = None;
        } else {
            self.capabilities = Some(list.join(","));
        }
    }

    /// Convert to MCPServerConfig for capability discovery and connection
    /// This eliminates duplicate config construction code across the codebase
    pub fn to_mcp_config(&self) -> crate::core::models::MCPServerConfig {
        crate::core::models::MCPServerConfig {
            kind: self.server_type,
            command: self.command.clone(),
            url: self.url.clone(),
            args: None, // Args are loaded separately when needed
            env: None,  // Env vars are loaded separately when needed
            transport_type: self.transport_type,
        }
    }

    /// find server config by name
    pub async fn find_by_name(
        pool: &Pool<Sqlite>,
        name: &str,
    ) -> Result<Option<Self>> {
        use sqlx::query_as;

        let server = query_as("SELECT * FROM server_config WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await?;

        Ok(server)
    }

    /// find server config by type
    pub async fn find_by_type(
        pool: &Pool<Sqlite>,
        server_type: ServerType,
    ) -> Result<Vec<Self>> {
        use sqlx::query_as;

        let servers = query_as("SELECT * FROM server_config WHERE server_type = ?")
            .bind(server_type)
            .fetch_all(pool)
            .await?;

        Ok(servers)
    }

    /// save server config
    pub async fn save(
        &mut self,
        pool: &Pool<Sqlite>,
    ) -> Result<()> {
        if self.id.is_none() {
            self.create(pool).await
        } else {
            self.update(pool).await
        }
    }

    /// delete server config
    pub async fn delete(
        &self,
        pool: &Pool<Sqlite>,
    ) -> Result<()> {
        DatabaseEntity::delete(self, pool).await
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
