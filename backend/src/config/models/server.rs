// Server models for MCPMate
// Contains data models for server configuration

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};

use crate::common::{
    constants::database::{columns, tables},
    server::ServerType,
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
    /// Registry server id (from official registry)
    pub registry_server_id: Option<String>,
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
        tables::SERVER_CONFIG
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

            registry_server_id: None,
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

            registry_server_id: None,
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

            registry_server_id: None,
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
            headers: None,
        }
    }

    /// find server config by name
    pub async fn find_by_name(
        pool: &Pool<Sqlite>,
        name: &str,
    ) -> Result<Option<Self>> {
        use sqlx::query_as;

        let server = query_as(&format!(
            "SELECT * FROM {} WHERE {} = ?",
            tables::SERVER_CONFIG,
            columns::NAME
        ))
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

        let servers = query_as(&format!(
            "SELECT * FROM {} WHERE {} = ?",
            tables::SERVER_CONFIG,
            columns::SERVER_TYPE
        ))
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
    /// Website of the server (registry `websiteUrl`)
    pub website: Option<String>,
    /// Repository payload serialized as JSON
    pub repository: Option<String>,
    /// Registry-declared version string
    pub registry_version: Option<String>,
    /// Serialized registry `_meta` block (namespaced metadata)
    pub registry_meta_json: Option<String>,
    /// Raw extras JSON (e.g., MCPB manifest contents)
    pub extras_json: Option<String>,
    /// Legacy author field (kept for backward compatibility)
    pub author: Option<String>,
    /// Legacy category field (kept for backward compatibility)
    pub category: Option<String>,
    /// Legacy recommended scenario field (kept for backward compatibility)
    pub recommended_scenario: Option<String>,
    /// Legacy rating field (kept for backward compatibility)
    pub rating: Option<i32>,
    /// JSON-serialized list of server icons (rmcp::model::Icon)
    pub icons_json: Option<String>,
    /// Upstream server version (from Implementation.version)
    pub server_version: Option<String>,
    /// MCP protocol version advertised by the server
    pub protocol_version: Option<String>,
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
            server_version: None,
            author: None,
            category: None,
            description: None,
            extras_json: None,
            icons_json: None,
            protocol_version: None,
            rating: None,
            recommended_scenario: None,
            registry_meta_json: None,
            registry_version: None,
            repository: None,
            website: None,
            created_at: None,
            updated_at: None,
        }
    }
}
