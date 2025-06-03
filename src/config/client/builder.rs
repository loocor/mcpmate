// Configuration builder for client applications
// Handles the high-level configuration building logic

use anyhow::Result;
use serde_json::json;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

use super::models::{ConfigRule, GeneratedConfig, GenerationMode, GenerationRequest};
use super::loader::{ServerInfo, ServerLoader};
use super::strategy::TransportStrategy;

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

        match request.mode {
            GenerationMode::Transparent => {
                // Transparent mode: generate individual server configurations
                let mut servers_config = json!({});
                let mut skipped_servers = Vec::new();

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
                            // Log skipped server (unsupported transport in transparent mode)
                            tracing::warn!(
                                "Skipping server '{}' with transport '{}': {}",
                                server.name,
                                server.server_type,
                                e
                            );
                            skipped_servers.push(server.name.clone());
                        }
                    }
                }

                if !skipped_servers.is_empty() {
                    tracing::info!(
                        "Skipped {} servers in transparent mode due to unsupported transport types: {}",
                        skipped_servers.len(),
                        skipped_servers.join(", ")
                    );
                }

                config[&config_rule.top_level_key] = servers_config;
            }
            GenerationMode::Hosted => {
                // Hosted mode: choose the best transport type based on client capabilities
                let best_transport = self
                    .transport_strategy
                    .get_best_supported_transport(config_rule);

                match best_transport.as_str() {
                    "streamableHttp" => {
                        // Use streamable HTTP endpoint
                        let endpoint_config = self
                            .transport_strategy
                            .generate_unified_endpoint_config(
                                config_rule,
                                &request.client_identifier,
                                "streamableHttp",
                            )
                            .await?;
                        config[&config_rule.top_level_key] = json!({
                            "mcpmate": endpoint_config
                        });
                    }
                    "sse" => {
                        // Use SSE endpoint
                        let endpoint_config = self
                            .transport_strategy
                            .generate_unified_endpoint_config(
                                config_rule,
                                &request.client_identifier,
                                "sse",
                            )
                            .await?;
                        config[&config_rule.top_level_key] = json!({
                            "mcpmate": endpoint_config
                        });
                    }
                    "stdio" => {
                        // Fallback to bridge for stdio-only clients
                        let bridge_config = self
                            .transport_strategy
                            .generate_unified_bridge_config(config_rule, &request.client_identifier)
                            .await?;
                        config[&config_rule.top_level_key] = json!({
                            "mcpmate": bridge_config
                        });
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "No supported transport types found for client"
                        ));
                    }
                }
            }
        }

        // Pretty print JSON with proper formatting
        Ok(serde_json::to_string_pretty(&config)?)
    }
}
