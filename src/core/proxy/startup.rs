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
    });

    Ok(())
}

/// Start the MCP proxy server with specified transport using core implementation
/// Returns a JoinHandle for the MCP server that can be awaited for graceful shutdown
pub async fn start_proxy_server(
    proxy: &mut ProxyServer,
    args: &Args,
) -> Result<Option<tokio::task::JoinHandle<Result<(), anyhow::Error>>>> {
    // Start proxy server with specified transport
    let mcp_bind_address = format!("127.0.0.1:{}", args.mcp_port).parse()?;
    tracing::info!("🚀 MCP Proxy Server binding to address: {}", mcp_bind_address);
    tracing::info!("🔧 Using port from args.port: {}", args.mcp_port);

    // Start server based on transport mode using early return pattern
    match args.transport.as_str() {
        "unified" | "uni" => start_unified_server(proxy, mcp_bind_address).await,
        transport => start_single_transport_server(proxy, mcp_bind_address, transport).await,
    }
}

/// Start unified server (both SSE and Streamable HTTP)
async fn start_unified_server(
    proxy: &mut ProxyServer,
    bind_address: SocketAddr,
) -> Result<Option<tokio::task::JoinHandle<Result<(), anyhow::Error>>>> {
    tracing::info!(
        "Starting MCP proxy server on {} with unified transport (both SSE and Streamable HTTP)",
        bind_address
    );

    let server_handle = proxy.start_unified(bind_address).await.map_err(|e| {
        tracing::error!("Failed to start unified proxy server: {}", e);
        e
    })?;

    Ok(Some(server_handle))
}

/// Start single transport server (SSE or Streamable HTTP)
async fn start_single_transport_server(
    proxy: &mut ProxyServer,
    bind_address: SocketAddr,
    transport: &str,
) -> Result<Option<tokio::task::JoinHandle<Result<(), anyhow::Error>>>> {
    let transport_type = parse_transport_type(transport);

    tracing::info!(
        "Starting MCP proxy server on {} with transport type {:?}",
        bind_address,
        transport_type
    );

    let path = get_transport_path(&transport_type);
    tracing::info!("Using path '{}' for transport type {:?}", path, transport_type);

    proxy.start(transport_type, bind_address).await.map_err(|e| {
        tracing::error!("Failed to start proxy server: {}", e);
        e
    })?;

    // Non-unified mode doesn't return a JoinHandle yet
    Ok(None)
}

/// Parse transport type from string with fallback
fn parse_transport_type(transport: &str) -> TransportType {
    match transport {
        "sse" => TransportType::Sse,
        "streamable_http" | "streamablehttp" | "str" => TransportType::StreamableHttp,
        _ => {
            tracing::warn!("Unknown transport type: {}, defaulting to SSE", transport);
            TransportType::Sse
        }
    }
}

/// Get endpoint path for transport type
fn get_transport_path(transport_type: &TransportType) -> &'static str {
    match transport_type {
        TransportType::StreamableHttp => "/mcp",
        TransportType::Sse => "/sse",
        _ => "/sse",
    }
}

/// Start the API server with graceful shutdown support
pub async fn start_api_server(
    proxy_arc: Arc<ProxyServer>,
    args: &Args,
) -> Result<(tokio::task::JoinHandle<()>, tokio_util::sync::CancellationToken)> {
    // Start API server
    let api_bind_address: SocketAddr = format!("127.0.0.1:{}", args.api_port).parse()?;
    tracing::info!("🚀 API Server binding to address: {}", api_bind_address);
    tracing::info!("🔧 Using port from args.api_port: {}", args.api_port);

    // Clone necessary references for the API server
    let connection_pool = Arc::clone(&proxy_arc.connection_pool);

    // Create cancellation token for API server
    let api_cancellation_token = tokio_util::sync::CancellationToken::new();
    let api_token_clone = api_cancellation_token.clone();

    // Start the API server in a background task
    let api_task = tokio::spawn(async move {
        let api_server = crate::api::ApiServer::new(api_bind_address);

        if let Err(e) = api_server
            .start_with_shutdown(connection_pool, Some(proxy_arc), api_token_clone)
            .await
        {
            tracing::error!("API server failed: {}", e);
        }
    });

    tracing::info!("API server started successfully on {}", api_bind_address);
    Ok((api_task, api_cancellation_token))
}
