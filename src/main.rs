use anyhow::Result;
use clap::Parser;

use mcpmate::core::proxy::{
    Args,
    init::{setup_database, setup_logging, setup_proxy_server_with_params},
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

    // Setup database
    let db = setup_database().await?;

    // Setup proxy server with startup parameters
    let (mut proxy, proxy_arc) = setup_proxy_server_with_params(db, &startup_mode).await?;

    // Start background connections
    start_background_connections(&proxy, proxy_arc.clone()).await?;

    // Start proxy server
    start_proxy_server(&mut proxy, &args).await?;

    // Start API server
    let api_task = start_api_server(proxy_arc.clone(), &args).await?;

    tracing::info!("Servers started. Press Ctrl+C to stop.");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received Ctrl+C, shutting down...");

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
