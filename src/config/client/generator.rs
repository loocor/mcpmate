// Configuration generator for client applications
// Generates client-specific configurations based on rules and server data

use crate::config::client::models::*;
use anyhow::Result;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration generator for client applications
pub struct ConfigGenerator {
    db_pool: Arc<SqlitePool>,
}

impl ConfigGenerator {
    /// Create a new configuration generator
    pub fn new(db_pool: Arc<SqlitePool>) -> Self {
        Self { db_pool }
    }

    /// Generate configuration for a specific client
    pub async fn generate_config(
        &self,
        request: &GenerationRequest,
    ) -> Result<GeneratedConfig> {
        // Get client configuration rule
        let config_rule = self.get_config_rule(&request.client_identifier).await?;

        // Get servers to include in configuration
        let servers = self.get_servers_for_generation(request).await?;

        // Generate configuration content based on client rules
        let config_content = self
            .generate_config_content(&config_rule, &servers, request)
            .await?;

        // Get config path for the client
        let config_path = self
            .get_client_config_path(&request.client_identifier)
            .await?;

        Ok(GeneratedConfig {
            client_identifier: request.client_identifier.clone(),
            mode: request.mode.clone(),
            config_content,
            config_path,
            backup_needed: true,
            preview_only: false,
        })
    }

    /// Generate preview configuration (same as generate but marked as preview)
    pub async fn generate_preview(
        &self,
        request: &GenerationRequest,
    ) -> Result<GeneratedConfig> {
        let mut config = self.generate_config(request).await?;
        config.preview_only = true;
        config.backup_needed = false;
        Ok(config)
    }

    /// Get configuration rule for a client
    async fn get_config_rule(
        &self,
        client_identifier: &str,
    ) -> Result<ConfigRule> {
        let row = sqlx::query(
            r#"
            SELECT id, client_app_id, client_identifier, top_level_key, is_mixed_config,
                   supported_transports, supported_runtimes, format_rules, security_features
            FROM client_config_rules
            WHERE client_identifier = ?
            "#,
        )
        .bind(client_identifier)
        .fetch_one(self.db_pool.as_ref())
        .await?;

        let supported_transports: Vec<String> =
            serde_json::from_str(&row.get::<String, _>("supported_transports"))?;
        let supported_runtimes: HashMap<String, Vec<String>> =
            serde_json::from_str(&row.get::<String, _>("supported_runtimes"))?;
        let format_rules: HashMap<String, FormatRule> =
            serde_json::from_str(&row.get::<String, _>("format_rules"))?;
        let security_features: Option<SecurityFeatures> = row
            .get::<Option<String>, _>("security_features")
            .map(|s| serde_json::from_str(&s))
            .transpose()?;

        Ok(ConfigRule {
            id: row.get("id"),
            client_app_id: row.get("client_app_id"),
            client_identifier: row.get("client_identifier"),
            top_level_key: row.get("top_level_key"),
            is_mixed_config: row.get("is_mixed_config"),
            supported_transports,
            supported_runtimes,
            format_rules,
            security_features,
        })
    }

    /// Get servers for configuration generation
    async fn get_servers_for_generation(
        &self,
        request: &GenerationRequest,
    ) -> Result<Vec<ServerInfo>> {
        // If specific servers are requested, use those
        if let Some(server_ids) = &request.servers {
            return self.get_servers_by_ids(server_ids).await;
        }

        // If config_suit_id is provided, get servers from that suit
        if let Some(config_suit_id) = &request.config_suit_id {
            return self.get_servers_by_config_suit(config_suit_id).await;
        }

        // Default: get all enabled servers
        self.get_all_enabled_servers().await
    }

    /// Get servers by their IDs
    async fn get_servers_by_ids(
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

    /// Get servers by config suit ID
    async fn get_servers_by_config_suit(
        &self,
        config_suit_id: &str,
    ) -> Result<Vec<ServerInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT cs.server_id, cs.server_name, cs.transport, cs.enabled,
                   s.command, s.args, s.env, s.runtime
            FROM config_suit_servers cs
            JOIN servers s ON cs.server_id = s.id
            WHERE cs.config_suit_id = ? AND cs.enabled = TRUE
            ORDER BY cs.server_name
            "#,
        )
        .bind(config_suit_id)
        .fetch_all(self.db_pool.as_ref())
        .await?;

