// Transport strategy for configuration generation
// Handles different transport types and their specific logic

use anyhow::Result;
use serde_json::{Value, json};

use super::loader::ServerInfo;
use super::models::{ConfigRule, FormatRule, GenerationMode};
use super::template::TemplateEngine;
use crate::common::get_bridge_path;
use crate::common::server::{
    TRANSPORT_PRIORITY,
    transport_formats::{SSE, STDIO, STREAMABLE_HTTP},
};
use crate::system::config::get_runtime_port_config;

/// Transport strategy handler
pub struct TransportStrategy {
    template_engine: TemplateEngine,
}

impl Default for TransportStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl TransportStrategy {
    /// Create a new transport strategy handler
    pub fn new() -> Self {
        Self {
            template_engine: TemplateEngine::new(),
        }
    }

    /// Get the best supported transport type based on priority
    /// Priority: streamableHttp > sse > stdio
    pub fn get_best_supported_transport(
        &self,
        config_rule: &ConfigRule,
    ) -> String {
        let transports = &config_rule.supported_transports;

        for transport in TRANSPORT_PRIORITY {
            if transports.contains(&transport.to_string()) {
                return transport.to_string();
            }
        }

        // Fallback to stdio if none of the priority transports are supported
        STDIO.to_string()
    }

    /// Generate configuration for a single server based on mode and transport
    pub async fn generate_server_config(
        &self,
        config_rule: &ConfigRule,
        server: &ServerInfo,
        mode: &GenerationMode,
    ) -> Result<Value> {
        match mode {
            GenerationMode::Transparent => self.generate_transparent_config(config_rule, server).await,
            GenerationMode::Hosted => self.generate_hosted_config(config_rule, server).await,
        }
    }

    /// Generate transparent mode configuration (direct connection)
    async fn generate_transparent_config(
        &self,
        config_rule: &ConfigRule,
        server: &ServerInfo,
    ) -> Result<Value> {
        // In transparent mode, skip servers with unsupported transport types
        if !config_rule.supported_transports.contains(&server.server_type) {
            return Err(anyhow::anyhow!(
                "Transport type '{}' is not supported by client '{}' in transparent mode",
                server.server_type,
                config_rule.identifier
            ));
        }

        // Use the server's native transport type
        let format_rule = config_rule
            .format_rules
            .get(&server.server_type)
            .ok_or_else(|| anyhow::anyhow!("No format rule for transport: {}", server.server_type))?;

        self.apply_format_rule(format_rule, server, &server.server_type).await
    }

    /// Generate hosted mode configuration
    async fn generate_hosted_config(
        &self,
        config_rule: &ConfigRule,
        server: &ServerInfo,
    ) -> Result<Value> {
        // In hosted mode, use direct endpoints if client supports the transport type,
        // otherwise fall back to bridge stdio
        if config_rule.supported_transports.contains(&server.server_type) {
            // Client supports this transport type, use direct endpoint
            let format_rule = config_rule
                .format_rules
                .get(&server.server_type)
                .ok_or_else(|| anyhow::anyhow!("No format rule for transport: {}", server.server_type))?;

            self.generate_hosted_endpoint_config(format_rule, server, &server.server_type)
                .await
        } else {
            // Client doesn't support this transport type, use bridge stdio
            let format_rule = config_rule
                .format_rules
                .get(STDIO)
                .ok_or_else(|| anyhow::anyhow!("No stdio format rule for bridge mode"))?;

            self.generate_bridge_config(format_rule, server).await
        }
    }

    /// Generate hosted mode configuration with direct endpoints
    async fn generate_hosted_endpoint_config(
        &self,
        format_rule: &FormatRule,
        server: &ServerInfo,
        transport: &str,
    ) -> Result<Value> {
        let mut config = json!({});

        // For hosted mode with direct endpoints, use MCPMate's endpoints
        for (key, template) in &format_rule.template {
            let value = match key.as_str() {
                "url" | "serverUrl" => match transport {
                    t if t == SSE => {
                        let runtime_config = get_runtime_port_config();
                        json!(format!(
                            "http://localhost:{}/sse/{}",
                            runtime_config.mcp_port, server.id
                        ))
                    }
                    t if t == STREAMABLE_HTTP => {
                        let runtime_config = get_runtime_port_config();
                        json!(format!(
                            "http://localhost:{}/mcp/{}",
                            runtime_config.mcp_port, server.id
                        ))
                    }
                    _ => self.template_engine.apply_template(template, server, transport).await?,
                },
                "headers" => json!({}),
                _ => self.template_engine.apply_template(template, server, transport).await?,
            };
            config[key] = value;
        }

        Ok(config)
    }

