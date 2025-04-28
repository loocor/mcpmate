use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mcp_client::config::load_config;
use rmcp::{service::ServiceExt, transport::TokioChildProcess};
use serde_json;
use std::path::PathBuf;
use tokio::process::Command;
use tracing_subscriber::{self, EnvFilter};

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
                println!(
                    "  - {} ({})",
                    name,
                    if enabled { "enabled" } else { "disabled" }
                );
            }
        }
        Commands::Info { server } => {
            // Check if the server exists
            let server_config = config
                .mcp_servers
                .get(&server)
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
                    let mut cmd = Command::new(&server_config.command);
                    cmd.current_dir(command_path);
                    let path_var = std::env::var("PATH").unwrap_or_default();
                    let new_path = format!("{}:{}", command_path, path_var);
                    cmd.env("PATH", new_path);

                    cmd
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
                let service = ().serve(TokioChildProcess::new(&mut command)?).await?;

                // List tools
                let tools_result = service.list_tools(Default::default()).await?;

                println!("\nAvailable tools:");
                let tools = tools_result.tools;
                if !tools.is_empty() {
                    for tool in &tools {
                        println!("  - {}", tool.name);
                        println!(
                            "      Description: {}",
                            tool.description.clone().into_owned()
                        );
                        println!(
                            "      Parameters: {}",
                            serde_json::to_string_pretty(&tool.input_schema)
                                .unwrap_or_else(|_| "<invalid schema>".to_string())
                        );
                    }
                } else {
                    println!("  No tools available");
                }

                // Close the connection
                service.cancel().await?;
            } else {
                println!("\nServer is disabled. Use --enable to enable it.");
            }
        }
    }

    Ok(())
}
