//! Startup logic for recore proxy server
//!
//! This module handles the startup and background connection management using recore modules.

use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};

use super::{Args, ProxyServer};

// Import required types and modules from recore and other modules
use crate::{
    config::server,
    recore::{
        foundation::types::ConnectionStatus, //
        pool::UpstreamConnectionPool,        //
        transport::TransportType,            //
    },
};

/// Start background connections to all configured servers using recore connection pool
pub async fn start_background_connections(
    proxy: &ProxyServer,
    proxy_arc: Arc<ProxyServer>,
) -> Result<()> {
    // Get a reference to the recore connection pool
    let connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>> =
        Arc::clone(&proxy.connection_pool);
    let proxy_arc_clone = Arc::clone(&proxy_arc);

    // Connect to all servers in the background
    tokio::spawn(async move {
        // Wait for a short time to ensure the transport servers are started
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Connect to all servers using high-performance parallel method
        if let Err(e) =
            UpstreamConnectionPool::trigger_connect_all_parallel_new(connection_pool.clone()).await
        {
            tracing::error!("Error in parallel connection process: {}", e);
        }

        // Get pool reference for status reporting
        let pool = connection_pool.lock().await;

        // Get the total number of servers in the database
        let total_server_count_in_db = if let Some(db) = proxy_arc_clone.database.as_ref() {
            match server::get_all_servers(&db.pool).await {
                Ok(servers) => servers.len(),
                Err(e) => {
                    tracing::error!("Failed to get servers from database: {}", e);
                    0 // If failed, use 0 and we'll fall back to connections.len() below
                }
            }
        } else {
            0 // If database not available, use 0
        };

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

        // Use database count if available, otherwise fall back to connections length
        let total_count = if total_server_count_in_db > 0 {
            total_server_count_in_db
        } else {
            pool.connections.len()
        };

        // Display system-wide server count, showing ratio of connected vs all configured servers
        // This is consistent with the /api/system/status endpoint which shows total_servers as all servers in the system
        tracing::info!(
            "Connected to {}/{} upstream servers",
            connected_count,
            total_count
        );
    });

    Ok(())
}

/// Start the MCP proxy server with specified transport using recore implementation
pub async fn start_proxy_server(
    proxy: &mut ProxyServer,
    args: &Args,
) -> Result<()> {
    // Start proxy server with specified transport
    let mcp_bind_address = format!("127.0.0.1:{}", args.port).parse()?;

    // Check if using unified mode
    if args.transport == "unified" || args.transport == "uni" {
        tracing::info!(
            "Starting MCP proxy server on {} with unified transport (both SSE and Streamable HTTP)",
            mcp_bind_address
        );

        // Start the unified server using recore implementation
        if let Err(e) = proxy.start_unified(mcp_bind_address).await {
            tracing::error!("Failed to start unified proxy server: {}", e);
            return Err(e);
        }
    } else {
        // Parse transport type for non-unified mode
        let transport_type = match args.transport.as_str() {
            "sse" => TransportType::Sse,
            "streamable_http" | "streamablehttp" | "str" => TransportType::StreamableHttp,
            _ => {
                tracing::warn!(
                    "Unknown transport type: {}, defaulting to SSE",
                    args.transport
                );
                TransportType::Sse
            }
        };

        tracing::info!(
            "Starting MCP proxy server on {} with transport type {:?}",
            mcp_bind_address,
            transport_type
        );

        // Start the server with specific transport using recore implementation
        let path = match transport_type {
            TransportType::Sse => "/sse",
            TransportType::StreamableHttp => "/mcp", // Path for Streamable HTTP
            _ => "/sse",                             // Default
        };

        tracing::info!(
            "Using path '{}' for transport type {:?}",
            path,
            transport_type
        );

        if let Err(e) = proxy.start(transport_type, mcp_bind_address).await {
            tracing::error!("Failed to start proxy server: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Start the API server (temporarily disabled due to type incompatibility)
pub async fn start_api_server(
    _proxy: &ProxyServer,
    args: &Args,
) -> Result<tokio::task::JoinHandle<()>> {
    // Start API server
    let api_bind_address: SocketAddr = format!("127.0.0.1:{}", args.api_port).parse()?;
    tracing::info!("Starting API server on {}", api_bind_address);

    // TODO: API server integration with recore
    // The API server currently expects core::UpstreamConnectionPool, but we have recore::UpstreamConnectionPool
    // This will need to be resolved when we migrate the API module or create an adapter

    tracing::warn!(
        "API server temporarily disabled - type incompatibility between core and recore connection pools"
    );
    tracing::warn!("API functionality will be restored after API module migration");

    // Return a dummy task for now
    let api_task = tokio::spawn(async move {
        tracing::info!("API server placeholder task running");
        // Keep the task alive but do nothing
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });

    Ok(api_task)
}
