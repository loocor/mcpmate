// This file will contain shared code between the client and server
// For now, it's just a placeholder

/// Configuration module for loading and parsing MCP configuration
pub mod config {
    use anyhow::{Context, Result};
    use json5;
    use serde::{Deserialize, Serialize};
    use std::{collections::HashMap, path::Path};

    /// Configuration for MCP servers
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Config {
        #[serde(rename = "mcpServers")]
        pub mcp_servers: HashMap<String, ServerConfig>,
    }

    /// Configuration for a single MCP server
    #[derive(Debug, Deserialize, Serialize)]
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
    #[derive(Debug, Deserialize, Serialize)]
    pub struct RuleConfig {
        pub rules: HashMap<String, ServiceRule>,
    }

    /// Rule config for a single service
    #[derive(Debug, Deserialize, Serialize)]
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
}

#[cfg(test)]
mod tests {
    use super::config::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_server_config() {
        let config_path = PathBuf::from("config/mcp.json");
        let config = load_server_config(config_path).unwrap();

        // verify the basic structure
        assert!(config.mcp_servers.contains_key("blender"));
        assert!(config.mcp_servers.contains_key("firecrawl"));

        // verify stdio type service
        let blender = config.mcp_servers.get("blender").unwrap();
        assert_eq!(blender.kind, "stdio");
        assert_eq!(blender.command.as_deref(), Some("uvx"));
        assert!(blender
            .args
            .as_ref()
            .map_or(false, |args| args.contains(&"blender-mcp".to_string())));

        // verify optional fields
        let firecrawl = config.mcp_servers.get("firecrawl").unwrap();
        assert!(firecrawl.env.is_some());
        assert!(firecrawl
            .env
            .as_ref()
            .unwrap()
            .contains_key("FIRECRAWL_API_KEY"));

        // verify sse type service
        let thinking = config.mcp_servers.get("thinking").unwrap();
        assert_eq!(thinking.kind, "sse");
        assert!(thinking.url.is_some());
    }

    #[test]
    fn test_load_rule_config() {
        let rule_path = PathBuf::from("config/rule.json5");
        let config = load_rule_config(rule_path).unwrap();

        // verify the basic structure
        assert!(config.rules.contains_key("blender"));
        assert!(config.rules.contains_key("firecrawl"));

        // verify the service enabled status
        let blender = config.rules.get("blender").unwrap();
        assert!(blender.enabled);
        assert!(blender.tools.is_empty());

        // verify the disabled service
        let proxy = config.rules.get("proxy").unwrap();
        assert!(!proxy.enabled);
    }

    #[test]
    fn test_config_error_handling() {
        // test the file not exists case
        let result = load_server_config("nonexistent.json");
        assert!(result.is_err());

        // test the file format error case
        let result = load_rule_config("Cargo.toml");
        assert!(result.is_err());
    }
}
