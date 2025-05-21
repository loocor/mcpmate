// Transport implementations for the HTTP proxy server

use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Result};

use crate::http::proxy::core::HttpProxyServer;

/// Create a service factory function that returns a new HttpProxyServer instance
///
/// This helper method is used by all server start methods to create a factory function
/// that returns a new HttpProxyServer instance for handling requests.
pub fn create_service_factory(
    server: &HttpProxyServer
) -> impl Fn() -> HttpProxyServer + Clone + Send + Sync + 'static {
    let proxy_clone = server.clone();
    move || proxy_clone.clone()
}

/// Start the SSE server
///
/// This method starts an SSE server on the specified address and path.
/// The server will handle Server-Sent Events (SSE) connections from clients
/// and route tool calls to the appropriate upstream servers.
///
/// # Arguments
/// * `server` - The HTTP proxy server instance
/// * `bind_address` - The socket address to bind the server to
/// * `sse_path` - The path for the SSE endpoint (e.g., "/sse")
///
/// # Returns
/// * `Result<()>` - Ok if the server started successfully, Err otherwise
pub async fn start_sse(
    server: &HttpProxyServer,
    bind_address: SocketAddr,
    sse_path: &str,
) -> Result<()> {
    tracing::info!(
        "Configuring SSE server on {} at path {}",
        bind_address,
        sse_path
    );

    // Create SSE server config
    let server_config = rmcp::transport::sse_server::SseServerConfig {
        bind: bind_address,
        sse_path: sse_path.to_string(),
        post_path: "/message".to_string(),
        ct: Default::default(),
        sse_keep_alive: Some(Duration::from_secs(15)),
    };

    // Create a factory function
    let factory = create_service_factory(server);

    // Start the SSE server
    tracing::info!("Starting SSE server...");
    let server = rmcp::transport::sse_server::SseServer::serve_with_config(server_config)
        .await
        .context("Failed to start SSE server")?;

    // Register our service with the server
    server.with_service(factory);

    tracing::info!(
        "Successfully started SSE server on {} at path {} with message path /message",
        bind_address,
        sse_path
    );

    // Publish server ready event
    crate::core::events::EventBus::global().publish(
        crate::core::events::Event::ServerTransportReady {
            transport_type: crate::core::transport::TransportType::Sse,
            ready: true,
        },
    );
    tracing::debug!("Published SSE server ready event");

    Ok(())
}

/// Start the Streamable HTTP server
///
/// This method starts a Streamable HTTP server on the specified address.
/// The server will handle Streamable HTTP connections from clients
/// and route tool calls to the appropriate upstream servers.
///
/// # Arguments
/// * `server` - The HTTP proxy server instance
/// * `bind_address` - The socket address to bind the server to
/// * `path` - The path for the Streamable HTTP endpoint (e.g., "/mcp")
///
/// # Returns
/// * `Result<()>` - Ok if the server started successfully, Err otherwise
pub async fn start_streamable_http(
    server: &HttpProxyServer,
    bind_address: SocketAddr,
    path: &str,
) -> Result<()> {
    // For Streamable HTTP, we use the specified path
    tracing::info!(
        "Configuring Streamable HTTP server on {} at path {}",
        bind_address,
        path
    );

    // Create a factory function
    let factory = create_service_factory(server);

    // Create Streamable HTTP server config
    let server_config = rmcp::transport::streamable_http_server::axum::StreamableHttpServerConfig {
        bind: bind_address,
        path: path.to_string(),
        ct: Default::default(),
        sse_keep_alive: Some(Duration::from_secs(15)),
    };

    // Start the Streamable HTTP server
    tracing::info!("Starting Streamable HTTP server...");
    let server =
        rmcp::transport::streamable_http_server::axum::StreamableHttpServer::serve_with_config(
            server_config,
        )
        .await
        .context("Failed to start Streamable HTTP server")?;

    // Register our service with the server
    server.with_service(factory);

    tracing::info!(
        "Successfully started Streamable HTTP server on {} at path {}",
        bind_address,
        path
    );

    // Publish server ready event
    crate::core::events::EventBus::global().publish(
        crate::core::events::Event::ServerTransportReady {
            transport_type: crate::core::transport::TransportType::StreamableHttp,
            ready: true,
        },
    );
    tracing::debug!("Published Streamable HTTP server ready event");

    Ok(())
}

/// Start the proxy server with both Streamable HTTP and SSE support
///
/// This method starts a unified HTTP server that supports both Streamable HTTP and SSE protocols
/// on the same port. It uses the following endpoints:
/// - `/mcp` - Streamable HTTP endpoint (2025-03-26 MCP specification)
/// - `/sse` - SSE endpoint (2024-11-05 MCP specification)
/// - `/message` - SSE message endpoint (2024-11-05 MCP specification)
///
/// This is the recommended way to start the server, as it provides maximum compatibility
/// with different client implementations.
///
/// # Arguments
/// * `server` - The HTTP proxy server instance
/// * `bind_address` - The socket address to bind the server to
///
/// # Returns
/// * `Result<()>` - Ok if the server started successfully, Err otherwise
pub async fn start_unified(
    server: &HttpProxyServer,
    bind_address: SocketAddr,
) -> Result<()> {
    tracing::info!(
        "Starting unified HTTP server on {} with both Streamable HTTP and SSE support",
        bind_address
    );

    // Import the UnifiedHttpServer
    use crate::http::unified::{UnifiedHttpServer, UnifiedHttpServerConfig};

    // Create unified server config with standard MCP endpoints
    let config = UnifiedHttpServerConfig {
        bind_address,
        streamable_http_path: "/mcp".to_string(), // 2025-03-26 spec endpoint
        sse_path: "/sse".to_string(),             // 2024-11-05 spec endpoint
        sse_message_path: "/message".to_string(), // 2024-11-05 spec endpoint
        keep_alive_interval: Some(Duration::from_secs(15)),
        cancellation_token: Default::default(),
    };

    // Create a factory function
    let factory = create_service_factory(server);

    // Create and start the unified server
    let server = UnifiedHttpServer::with_config(config);
    server
        .start(factory)
        .await
        .context("Failed to start unified HTTP server")?;

    tracing::info!(
        "Successfully started unified HTTP server on {} with endpoints /mcp, /sse, and /message",
        bind_address
    );
    Ok(())
}