        let mut servers = Vec::new();
        for row in rows {
            servers.push(ServerInfo {
                id: row.get("server_id"),
                name: row.get("server_name"),
                command: row.get("command"),
                args: serde_json::from_str(&row.get::<String, _>("args")).unwrap_or_default(),
                env: serde_json::from_str(&row.get::<String, _>("env")).unwrap_or_default(),
                runtime: row.get("runtime"),
                transport: row.get("transport"),
            });
        }
        Ok(servers)
    }

    /// Get all enabled servers
    async fn get_all_enabled_servers(&self) -> Result<Vec<ServerInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, command, args, env, runtime
            FROM servers
            WHERE enabled = TRUE
            ORDER BY name
            "#,
        )
        .fetch_all(self.db_pool.as_ref())
        .await?;

        let mut servers = Vec::new();
        for row in rows {
            servers.push(ServerInfo {
                id: row.get("id"),
                name: row.get("name"),
                command: row.get("command"),
                args: serde_json::from_str(&row.get::<String, _>("args")).unwrap_or_default(),
                env: serde_json::from_str(&row.get::<String, _>("env")).unwrap_or_default(),
                runtime: row.get("runtime"),
                transport: "stdio".to_string(), // Default transport
            });
        }
        Ok(servers)
    }

    /// Get server by ID
    async fn get_server_by_id(
        &self,
        server_id: &str,
    ) -> Result<ServerInfo> {
        let row = sqlx::query(
            r#"
            SELECT id, name, command, args, env, runtime
            FROM servers
            WHERE id = ?
            "#,
        )
        .bind(server_id)
        .fetch_one(self.db_pool.as_ref())
        .await?;

        Ok(ServerInfo {
            id: row.get("id"),
            name: row.get("name"),
            command: row.get("command"),
            args: serde_json::from_str(&row.get::<String, _>("args")).unwrap_or_default(),
            env: serde_json::from_str(&row.get::<String, _>("env")).unwrap_or_default(),
            runtime: row.get("runtime"),
            transport: "stdio".to_string(),
        })
    }

    /// Get client configuration path
    async fn get_client_config_path(
        &self,
        client_identifier: &str,
    ) -> Result<String> {
        let row = sqlx::query(
            r#"
            SELECT config_path
            FROM client_detection_rules
            WHERE client_identifier = ? AND enabled = TRUE
            ORDER BY priority ASC
            LIMIT 1
            "#,
        )
        .bind(client_identifier)
        .fetch_one(self.db_pool.as_ref())
        .await?;

        Ok(row.get("config_path"))
    }

    /// Generate configuration content based on client rules and servers
    async fn generate_config_content(
        &self,
        config_rule: &ConfigRule,
        servers: &[ServerInfo],
        request: &GenerationRequest,
    ) -> Result<String> {
        let mut config = json!({});

        // Create the top-level key structure
        let mut servers_config = json!({});

        for server in servers {
            let server_config = self
                .generate_server_config(config_rule, server, request)
                .await?;
            servers_config[&server.name] = server_config;
        }

        config[&config_rule.top_level_key] = servers_config;

        // Pretty print JSON with proper formatting
        Ok(serde_json::to_string_pretty(&config)?)
    }

    /// Generate configuration for a single server
    async fn generate_server_config(
        &self,
        config_rule: &ConfigRule,
        server: &ServerInfo,
        request: &GenerationRequest,
    ) -> Result<Value> {
        // Determine transport to use
        let transport = self.select_transport(config_rule, &server.transport);

        // Get format rule for the transport
        let format_rule = config_rule
            .format_rules
            .get(&transport)
            .ok_or_else(|| anyhow::anyhow!("No format rule for transport: {}", transport))?;

        // Generate server configuration based on mode
        match request.mode {
            GenerationMode::Transparent => {
                self.generate_transparent_config(format_rule, server, &transport)
                    .await
            }
            GenerationMode::Hosted => {
                self.generate_hosted_config(format_rule, server, &transport)
                    .await
            }
        }
    }

    /// Select appropriate transport for server
    fn select_transport(
        &self,
        config_rule: &ConfigRule,
        preferred_transport: &str,
    ) -> String {
        // If preferred transport is supported, use it
        if config_rule
            .supported_transports
            .contains(&preferred_transport.to_string())
        {
            return preferred_transport.to_string();
        }

        // Otherwise, use the first supported transport
        config_rule
            .supported_transports
            .first()
            .cloned()
            .unwrap_or_else(|| "stdio".to_string())
    }

    /// Generate transparent mode configuration (direct connection)
    async fn generate_transparent_config(
        &self,
        format_rule: &FormatRule,
        server: &ServerInfo,
        transport: &str,
    ) -> Result<Value> {
        let mut config = json!({});

        // Apply template with actual values
        for (key, template) in &format_rule.template {
            let value = self.apply_template(template, server, transport).await?;
            config[key] = value;
        }

        Ok(config)
    }

    /// Generate hosted mode configuration (through MCPMate proxy)
    async fn generate_hosted_config(
        &self,
        format_rule: &FormatRule,
        server: &ServerInfo,
        transport: &str,
    ) -> Result<Value> {
        let mut config = json!({});

        // For hosted mode, we connect to MCPMate proxy instead of direct server
        for (key, template) in &format_rule.template {
            let value = match key.as_str() {
                "command" => json!("mcpmate"),
                "args" => json!(["proxy", "--server", &server.id]),
                "url" => json!(format!("http://localhost:3000/mcp/{}", server.id)),
                _ => self.apply_template(template, server, transport).await?,
            };
            config[key] = value;
        }

        Ok(config)
    }

    /// Apply template with actual server values
    async fn apply_template(
        &self,
        template: &str,
        server: &ServerInfo,
        _transport: &str,
    ) -> Result<Value> {
        // Apply cross-platform command wrapping for stdio transport
        let (final_command, final_args) =
            self.apply_platform_wrapper(&server.command, &server.args)?;

        let result = template
            .replace("{{command}}", &final_command)
            .replace("{{args}}", &serde_json::to_string(&final_args)?)
            .replace("{{env}}", &serde_json::to_string(&server.env)?)
            .replace("{{runtime}}", &server.runtime)
            .replace(
                "{{url}}",
                &format!("http://localhost:3000/mcp/{}", server.id),
            )
            .replace("{{headers}}", "{}");

        // Try to parse as JSON first, fallback to string
        serde_json::from_str(&result).or_else(|_| Ok(json!(result)))
    }

    /// Apply platform-specific command wrapping (Windows cmd /c, etc.)
    fn apply_platform_wrapper(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<(String, Vec<String>)> {
        match std::env::consts::OS {
            "windows" => {
                // Windows: wrap with cmd /c
                let mut wrapped_args = vec!["/c".to_string(), command.to_string()];
                wrapped_args.extend_from_slice(args);
                Ok(("cmd".to_string(), wrapped_args))
            }
            _ => {
                // Unix-like systems: use command directly
                Ok((command.to_string(), args.to_vec()))
            }
        }
    }
}

/// Server information for configuration generation
#[derive(Debug, Clone)]
struct ServerInfo {
    id: String,
    name: String,
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    runtime: String,
    transport: String,
}
