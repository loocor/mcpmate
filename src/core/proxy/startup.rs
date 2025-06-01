use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};

use super::Args;

// Import required types and modules from our library crate
use crate::api::ApiServer;
use crate::config::server;
use crate::core::http::HttpProxyServer;
use crate::core::http::pool::UpstreamConnectionPool;
use crate::core::{ConnectionStatus, TransportType};

/// Start background connections to all configured servers
pub async fn start_background_connections(
    proxy: &HttpProxyServer,
    proxy_arc: Arc<HttpProxyServer>,
) -> Result<()> {
    // Get a reference to the connection pool
    let connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>> =
        Arc::clone(&proxy.connection_pool);
    let proxy_arc_clone = Arc::clone(&proxy_arc);

    // Connect to all servers in the background
    tokio::spawn(async move {
        // Wait for a short time to ensure the SSE server is started
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Connect to all servers
        let mut pool = connection_pool.lock().await;

        // Connect to all servers in parallel
        if let Err(e) = pool.connect_all().await {
            tracing::error!("Error in parallel connection process: {}", e);
        }

        // Get the total number of servers in the database
        let total_server_count_in_db = if let Some(db) = &proxy_arc_clone.database {
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

/// Start the MCP proxy server with specified transport
pub async fn start_proxy_server(
    proxy: &mut HttpProxyServer,
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

        // Start the unified server
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

        // Start the server with specific transport
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

        if let Err(e) = proxy.start(mcp_bind_address, path, transport_type).await {
            tracing::error!("Failed to start proxy server: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Start the API server
pub async fn start_api_server(
    proxy: &HttpProxyServer,
    args: &Args,
) -> Result<tokio::task::JoinHandle<()>> {
    // Start API server
    let api_bind_address: SocketAddr = format!("127.0.0.1:{}", args.api_port).parse()?;
    tracing::info!("Starting API server on {}", api_bind_address);

    let api_server = ApiServer::new(api_bind_address);
    let connection_pool_clone = Arc::clone(&proxy.connection_pool);
    let proxy_clone = Arc::new(proxy.clone());

    // Start API server in a separate task
    let api_task = tokio::spawn(async move {
        if let Err(e) = api_server
            .start(connection_pool_clone, Some(proxy_clone))
            .await
        {
            tracing::error!("API server error: {}", e);
        }
    });

    tracing::info!("API server started with HTTP proxy server reference");

    Ok(api_task)
}
