use anyhow::Result;
use clap::Parser;
use mcpman::{
    config::{load_rule_config, load_server_config},
    proxy::{ConnectionStatus, ProxyServer},
};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use tracing_subscriber::{self, EnvFilter};

/// Command line arguments for the MCP stdio proxy server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the MCP configuration file
    #[arg(short, long, default_value = "config/mcp.json")]
    config: PathBuf,

    /// Path to the rule configuration file
    #[arg(short, long, default_value = "config/rule.json5")]
    rule_config: PathBuf,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Operation timeout in seconds
    #[arg(long, default_value = "30")]
    timeout: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(
                args.log_level
                    .parse()
                    .unwrap_or(tracing::Level::INFO.into()),
            ),
        )
        .init();

    // Load environment variables from .env file
    if let Ok(path) = std::env::current_dir().map(|p| p.join(".env")) {
        if path.exists() {
            match dotenvy::from_path(&path) {
                Ok(_) => {
                    tracing::info!("Loaded environment from {}", path.display());
                }
                Err(e) => {
                    tracing::error!("Error loading .env file: {}", e);
                }
            }
        } else {
            tracing::debug!("No .env file found at {}", path.display());
        }
    }

    // Load the MCP server and rule configuration
    let config = load_server_config(&args.config)?;
    let rule_config = load_rule_config(&args.rule_config)?;

    // Log the loaded configuration
    tracing::info!("Loaded configuration from: {}", args.config.display());
    tracing::info!(
        "Found {} MCP servers in configuration",
        config.mcp_servers.len()
    );
    tracing::info!(
        "Loaded rule configuration from: {}",
        args.rule_config.display()
    );

    // Convert rule config to HashMap<String, bool>
    let rule_map = rule_config
        .rules
        .iter()
        .map(|(name, rule)| (name.clone(), rule.enabled))
        .collect::<HashMap<String, bool>>();

    // Create proxy server
    let proxy = ProxyServer::new(Arc::new(config), Arc::new(rule_map));

    // Get a reference to the connection pool
    let connection_pool = Arc::clone(&proxy.connection_pool);

    // Connect to all servers in the background
    tokio::spawn(async move {
        // Wait for a short time to ensure everything is initialized
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect to all servers
        let mut pool = connection_pool.lock().await;

        // Connect to all servers in parallel
        if let Err(e) = pool.connect_all().await {
            tracing::error!("Error in parallel connection process: {}", e);
        }

        // Record the connection status
        let connected_count = pool
            .connections
            .values()
            .filter(|instances| {
                instances
                    .values()
                    .any(|conn| matches!(conn.status, ConnectionStatus::Ready))
            })
            .count();

        tracing::info!(
            "Connected to {}/{} upstream servers",
            connected_count,
            pool.connections.len()
        );
    });

    // TODO: Implement stdio server
    tracing::info!("Starting MCP stdio server");
    tracing::warn!("stdio server mode is not yet implemented");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down...");

    // Disconnect from all servers
    {
        let mut pool = proxy.connection_pool.lock().await;
        pool.disconnect_all().await?;
        tracing::info!("Disconnected from all upstream servers");
    }

    Ok(())
}
