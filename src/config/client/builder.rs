// Configuration builder for client applications
// Handles the high-level configuration building logic

use anyhow::Result;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

use super::loader::{ServerInfo, ServerLoader};
use super::models::{ConfigRule, GeneratedConfig, GenerationMode, GenerationRequest};
use super::strategy::TransportStrategy;
use super::utils::set_nested_value;
use crate::common::config::config_keys;
use crate::common::server::transport_formats;

/// Configuration builder that orchestrates the generation process
pub struct ConfigBuilder {
    db_pool: Arc<SqlitePool>,
    server_loader: ServerLoader,
    transport_strategy: TransportStrategy,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new(db_pool: Arc<SqlitePool>) -> Self {
        Self {
            server_loader: ServerLoader::new(db_pool.clone()),
            transport_strategy: TransportStrategy::new(),
            db_pool,
        }
    }

    /// Generate configuration for a specific client
    pub async fn generate_config(
        &self,
        request: &GenerationRequest,
    ) -> Result<GeneratedConfig> {
        // Get client configuration rule
        let config_rule = self.get_config_rule(&request.client_identifier).await?;

        // Get servers to include in configuration
        let servers = self
            .server_loader
            .get_servers_for_generation(request)
            .await?;

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
            SELECT id, client_app_id, client_identifier, top_level_key, config_type,
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
        let supported_runtimes: std::collections::HashMap<String, Vec<String>> =
            serde_json::from_str(&row.get::<String, _>("supported_runtimes"))?;
        let format_rules: std::collections::HashMap<
            String,
            crate::config::client::models::FormatRule,
        > = serde_json::from_str(&row.get::<String, _>("format_rules"))?;
        let security_features: Option<crate::config::client::models::SecurityFeatures> = row
            .get::<Option<String>, _>("security_features")
            .map(|s| serde_json::from_str(&s))
            .transpose()?;

        // Parse config_type from database
        let config_type_str: String = row.get("config_type");
        let config_type = match config_type_str.as_str() {
            "mixed" => crate::config::client::models::ConfigType::Mixed,
            "array" => crate::config::client::models::ConfigType::Array,
            _ => crate::config::client::models::ConfigType::Standard,
        };

        Ok(ConfigRule {
            id: row.get("id"),
            client_app_id: row.get("client_app_id"),
            client_identifier: row.get("client_identifier"),
            top_level_key: row.get("top_level_key"),
            config_type,
            supported_transports,
            supported_runtimes,
            format_rules,
            security_features,
        })
    }

