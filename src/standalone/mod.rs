//! Standalone mode module
//!
//! This module encapsulates the original main.rs logic for standalone execution.
//! It provides the same functionality as the original binary but as a library function.

use anyhow::Result;
use clap::Parser;

use crate::core::proxy::{
    Args,
    init::{setup_database, setup_logging, setup_proxy_server},
    startup::{start_api_server, start_background_connections, start_proxy_server},
};

/// Run MCPMate in standalone mode (equivalent to the original main.rs)
pub async fn run_standalone_mode(args: Args) -> Result<()> {
    // Setup logging
    setup_logging(&args)?;

    // Setup database
    let db = setup_database().await?;

    // Setup proxy server
    let (mut proxy, proxy_arc) = setup_proxy_server(db).await?;

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

/// Run MCPMate in standalone mode with command line argument parsing
pub async fn run_standalone_with_args() -> Result<()> {
    let args = Args::parse();
    run_standalone_mode(args).await
}
