// Unified HTTP Server implementation
// Combines StreamableHttpServer and SseServer into a single server

use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, Result};
use axum::Router;
use rmcp::{
    RoleServer, Service,
    transport::{
        sse_server::{SseServer, SseServerConfig},
        streamable_http_server::axum::{StreamableHttpServer, StreamableHttpServerConfig},
    },
};
use tokio_util::sync::CancellationToken;
use tracing;

/// Configuration for the unified HTTP server
#[derive(Debug, Clone)]
pub struct UnifiedHttpServerConfig {
    /// Address to bind the server to
    pub bind_address: SocketAddr,
    /// Path for the Streamable HTTP endpoint
    pub streamable_http_path: String,
    /// Path for the SSE endpoint
    pub sse_path: String,
    /// Path for the SSE message endpoint
    pub sse_message_path: String,
    /// Keep-alive interval for SSE connections
    pub keep_alive_interval: Option<Duration>,
    /// Cancellation token for graceful shutdown
    pub cancellation_token: CancellationToken,
}

impl Default for UnifiedHttpServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8000".parse().unwrap(),
            streamable_http_path: "/mcp".to_string(),
            sse_path: "/sse".to_string(),
            sse_message_path: "/message".to_string(),
            keep_alive_interval: Some(Duration::from_secs(15)),
            cancellation_token: CancellationToken::new(),
        }
    }
}

/// Unified HTTP server that supports both Streamable HTTP and SSE
pub struct UnifiedHttpServer {
    /// Server configuration
    pub config: UnifiedHttpServerConfig,
}

impl Default for UnifiedHttpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedHttpServer {
    /// Create a new unified HTTP server with default configuration
    pub fn new() -> Self {
        Self::with_config(UnifiedHttpServerConfig::default())
    }

    /// Create a new unified HTTP server with custom configuration
    pub fn with_config(config: UnifiedHttpServerConfig) -> Self {
        Self { config }
    }

    /// Start the unified HTTP server with both Streamable HTTP and SSE endpoints
    ///
    /// This method starts a unified HTTP server that supports both Streamable HTTP and SSE protocols
    /// on the same port. It creates a combined Axum router that merges the routers from both
    /// server types, allowing them to share the same port.
    ///
    /// The server will handle requests at the following endpoints:
    /// - Streamable HTTP endpoint (specified by `config.streamable_http_path`, default: "/mcp")
    /// - SSE endpoint (specified by `config.sse_path`, default: "/sse")
    /// - SSE message endpoint (specified by `config.sse_message_path`, default: "/message")
    ///
    /// # Arguments
    /// * `service_factory` - A factory function that creates service instances for handling requests
    ///
    /// # Returns
    /// * `Result<()>` - Ok if the server started successfully, Err otherwise
    pub async fn start<F, S>(
        &self,
        service_factory: F,
    ) -> Result<()>
    where
        F: Fn() -> S + Clone + Send + Sync + 'static,
        S: Service<RoleServer> + Send + Sync + 'static,
    {
        tracing::info!(
            "Starting unified HTTP server on {} with Streamable HTTP at {} and SSE at {}",
            self.config.bind_address,
            self.config.streamable_http_path,
            self.config.sse_path
        );

        // Create Streamable HTTP server config
        let streamable_http_config = StreamableHttpServerConfig {
            bind: self.config.bind_address,
            path: self.config.streamable_http_path.clone(),
            ct: self.config.cancellation_token.clone(),
            sse_keep_alive: self.config.keep_alive_interval,
        };

        // Create SSE server config
        let sse_config = SseServerConfig {
            bind: self.config.bind_address,
            sse_path: self.config.sse_path.clone(),
            post_path: self.config.sse_message_path.clone(),
            ct: self.config.cancellation_token.clone(),
            sse_keep_alive: self.config.keep_alive_interval,
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
        let listener = tokio::net::TcpListener::bind(self.config.bind_address)
            .await
            .context(format!(
                "Failed to bind to address {}",
                self.config.bind_address
            ))?;

        let ct = self.config.cancellation_token.child_token();

        // Start the HTTP server with the combined router
        let server = axum::serve(listener, combined_router).with_graceful_shutdown(async move {
            ct.cancelled().await;
            tracing::info!("Unified HTTP server cancelled");
        });

        // Register the service with both servers
        tracing::info!("Registering service with Streamable HTTP server");
        let factory_clone = service_factory.clone();
        streamable_http_server.with_service(factory_clone);

        tracing::info!("Registering service with SSE server");
        sse_server.with_service(service_factory);

        // Start the server in a background task
        tokio::spawn(async move {
            if let Err(e) = server.await {
                tracing::error!(error = %e, "Unified HTTP server shutdown with error");
            }
        });

        tracing::info!("Unified HTTP server started successfully with the following endpoints:");
        tracing::info!(
            "  - Streamable HTTP: {}{}",
            self.config.bind_address,
            self.config.streamable_http_path
        );
        tracing::info!(
            "  - SSE: {}{}",
            self.config.bind_address,
            self.config.sse_path
        );
        tracing::info!(
            "  - SSE Message: {}{}",
            self.config.bind_address,
            self.config.sse_message_path
        );

        Ok(())
    }
}
