//! Startup logic for core proxy server
//!
//! This module handles the startup and background connection management using core modules.

use super::{Args, ProxyServer};
use crate::core::{pool::UpstreamConnectionPool, transport::TransportType};
use crate::system::config::bind_socket_addr;
use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};

/// Start background connections to all configured servers using core connection pool
pub async fn start_background_connections(
    proxy: &ProxyServer,
    proxy_arc: Arc<ProxyServer>,
) -> Result<()> {
    // Get a reference to the core connection pool
    let connection_pool: Arc<tokio::sync::Mutex<UpstreamConnectionPool>> = Arc::clone(&proxy.connection_pool);
    let connection_pool_for_prewarm = Arc::clone(&connection_pool);
    let proxy_arc_for_prewarm = Arc::clone(&proxy_arc);

    // No eager connections at startup; keep instances registered as Idle placeholders.
    // Background task: delegate prewarm to capability service
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Log current registered servers as placeholders
        {
            let pool = connection_pool_for_prewarm.lock().await;
            let enabled_servers_count = pool.connections.len();
            tracing::info!(
                "Startup: {} enabled servers registered as placeholders (Idle). Will connect on demand.",
                enabled_servers_count
            );
        }

        if let Some(db) = proxy_arc_for_prewarm.database.clone() {
            let service = crate::core::capability::CapabilityService::new(
                proxy_arc_for_prewarm.redb_cache.clone(),
                connection_pool_for_prewarm.clone(),
                db.clone(),
            );
            if let Err(e) = service.prewarm_enabled_servers_if_cache_miss().await {
                tracing::warn!(error = %e, "Capability prewarm task failed");
            }
        } else {
            tracing::debug!("Database not set on proxy; skipping cache prewarm");
        }
    });

    // Background task: reap idle instances to keep pool lean
    let idle_pool = connection_pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let mut pool = idle_pool.lock().await;
            pool.reap_idle_instances().await;
        }
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
    let mcp_bind_address = bind_socket_addr(args.mcp_port)?;
    tracing::info!("MCP Proxy Server binding to address: {}", mcp_bind_address);
    tracing::info!("Using port from args.port: {}", args.mcp_port);

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
    use crate::common::constants::transport;

    match transport {
        t if t == transport::SSE => TransportType::StreamableHttp,
        t if t == transport::STREAMABLE_HTTP => TransportType::StreamableHttp,
        t if t == transport::STDIO => TransportType::Stdio,
        _ => {
            tracing::warn!("Unknown transport type: {}, defaulting to Streamable HTTP", transport);
            TransportType::StreamableHttp
        }
    }
}

/// Get endpoint path for transport type
fn get_transport_path(transport_type: &TransportType) -> &'static str {
    match transport_type {
        TransportType::StreamableHttp => "/mcp",
        TransportType::Stdio => "/mcp",
    }
}

/// Start the API server with graceful shutdown support
pub async fn start_api_server(
    proxy_arc: Arc<ProxyServer>,
    args: &Args,
) -> Result<(tokio::task::JoinHandle<()>, tokio_util::sync::CancellationToken)> {
    // Start API server
    let api_bind_address: SocketAddr = bind_socket_addr(args.api_port)?;
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
