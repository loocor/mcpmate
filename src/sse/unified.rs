// Unified MCP Server implementation
// Combines StreamableHttpServer and SseServer into a single server

use anyhow::{Context, Result};
use axum::Router;
use rmcp::{
    transport::{
        sse_server::{SseServer, SseServerConfig},
        streamable_http_server::axum::{StreamableHttpServer, StreamableHttpServerConfig},
    },
    RoleServer, Service,
};
use std::{net::SocketAddr, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing;

/// Configuration for the unified MCP server
#[derive(Debug, Clone)]
pub struct UnifiedMcpServerConfig {
    /// Address to bind the server to
    pub bind: SocketAddr,
    /// Path for the Streamable HTTP endpoint
    pub streamable_http_path: String,
    /// Path for the SSE endpoint
    pub sse_path: String,
    /// Path for the SSE message endpoint
    pub sse_message_path: String,
    /// Keep-alive interval for SSE connections
    pub sse_keep_alive: Option<Duration>,
    /// Cancellation token for graceful shutdown
    pub ct: CancellationToken,
}

impl Default for UnifiedMcpServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8000".parse().unwrap(),
            streamable_http_path: "/mcp".to_string(),
            sse_path: "/sse".to_string(),
            sse_message_path: "/message".to_string(),
            sse_keep_alive: Some(Duration::from_secs(15)),
            ct: CancellationToken::new(),
        }
    }
}

/// Unified MCP server that supports both Streamable HTTP and SSE
pub struct UnifiedMcpServer {
    /// Server configuration
    pub config: UnifiedMcpServerConfig,
}

impl UnifiedMcpServer {
    /// Create a new unified MCP server with default configuration
    pub fn new() -> Self {
        Self::with_config(UnifiedMcpServerConfig::default())
    }

    /// Create a new unified MCP server with custom configuration
    pub fn with_config(config: UnifiedMcpServerConfig) -> Self {
        Self { config }
    }

    /// Start the unified MCP server with both Streamable HTTP and SSE endpoints
    pub async fn start<F, S>(&self, service_factory: F) -> Result<()>
    where
        F: Fn() -> S + Clone + Send + Sync + 'static,
        S: Service<RoleServer> + Send + Sync + 'static,
    {
        tracing::info!(
            "Starting unified MCP server on {} with Streamable HTTP at {} and SSE at {}",
            self.config.bind,
            self.config.streamable_http_path,
            self.config.sse_path
        );

        // Create Streamable HTTP server config
        let streamable_http_config = StreamableHttpServerConfig {
            bind: self.config.bind,
            path: self.config.streamable_http_path.clone(),
            ct: self.config.ct.clone(),
            sse_keep_alive: self.config.sse_keep_alive,
        };

        // Create SSE server config
        let sse_config = SseServerConfig {
            bind: self.config.bind,
            sse_path: self.config.sse_path.clone(),
            post_path: self.config.sse_message_path.clone(),
            ct: self.config.ct.clone(),
            sse_keep_alive: self.config.sse_keep_alive,
        };

        // Create both servers but don't start them yet
        let (streamable_http_server, streamable_http_router) =
            StreamableHttpServer::new(streamable_http_config);

        let (sse_server, sse_router) = SseServer::new(sse_config);

        // Merge the routers
        let combined_router = Router::new()
            .merge(streamable_http_router)
            .merge(sse_router);

        // Start the combined server
        let listener = tokio::net::TcpListener::bind(self.config.bind)
            .await
            .context("Failed to bind to address")?;

        let ct = self.config.ct.child_token();

        // Start the HTTP server with the combined router
        let server = axum::serve(listener, combined_router).with_graceful_shutdown(async move {
            ct.cancelled().await;
            tracing::info!("Unified MCP server cancelled");
        });

        // Register the service with both servers
        let factory_clone = service_factory.clone();
        streamable_http_server.with_service(factory_clone);
        sse_server.with_service(service_factory);

        // Start the server in a background task
        tokio::spawn(async move {
            if let Err(e) = server.await {
                tracing::error!(error = %e, "Unified MCP server shutdown with error");
            }
        });

        tracing::info!("Unified MCP server started successfully");
        Ok(())
    }
}
