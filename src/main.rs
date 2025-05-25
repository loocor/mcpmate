use anyhow::Result;
use clap::Parser;

mod proxy;

use proxy::{
    Args,
    init::{setup_database, setup_environment, setup_logging, setup_proxy_server},
    startup::{start_api_server, start_background_connections, start_proxy_server},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup logging
    setup_logging(&args)?;

    // Setup environment
    setup_environment()?;

    // Setup database
    let db = setup_database().await?;

    // Setup proxy server
    let (mut proxy, proxy_arc) = setup_proxy_server(db).await?;

    // Start background connections
    start_background_connections(&proxy, proxy_arc.clone()).await?;

    // Start proxy server
    start_proxy_server(&mut proxy, &args).await?;

    // Start API server
    let api_task = start_api_server(&proxy, &args).await?;

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
