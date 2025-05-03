// Core configuration module for MCPMan
// Contains shared configuration types and functions

use anyhow::{Context, Result};
use json5;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

/// Configuration for MCP servers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, ServerConfig>,
}

/// Configuration for a single MCP server
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

/// Load the MCP server configuration from a file
pub fn load_server_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let config_str = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
    let config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;

    Ok(config)
}

/// Rule config for services
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleConfig {
    pub rules: HashMap<String, ServiceRule>,
}

/// Rule config for a single service
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceRule {
    pub enabled: bool,
    pub tools: HashMap<String, bool>,
}

/// Load the rule config from a file
pub fn load_rule_config<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<RuleConfig> {
    let config_str = std::fs::read_to_string(path.as_ref()).with_context(|| {
        format!(
            "Failed to read rule config file: {}",
            path.as_ref().display()
        )
    })?;
    let config: RuleConfig = json5::from_str(&config_str).with_context(|| {
        format!(
            "Failed to parse rule config file: {}",
            path.as_ref().display()
        )
    })?;
    Ok(config)
}