    /// Generate bridge configuration for hosted mode
    async fn generate_bridge_config(
        &self,
        format_rule: &FormatRule,
        server: &ServerInfo,
    ) -> Result<Value> {
        let mut config = json!({});

        // For hosted mode bridge, we connect to MCPMate proxy instead of direct server
        for (key, template) in &format_rule.template {
            let value = match key.as_str() {
                "command" => json!("mcpmate"),
                "args" => json!(["proxy", "--server", &server.id]),
                "url" => {
                    let runtime_config = get_runtime_port_config();
                    json!(format!(
                        "http://localhost:{}/mcp/{}",
                        runtime_config.mcp_port, server.id
                    ))
                }
                _ => self.template_engine.apply_template(template, server, STDIO).await?,
            };
            config[key] = value;
        }

        Ok(config)
    }

    /// Generate unified endpoint configuration for hosted mode
    pub async fn generate_unified_endpoint_config(
        &self,
        config_rule: &ConfigRule,
        identifier: &str,
        transport: &str,
    ) -> Result<Value> {
        // Use the appropriate format rule for the transport type
        let format_rule = config_rule
            .format_rules
            .get(transport)
            .ok_or_else(|| anyhow::anyhow!("No format rule for transport: {}", transport))?;

        let mut config = json!({});

        // Create endpoint URL based on transport type with dynamic port
        let runtime_config = get_runtime_port_config();
        let endpoint_url = match transport {
            t if t == SSE => format!("http://localhost:{}/sse", runtime_config.mcp_port),
            t if t == STREAMABLE_HTTP => {
                format!("http://localhost:{}/mcp", runtime_config.mcp_port)
            }
            _ => format!("http://localhost:{}/mcp", runtime_config.mcp_port),
        };

        // Create a mock server info for hosted mode endpoint configuration
        let endpoint_server = TemplateEngine::create_mock_server(
            "mcpmate-endpoint",
            "mcpmate",
            None,
            Some(endpoint_url.to_string()),
            vec![],
            {
                let mut env = std::collections::HashMap::new();
                env.insert("X-Client-ID".to_string(), identifier.to_string());
                env
            },
            transport,
        );

        // Generate endpoint configuration using template processing
        for (key, template) in &format_rule.template {
            let value = match key.as_str() {
                "url" | "serverUrl" => json!(endpoint_url),
                "headers" => {
                    match transport {
                        t if t == STREAMABLE_HTTP => json!({}), // Streamable HTTP doesn't use custom headers in this format
                        _ => json!({"X-Client-ID": identifier}),
                    }
                }
                _ => {
                    self.template_engine
                        .apply_template(template, &endpoint_server, transport)
                        .await?
                }
            };
            config[key] = value;
        }

        Ok(config)
    }

    /// Generate unified MCPMate bridge configuration for hosted mode
    pub async fn generate_unified_bridge_config(
        &self,
        config_rule: &ConfigRule,
        identifier: &str,
    ) -> Result<Value> {
        // Use stdio format rule for the bridge configuration
        let format_rule = config_rule
            .format_rules
            .get(STDIO)
            .ok_or_else(|| anyhow::anyhow!("No stdio format rule for bridge mode"))?;

        let mut config = json!({});

        // Get dynamic bridge path
        let bridge_path = get_bridge_path().map_err(|e| anyhow::anyhow!("Failed to locate bridge component: {}", e))?;

        tracing::debug!("Using dynamic bridge path for client config: {}", bridge_path);

        // Create a mock server info for hosted mode bridge configuration
        let bridge_server = TemplateEngine::create_mock_server(
            "mcpmate-bridge",
            "mcpmate",
            Some(bridge_path),
            None,
            vec![],
            {
                let mut env = std::collections::HashMap::new();
                env.insert("APPID".to_string(), identifier.to_string());
                env
            },
            STDIO,
        );

        // Generate unified bridge configuration using template processing
        for (key, template) in &format_rule.template {
            let value = match key.as_str() {
                "type" => json!(STDIO),
                _ => {
                    self.template_engine
                        .apply_template(template, &bridge_server, STDIO)
                        .await?
                }
            };
            config[key] = value;
        }

        Ok(config)
    }

    /// Apply format rule with template processing
    async fn apply_format_rule(
        &self,
        format_rule: &FormatRule,
        server: &ServerInfo,
        transport: &str,
    ) -> Result<Value> {
        let mut config = json!({});

        // Apply template with actual values
        for (key, template) in &format_rule.template {
            let value = self.template_engine.apply_template(template, server, transport).await?;
            config[key] = value;
        }

        Ok(config)
    }
}
