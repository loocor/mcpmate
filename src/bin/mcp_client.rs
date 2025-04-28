use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rmcp::{
    service::ServiceExt,
    transport::TokioChildProcess,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tokio::process::Command;
use tracing_subscriber::{self, EnvFilter};

/// Configuration for MCP servers
#[derive(Debug, Deserialize, Serialize)]
struct Config {
    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, ServerConfig>,
}

/// Configuration for a single MCP server
#[derive(Debug, Deserialize, Serialize)]
struct ServerConfig {
    command: String,
    args: Vec<String>,
    #[serde(rename = "commandPath")]
    command_path: Option<String>,
    enabled: Option<bool>,
    env: Option<HashMap<String, String>>,
}

/// Command line arguments for the MCP client
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the MCP configuration file
    #[arg(short, long, default_value = "mcp.json")]
    config: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List available servers
    List,

    /// Get information about a specific server
    Info {
        /// Name of the server
        #[arg(required = true)]
        server: String,
    },
}

/// Load the MCP configuration from a file
fn load_config(path: &PathBuf) -> Result<Config> {
    let config_str = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = serde_json::from_str(&config_str)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    // Parse command line arguments
    let args = Args::parse();

    // Load the configuration
    let config = load_config(&args.config)?;

    match args.command {
        Commands::List => {
            println!("Available MCP servers:");
            for (name, server_config) in &config.mcp_servers {
                let enabled = server_config.enabled.unwrap_or(false);
                println!("  - {} ({})", name, if enabled { "enabled" } else { "disabled" });
            }
        },
        Commands::Info { server } => {
            // Check if the server exists
            let server_config = config.mcp_servers.get(&server)
                .with_context(|| format!("Server '{}' not found in configuration", server))?;

            println!("Server: {}", server);
            println!("Command: {}", server_config.command);
            println!("Arguments: {:?}", server_config.args);
            println!("Enabled: {}", server_config.enabled.unwrap_or(false));

            // Only connect to the server if it's enabled
            if server_config.enabled.unwrap_or(false) {
                println!("\nConnecting to server...");

                // Build the command
                let mut command = if let Some(command_path) = &server_config.command_path {
                    let full_path = format!("{}/{}", command_path, server_config.command);
                    Command::new(full_path)
                } else {
                    Command::new(&server_config.command)
                };

                // Add arguments
                command.args(&server_config.args);

                // Add environment variables
                if let Some(env) = &server_config.env {
                    for (key, value) in env {
                        command.env(key, value);
                    }
                }

                // Connect to the server
                let service = ()
                    .serve(TokioChildProcess::new(&mut command)?)
                    .await?;

                // List tools
                let tools_result = service.list_tools(Default::default()).await?;

                println!("\nAvailable tools:");
                let tools = tools_result.tools;
                if !tools.is_empty() {
                    for tool in tools {
                        println!("  - {}: {}", tool.name, tool.description.clone().map_or_else(|| "No description".to_string(), |d| d.to_string()));
                    }
                } else {
                    println!("  No tools available");
                }

                // Close the connection
                service.cancel().await?;
            } else {
                println!("\nServer is disabled. Use --enable to enable it.");
            }
        },
    }

    Ok(())
}
