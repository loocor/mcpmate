//! Startup logic for core proxy server
//!
//! This module handles the startup and background connection management using core modules.

use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};

use super::{Args, ProxyServer};

// Import required types and modules from core and other modules
use crate::{
    config::server,
    core::{
        foundation::types::ConnectionStatus, // connection status
        pool::UpstreamConnectionPool,        // connection pool
        transport::TransportType,            // transport type
    },
};

/// Start background connections to all configured servers using core connection pool
pub async fn start_background_connections(
    proxy: &ProxyServer,
    proxy_arc: Arc<ProxyServer>,
) -> Result<()> {
    // Get a reference to the core connection pool
    let connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>> = Arc::clone(&proxy.connection_pool);
    let proxy_arc_clone = Arc::clone(&proxy_arc);

    // Connect to all servers in the background
    tokio::spawn(async move {
        // Wait for a short time to ensure the transport servers are started
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Connect to all servers using high-performance parallel method
        if let Err(e) = UpstreamConnectionPool::trigger_connect_all_parallel_new(connection_pool.clone()).await {
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
        tracing::info!("Connected to {}/{} upstream servers", connected_count, total_count);

        // Note: Capability sync is now handled after API server startup
        // This avoids the need for temporary redb files and ensures clean separation
        // between MCP protocol functionality (available immediately) and API caching (optimized later)
        tracing::info!("Background connections established. Capability sync will be handled by API server startup.");
    });

    Ok(())
}

/// Start the MCP proxy server with specified transport using core implementation
pub async fn start_proxy_server(
    proxy: &mut ProxyServer,
    args: &Args,
) -> Result<()> {
    // Start proxy server with specified transport
    let mcp_bind_address = format!("127.0.0.1:{}", args.mcp_port).parse()?;
    tracing::info!("🚀 MCP Proxy Server binding to address: {}", mcp_bind_address);
    tracing::info!("🔧 Using port from args.port: {}", args.mcp_port);

    // Check if using unified mode
    if args.transport == "unified" || args.transport == "uni" {
        tracing::info!(
            "Starting MCP proxy server on {} with unified transport (both SSE and Streamable HTTP)",
            mcp_bind_address
        );

        // Start the unified server using core implementation
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
                tracing::warn!("Unknown transport type: {}, defaulting to SSE", args.transport);
                TransportType::Sse
            }
        };

        tracing::info!(
            "Starting MCP proxy server on {} with transport type {:?}",
            mcp_bind_address,
            transport_type
        );

        // Start the server with specific transport using core implementation
        let path = match transport_type {
            TransportType::Sse => "/sse",
            TransportType::StreamableHttp => "/mcp", // Path for Streamable HTTP
            _ => "/sse",                             // Default
        };

        tracing::info!("Using path '{}' for transport type {:?}", path, transport_type);

        if let Err(e) = proxy.start(transport_type, mcp_bind_address).await {
            tracing::error!("Failed to start proxy server: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Start the API server
pub async fn start_api_server(
    proxy_arc: Arc<ProxyServer>,
    args: &Args,
) -> Result<tokio::task::JoinHandle<()>> {
    // Start API server
    let api_bind_address: SocketAddr = format!("127.0.0.1:{}", args.api_port).parse()?;
    tracing::info!("🚀 API Server binding to address: {}", api_bind_address);
    tracing::info!("🔧 Using port from args.api_port: {}", args.api_port);

    // Clone necessary references for the API server
    let connection_pool = Arc::clone(&proxy_arc.connection_pool);

    // Start the API server in a background task
    let api_task = tokio::spawn(async move {
        let api_server = crate::api::ApiServer::new(api_bind_address);

        if let Err(e) = api_server.start(connection_pool, Some(proxy_arc)).await {
            tracing::error!("API server failed: {}", e);
        }
    });

    tracing::info!("API server started successfully on {}", api_bind_address);
    Ok(api_task)
}
