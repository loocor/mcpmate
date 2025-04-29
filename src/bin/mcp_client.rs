use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dotenvy;
use mcp_client::{
    client::{handle_sse_server, handle_stdio_server},
    config::{load_rule_config, load_server_config},
};
use std::path::PathBuf;
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

/// Command line arguments for the MCP client
#[derive(Subcommand, Debug)]
enum Commands {
    /// List available servers
    List,

    /// Get information about a specific server
    Info {
        /// name of the server
        #[arg(required = true)]
        server: String,
    },

    /// Call a tool on a specific server
    Call {
        /// name of the server
        #[arg(long)]
        server: String,
        /// name of the tool
        #[arg(long)]
        tool: String,
        /// tool parameters as a JSON string
        #[arg(long, required = false)]
        params: Option<String>,
        /// tool parameters from a JSON file
        #[arg(long, required = false)]
        params_file: Option<PathBuf>,
    },
}

/// Main function for the MCP client
#[tokio::main]
async fn main() -> Result<()> {
    // initialize the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    // parse command line arguments
    let args = Args::parse();

    // load the MCP server and rule configuration
    let config = load_server_config(&args.config)?;
    let rule_config = load_rule_config("config/rule.json5")?;

    // load env vars if .env file exists
    let _ = dotenvy::dotenv();

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

            match server_config.kind.as_str() {
                "stdio" => handle_stdio_server(&server, server_config).await?,
                "sse" => handle_sse_server(&server, server_config).await?,
                _ => println!(
                    "\nServer type '{}' is not supported for tool listing",
                    server_config.kind
                ),
            }
        }
        Commands::Call {
            server,
            tool,
            params,
            params_file,
        } => {
            // check if the server exists
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

            // only support stdio/sse
            match server_config.kind.as_str() {
                "stdio" | "sse" => {}
                _ => {
                    println!(
                        "\nServer type '{}' is not supported for tool calling",
                        server_config.kind
                    );
                    return Ok(());
                }
            }

            // Load params from file or string
            let param_json: Option<serde_json::Value> =
                if let Some(file) = params_file {
                    let content = std::fs::read_to_string(&file)
                        .with_context(|| format!("Failed to read params file: {:?}", file))?;
                    Some(serde_json::from_str(&content).with_context(|| {
                        format!("Failed to parse JSON in params file: {:?}", file)
                    })?)
                } else if let Some(param_str) = params {
                    Some(
                        serde_json::from_str(&param_str)
                            .with_context(|| "Failed to parse JSON in --params")?,
                    )
                } else {
                    None
                };

            // build tool call parameters
            let call_param = mcp_client::client::CallToolInput {
                server_name: server.clone(),
                server_config,
                tool_name: tool.clone(),
                arguments: param_json,
            };

            // call tool based on server type
            match server_config.kind.as_str() {
                "stdio" => {
                    mcp_client::client::call_tool_stdio(call_param).await?;
                }
                "sse" => {
                    mcp_client::client::call_tool_sse(call_param).await?;
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}
