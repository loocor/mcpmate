use anyhow::Result;
use clap::Parser;
use mcpman::{
    api::ApiServer,
    config::{load_rule_config, load_server_config},
    proxy::{ConnectionStatus, ProxyServer},
};
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use tracing_subscriber::{self, EnvFilter};

/// Command line arguments for the MCP proxy server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the MCP configuration file
    #[arg(short, long, default_value = "config/mcp.json")]
    config: PathBuf,

    /// Path to the rule configuration file
    #[arg(short, long, default_value = "config/rule.json5")]
    rule_config: PathBuf,

    /// Port to listen on for MCP server
    #[arg(short, long, default_value = "8000")]
    port: u16,

    /// Port to listen on for API server
    #[arg(long, default_value = "8080")]
    api_port: u16,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
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
        // Wait for a short time to ensure the SSE server is started
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

    // Start SSE server
    let mcp_bind_address = format!("127.0.0.1:{}", args.port).parse()?;
    tracing::info!("Starting MCP SSE server on {}", mcp_bind_address);

    let server_config = SseServerConfig {
        bind: mcp_bind_address,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: Default::default(),
    };

    // Create a factory function that returns a new ProxyServer instance
    let proxy_clone = proxy.clone();
    let factory = move || proxy_clone.clone();

    let mcp_server = SseServer::serve_with_config(server_config)
        .await?
        .with_service(factory);

    // Start API server
    let api_bind_address: SocketAddr = format!("127.0.0.1:{}", args.api_port).parse()?;
    tracing::info!("Starting API server on {}", api_bind_address);

    let api_server = ApiServer::new(api_bind_address);
    let connection_pool_clone = Arc::clone(&proxy.connection_pool);

    // Start API server in a separate task
    let api_task = tokio::spawn(async move {
        if let Err(e) = api_server.start(connection_pool_clone).await {
            tracing::error!("API server error: {}", e);
        }
    });

    tracing::info!("Servers started. Press Ctrl+C to stop.");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down...");

    // Cancel MCP server
    mcp_server.cancel();

    // Cancel API server task
    api_task.abort();

    // Disconnect from all servers
    {
        let mut pool = proxy.connection_pool.lock().await;
        pool.disconnect_all().await?;
        tracing::info!("Disconnected from all upstream servers");
    }

    Ok(())
}
