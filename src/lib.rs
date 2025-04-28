// This file will contain shared code between the client and server
// For now, it's just a placeholder

/// Configuration module for loading and parsing MCP configuration
pub mod config {
    use anyhow::{Context, Result};
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
        pub command: String,
        pub args: Vec<String>,
        #[serde(rename = "commandPath")]
        pub command_path: Option<String>,
        pub enabled: Option<bool>,
        pub env: Option<HashMap<String, String>>,
    }

    /// Load the MCP configuration from a file
    pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
        let config_str = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        
        let config: Config = serde_json::from_str(&config_str)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;
        
        Ok(config)
    }
}
