// Server information loader for configuration generation
// Handles loading server data from database

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;

use crate::common::profile::defaults;

use crate::config::client::models::GenerationRequest;

/// Server information for configuration generation
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub command: Option<String>,
    pub url: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub runtime: String,
    pub server_type: String,
}

/// Server information loader
pub struct ServerLoader {
    db_pool: Arc<SqlitePool>,
}

impl ServerLoader {
    /// Create a new server loader
    pub fn new(db_pool: Arc<SqlitePool>) -> Self {
        Self { db_pool }
    }

    /// Get servers for configuration generation based on request
    pub async fn get_servers_for_generation(
        &self,
        request: &GenerationRequest,
    ) -> Result<Vec<ServerInfo>> {
        // If specific servers are requested, use those
        if let Some(server_ids) = &request.servers {
            return self.get_servers_by_ids(server_ids).await;
        }

        // If profile_id is provided, get servers from that profile
        if let Some(profile_id) = &request.profile_id {
            return self.get_servers_by_profile(profile_id).await;
        }

        // Default: get all enabled servers
        self.get_all_enabled_servers().await
    }

    /// Get servers by their IDs
    pub async fn get_servers_by_ids(
        &self,
        server_ids: &[String],
    ) -> Result<Vec<ServerInfo>> {
        let mut servers = Vec::new();
        for server_id in server_ids {
            if let Ok(server) = self.get_server_by_id(server_id).await {
                servers.push(server);
            }
        }
        Ok(servers)
    }

    /// Get servers by profile ID
    pub async fn get_servers_by_profile(
        &self,
        profile_id: &str,
    ) -> Result<Vec<ServerInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT cs.server_id, cs.server_name, cs.enabled,
                   s.command, s.url, s.server_type, s.transport_type
            FROM profile_server cs
            JOIN server_config s ON cs.server_id = s.id
            WHERE cs.profile_id = ? AND cs.enabled = TRUE AND s.enabled = TRUE
            ORDER BY cs.server_name
            "#,
        )
        .bind(profile_id)
        .fetch_all(self.db_pool.as_ref())
        .await?;

        let mut servers = Vec::new();
        for row in rows {
            let server_id: String = row.get("server_id");

            // Load args from server_args table
            let args = self.load_server_args(&server_id).await?;

            // Load env from server_env table
            let env = self.load_server_env(&server_id).await?;

            servers.push(ServerInfo {
                id: server_id,
                name: row.get("server_name"),
                command: row.get::<Option<String>, _>("command"),
                url: row.get::<Option<String>, _>("url"),
                args,
                env,
                runtime: defaults::RUNTIME.to_string(), // TODO: Load from appropriate table if needed
                server_type: row.get::<Option<String>, _>("server_type").unwrap_or_else(|| {
                    use crate::common::constants::transport;
                    transport::STDIO.to_string()
                }),
            });
        }
        Ok(servers)
    }

    /// Get all enabled servers
    pub async fn get_all_enabled_servers(&self) -> Result<Vec<ServerInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, command, url, server_type, transport_type
            FROM server_config
            WHERE enabled = TRUE
            ORDER BY name
            "#,
        )
        .fetch_all(self.db_pool.as_ref())
        .await?;

        let mut servers = Vec::new();
        for row in rows {
            let server_id: String = row.get("id");

            // Load args from server_args table
            let args = self.load_server_args(&server_id).await?;

            // Load env from server_env table
            let env = self.load_server_env(&server_id).await?;

            servers.push(ServerInfo {
                id: server_id,
                name: row.get("name"),
                command: row.get::<Option<String>, _>("command"),
                url: row.get::<Option<String>, _>("url"),
                args,
                env,
                runtime: defaults::RUNTIME.to_string(), // TODO: Load from appropriate table if needed
                server_type: row.get::<Option<String>, _>("server_type").unwrap_or_else(|| {
                    use crate::common::constants::transport;
                    transport::STDIO.to_string()
                }),
            });
        }
        Ok(servers)
    }

    /// Get server by ID
    pub async fn get_server_by_id(
        &self,
        server_id: &str,
    ) -> Result<ServerInfo> {
        let row = sqlx::query(
            r#"
            SELECT id, name, command, url, server_type, transport_type
            FROM server_config
            WHERE id = ?
            "#,
        )
        .bind(server_id)
        .fetch_one(self.db_pool.as_ref())
        .await?;

        let server_id: String = row.get("id");

        // Load args from server_args table
        let args = self.load_server_args(&server_id).await?;

        // Load env from server_env table
        let env = self.load_server_env(&server_id).await?;

        Ok(ServerInfo {
            id: server_id,
            name: row.get("name"),
            command: row.get::<Option<String>, _>("command"),
            url: row.get::<Option<String>, _>("url"),
            args,
            env,
            runtime: defaults::RUNTIME.to_string(), // TODO: Load from appropriate table if needed
            server_type: row.get::<Option<String>, _>("server_type").unwrap_or_else(|| {
                use crate::common::constants::transport;
                transport::STDIO.to_string()
            }),
        })
    }

    /// Load server arguments from server_args table
    async fn load_server_args(
        &self,
        server_id: &str,
    ) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT arg_value
            FROM server_args
            WHERE server_id = ?
            ORDER BY arg_index ASC
            "#,
        )
        .bind(server_id)
        .fetch_all(self.db_pool.as_ref())
        .await?;

        Ok(rows.into_iter().map(|row| row.get("arg_value")).collect())
    }

    /// Load server environment variables from server_env table
    async fn load_server_env(
        &self,
        server_id: &str,
    ) -> Result<HashMap<String, String>> {
        let rows = sqlx::query(
            r#"
            SELECT env_key, env_value
            FROM server_env
            WHERE server_id = ?
            "#,
        )
        .bind(server_id)
        .fetch_all(self.db_pool.as_ref())
        .await?;

        let mut env = HashMap::new();
        for row in rows {
            env.insert(row.get("env_key"), row.get("env_value"));
        }
        Ok(env)
    }
}