    /// Get client configuration path
    async fn get_client_config_path(
        &self,
        client_identifier: &str,
    ) -> Result<String> {
        // Get current platform
        let current_platform = {
            #[cfg(target_os = "macos")]
            {
                "macos"
            }
            #[cfg(target_os = "windows")]
            {
                "windows"
            }
            #[cfg(target_os = "linux")]
            {
                "linux"
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
            {
                "unknown"
            }
        };

        let row = sqlx::query(
            r#"
            SELECT config_path
            FROM client_detection_rules
            WHERE client_identifier = ? AND platform = ? AND enabled = TRUE
            ORDER BY priority ASC
            LIMIT 1
            "#,
        )
        .bind(client_identifier)
        .bind(current_platform)
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
        // process array config or object config based on the rule
        let json_value = match config_rule.config_type {
            crate::config::client::models::ConfigType::Array => {
                self.generate_array_config_value(config_rule, servers, request)
                    .await?
            }
            crate::config::client::models::ConfigType::Standard
            | crate::config::client::models::ConfigType::Mixed => {
                self.generate_object_config_value(config_rule, servers, request)
                    .await?
            }
        };

        // Pretty print JSON with proper formatting
        Ok(serde_json::to_string_pretty(&json_value)?)
    }

    /// Generate the object config JSON value
    async fn generate_object_config_value(
        &self,
        config_rule: &ConfigRule,
        servers: &[ServerInfo],
        request: &GenerationRequest,
    ) -> Result<Value> {
        let mut config = json!({});

        match request.mode {
            GenerationMode::Transparent => {
                // Handle transparent mode (direct server configs)
                let (servers_config, skipped_servers) = self
                    .process_transparent_servers(config_rule, servers, request, false)
                    .await?;

                // Log skipped servers if any
                self.log_skipped_servers(&skipped_servers);

                // Add servers to the top-level key (supports nested paths like "mcp.servers")
                set_nested_value(&mut config, &config_rule.top_level_key, servers_config);
            }
            GenerationMode::Hosted => {
                // Handle hosted mode (unified endpoint)
                let endpoint_config = self
                    .get_unified_endpoint_config(config_rule, &request.client_identifier)
                    .await?;

                // Add the endpoint config to the top-level key (supports nested paths like "mcp.servers")
                set_nested_value(
                    &mut config,
                    &config_rule.top_level_key,
                    json!({
                        config_keys::MCPMATE: endpoint_config
                    }),
                );
            }
        }

        Ok(config)
    }

    /// Generate the array config JSON value
    async fn generate_array_config_value(
        &self,
        config_rule: &ConfigRule,
        servers: &[ServerInfo],
        request: &GenerationRequest,
    ) -> Result<Value> {
        match request.mode {
            GenerationMode::Transparent => {
                // Handle transparent mode (direct server configs)
                let (server_configs, skipped_servers) = self
                    .process_transparent_servers(config_rule, servers, request, true)
                    .await?;

                // Log skipped servers if any
                self.log_skipped_servers(&skipped_servers);

                // For array configs, the result is already the array
                Ok(server_configs)
            }
            GenerationMode::Hosted => {
                // Handle hosted mode (unified endpoint)
                let endpoint_config = self
                    .get_unified_endpoint_config(config_rule, &request.client_identifier)
                    .await?;

                // Create a MCPMate config object and add to array
                let mut mcpmate_config = endpoint_config.as_object().cloned().unwrap_or_default();
                mcpmate_config.insert("name".to_string(), json!(config_keys::MCPMATE));

                // Return as an array with a single element
                Ok(json!([mcpmate_config]))
            }
        }
    }

    /// Process servers for transparent mode, returning the config and skipped servers
    async fn process_transparent_servers(
        &self,
        config_rule: &ConfigRule,
        servers: &[ServerInfo],
        request: &GenerationRequest,
        is_array: bool,
    ) -> Result<(Value, Vec<String>)> {
        let mut skipped_servers = Vec::new();

        if is_array {
            // For array configs, build a vector of server configs
            let mut server_configs = Vec::new();

            for server in servers {
                match self
                    .transport_strategy
                    .generate_server_config(config_rule, server, &request.mode)
                    .await
                {
                    Ok(server_config) => {
                        // Add server name to the config
                        let mut config_with_name =
                            server_config.as_object().cloned().unwrap_or_default();
                        config_with_name.insert("name".to_string(), json!(server.name));
                        server_configs.push(json!(config_with_name));
                    }
                    Err(e) => {
                        self.log_skipped_server(server, &e);
                        skipped_servers.push(server.name.clone());
                    }
                }
            }

            Ok((json!(server_configs), skipped_servers))
        } else {
            // For object configs, build a map of server name to config
            let mut servers_config = json!({});

            for server in servers {
                match self
                    .transport_strategy
                    .generate_server_config(config_rule, server, &request.mode)
                    .await
                {
                    Ok(server_config) => {
                        servers_config[&server.name] = server_config;
                    }
                    Err(e) => {
                        self.log_skipped_server(server, &e);
                        skipped_servers.push(server.name.clone());
                    }
                }
            }

            Ok((servers_config, skipped_servers))
        }
    }

    /// Get unified endpoint configuration for hosted mode
    async fn get_unified_endpoint_config(
        &self,
        config_rule: &ConfigRule,
        client_identifier: &str,
    ) -> Result<Value> {
        let best_transport = self
            .transport_strategy
            .get_best_supported_transport(config_rule);

        match best_transport.as_str() {
            t if t == transport_formats::STREAMABLE_HTTP => {
                self.transport_strategy
                    .generate_unified_endpoint_config(
                        config_rule,
                        client_identifier,
                        transport_formats::STREAMABLE_HTTP,
                    )
                    .await
            }
            t if t == transport_formats::SSE => {
                self.transport_strategy
                    .generate_unified_endpoint_config(
                        config_rule,
                        client_identifier,
                        transport_formats::SSE,
                    )
                    .await
            }
            t if t == transport_formats::STDIO => {
                self.transport_strategy
                    .generate_unified_bridge_config(config_rule, client_identifier)
                    .await
            }
            _ => Err(anyhow::anyhow!(
                "No supported transport types found for client"
            )),
        }
    }

    /// Log a skipped server with warning
    fn log_skipped_server(
        &self,
        server: &ServerInfo,
        error: &anyhow::Error,
    ) {
        tracing::warn!(
            "Skipping server '{}' with transport '{}': {}",
            server.name,
            server.server_type,
            error
        );
    }

    /// Log summary of skipped servers
    fn log_skipped_servers(
        &self,
        skipped_servers: &[String],
    ) {
        if !skipped_servers.is_empty() {
            tracing::info!(
                "Skipped {} servers in transparent mode due to unsupported transport types: {}",
                skipped_servers.len(),
                skipped_servers.join(", ")
            );
        }
    }
}
