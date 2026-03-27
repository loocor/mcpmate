use anyhow::Result;
use clap::Parser;

use mcpmate::core::proxy::{
    Args,
    init::{setup_audit_database, setup_database, setup_logging, setup_proxy_server_with_params},
    startup::{start_api_server, start_background_connections, start_proxy_server},
};
use mcpmate::system::config::init_port_config;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Validate command line arguments
    if let Err(e) = args.validate() {
        eprintln!("Invalid arguments: {}", e);
        std::process::exit(1);
    }

    // Get startup mode from arguments
    let startup_mode = args.get_startup_mode();
    tracing::info!("Starting MCPMate with mode: {:?}", startup_mode);

    // Initialize runtime port configuration from command line arguments
    init_port_config(args.api_port, args.mcp_port);

    // Setup logging
    setup_logging(&args)?;

    // Initialize metrics reporting
    mcpmate::core::foundation::monitor::initialize_metrics_reporting();

    // Setup database
    let db = setup_database().await?;
    let audit_db = setup_audit_database().await?;

    // Setup proxy server with startup parameters
    let (proxy_arc1, proxy_arc2) = setup_proxy_server_with_params(db, audit_db, &startup_mode).await?;

    // Start background connections
    start_background_connections(&proxy_arc1, proxy_arc2.clone()).await?;

    // Start proxy server - we need to get a mutable reference from Arc
    let mut proxy_clone = (*proxy_arc1).clone();
    let mcp_server_handle = start_proxy_server(&mut proxy_clone, &args).await?;

    // Start API server
    let (api_task, api_cancellation_token) = start_api_server(proxy_arc2.clone(), &args).await?;

    tracing::info!("Servers started. Press Ctrl+C to stop.");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down...");

    // Step 1: Initiate MCP server shutdown first
    tracing::info!("Step 1: Initiating MCP server shutdown...");
    proxy_clone.initiate_shutdown().await?;

    // Step 2: Wait for MCP server to complete gracefully (if handle is available)
    if let Some(mcp_handle) = mcp_server_handle {
        match tokio::time::timeout(std::time::Duration::from_secs(5), mcp_handle).await {
            Ok(Ok(Ok(()))) => {
                tracing::info!("MCP server shutdown completed successfully");
            }
            Ok(Ok(Err(e))) => {
                tracing::warn!("MCP server completed with error: {}", e);
            }
            Ok(Err(e)) => {
                tracing::warn!("MCP server task panicked: {}", e);
            }
            Err(_) => {
                tracing::warn!("MCP server shutdown timed out after 5 seconds");
            }
        }
    } else {
        tracing::info!("MCP server doesn't support graceful shutdown monitoring, proceeding...");
    }

    // Step 2: Shutdown API server after MCP server is done
    tracing::info!("Step 2: Initiating API server shutdown...");
    api_cancellation_token.cancel();

    match tokio::time::timeout(std::time::Duration::from_secs(5), api_task).await {
        Ok(Ok(())) => {
            tracing::info!("API server shutdown completed successfully");
        }
        Ok(Err(e)) => {
            tracing::warn!("API server task completed with error: {}", e);
        }
        Err(_) => {
            tracing::warn!("API server shutdown timed out after 5 seconds");
        }
    }

    // Step 4: Complete proxy server cleanup (connections, etc.)
    tracing::info!("Step 3: Completing proxy server cleanup...");
    proxy_clone.complete_shutdown().await?;

    Ok(())
}
