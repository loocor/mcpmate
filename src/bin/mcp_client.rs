use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mcp_client::config::{load_rule_config, load_server_config};
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
    #[arg(short, long, default_value = "config/mcp.json")]
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

    // Load the MCP server and rule configuration
    let config = load_server_config(&args.config)?;
    let rule_config = load_rule_config("config/rule.json5")?;

    match args.command {
        Commands::List => {
            println!("Available MCP servers:");
            for (name, server_config) in &config.mcp_servers {
                let enabled = rule_config
                    .rules
                    .get(name)
                    .map(|r| r.enabled)
                    .unwrap_or(false);
                println!(
                    "  - {} ({} type{}{})",
                    name,
                    server_config.kind,
                    server_config
                        .command
                        .as_deref()
                        .map_or("".to_string(), |cmd| format!(" with command: {}", cmd)),
                    if enabled { " [enabled]" } else { " [disabled]" }
                );
            }
        }
        Commands::Info { server } => {
            // Check if the server exists
            let server_config = config
                .mcp_servers
                .get(&server)
                .with_context(|| format!("Server '{}' not found in configuration", server))?;
            let enabled = rule_config
                .rules
                .get(&server)
                .map(|r| r.enabled)
                .unwrap_or(false);
            if !enabled {
                println!("Server '{}' is disabled (by rule config).", server);
                return Ok(());
            }

            println!("Server: {}", server);
            println!("Type: {}", server_config.kind);
            println!("Command: {:?}", server_config.command);
            println!("Arguments: {:?}", server_config.args);

            if server_config.kind == "stdio" {
                println!("\nConnecting to server...\n");

                // Build the command
                let command_str = server_config
                    .command
                    .as_ref()
                    .with_context(|| format!("Command not specified for server '{}'", server))?;
                let mut command = Command::new(command_str);

                // Add arguments if present
                if let Some(args) = &server_config.args {
                    command.args(args);
                }

                // Add environment variables if present
                if let Some(env) = &server_config.env {
                    for (key, value) in env {
                        command.env(key, value);
                    }
                }

                // Connect to the server
                let service = ().serve(TokioChildProcess::new(&mut command)?).await?;

                // List tools
                println!("ready to call list_all_tools ...");
                let tools_result = match tokio::time::timeout(
                    tokio::time::Duration::from_secs(10),
                    service.list_all_tools(),
                )
                .await
                {
                    Ok(Ok(tools)) => {
                        println!("list_all_tools call success, get {} tools", tools.len());
                        tools
                    }
                    Ok(Err(e)) => {
                        println!("list_all_tools call failed: {e}");
                        service.cancel().await?;
                        return Ok(());
                    }
                    Err(_) => {
                        println!("list_all_tools call timeout!");
                        service.cancel().await?;
                        return Ok(());
                    }
                };

                println!("\nAvailable tools:");
                if !tools_result.is_empty() {
                    for (i, tool) in tools_result.iter().enumerate() {
                        println!("{:02} - {}: {}", i + 1, tool.name, tool.description);
                        println!("     Parameters:");
                        println!("{}", schema_formater(&tool.input_schema));
                        println!();
                    }
                } else {
                    println!("  No tools available");
                }

                // Close the connection
                service.cancel().await?;
            } else {
                println!(
                    "\nServer type '{}' is not supported for tool listing",
                    server_config.kind
                );
            }
        }
    }

    Ok(())
}

/// Format the schema parameters into a human-readable string
fn schema_formater(schema: &serde_json::Map<String, serde_json::Value>) -> String {
    // Convert to Value for easier processing
    let schema_value: serde_json::Value =
        serde_json::to_value(schema).unwrap_or_else(|_| serde_json::json!({}));

    // Extract and format parameter information
    if let Some(properties) = schema_value.get("properties").and_then(|p| p.as_object()) {
        let mut param_info = Vec::new();

        for (param_name, param_details) in properties {
            let param_type = param_details
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            let param_desc = param_details
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("");
            let required = schema_value
                .get("required")
                .and_then(|r| r.as_array())
                .map(|r| r.iter().any(|v| v.as_str() == Some(param_name)))
                .unwrap_or(false);

            param_info.push(format!(
                "       - {}{}: {} ({})",
                param_name,
                if required { " [required]" } else { "" },
                param_type,
                param_desc
            ));

            // Handle nested properties
            if let Some(sub_properties) =
                param_details.get("properties").and_then(|p| p.as_object())
            {
                for (sub_name, sub_details) in sub_properties {
                    let sub_type = sub_details
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown");
                    let sub_desc = sub_details
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    param_info.push(format!(
                        "         • {}: {} ({})",
                        sub_name, sub_type, sub_desc
                    ));
                }
            }
        }

        param_info.join("\n")
    } else {
        "       No parameters required".to_string()
    }
}
