use anyhow::Result;
use clap::Parser;
use mcp_client::config::{load_rule_config, load_server_config};
use rmcp::{
    model::{ServerCapabilities, ServerInfo},
    tool,
    transport::sse_server::{SseServer, SseServerConfig},
    ServerHandler,
};
use std::path::PathBuf;
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

    /// Port to listen on
    #[arg(short, long, default_value = "8000")]
    port: u16,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

/// MCP Proxy Server that aggregates tools from multiple MCP servers
#[derive(Debug, Clone)]
struct ProxyServer {
    // This will be expanded in later phases
}

#[tool(tool_box)]
impl ProxyServer {
    pub fn new() -> Self {
        Self {}
    }
}

#[tool(tool_box)]
impl ServerHandler for ProxyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCP Proxy Server that aggregates tools from multiple MCP servers".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
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

    // TODO: Load the MCP server and rule configuration
    let config = load_server_config(&args.config)?;
    let _rule_config = load_rule_config(&args.rule_config)?;

    // TODO: Log the loaded configuration
    tracing::info!("Loaded configuration from: {}", args.config.display());
    tracing::info!(
        "Found {} MCP servers in configuration",
        config.mcp_servers.len()
    );
    tracing::info!(
        "Loaded rule configuration from: {}",
        args.rule_config.display()
    );

    // Create proxy server
    let proxy = ProxyServer::new();

    // Start SSE server
    let bind_address = format!("127.0.0.1:{}", args.port).parse()?;
    tracing::info!("Starting SSE server on {}", bind_address);

    let server_config = SseServerConfig {
        bind: bind_address,
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: Default::default(),
    };

    let server = SseServer::serve_with_config(server_config)
        .await?
        .with_service(move || proxy.clone());

    tracing::info!("Server started. Press Ctrl+C to stop.");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down...");
    server.cancel();

    Ok(())
}
