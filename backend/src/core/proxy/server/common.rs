use anyhow::{Context, Result};
use rmcp::{
    RoleServer, Service,
    transport::{
        StreamableHttpServerConfig, StreamableHttpService, streamable_http_server::session::local::LocalSessionManager,
    },
};

/// Determine whether a server declares a given capability token
pub fn supports_capability(
    capabilities: Option<&str>,
    kind: crate::core::capability::CapabilityType,
) -> bool {
    let token = match kind {
        crate::core::capability::CapabilityType::Tools => crate::common::capability::CapabilityToken::Tools.as_str(),
        crate::core::capability::CapabilityType::Prompts => {
            crate::common::capability::CapabilityToken::Prompts.as_str()
        }
        crate::core::capability::CapabilityType::Resources
        | crate::core::capability::CapabilityType::ResourceTemplates => {
            crate::common::capability::CapabilityToken::Resources.as_str()
        }
    };

    crate::core::capability::facade::capability_declared(capabilities, token)
}

#[derive(Debug, Clone)]
pub struct UnifiedHttpServerConfig {
    pub bind_address: std::net::SocketAddr,
    pub streamable_http_path: String,
    pub keep_alive_interval: Option<std::time::Duration>,
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

impl Default for UnifiedHttpServerConfig {
    fn default() -> Self {
        use crate::common::constants::ports;
        Self {
            bind_address: format!("127.0.0.1:{}", ports::MCP_PORT).parse().unwrap(),
            streamable_http_path: "/mcp".to_string(),
            keep_alive_interval: Some(std::time::Duration::from_secs(15)),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        }
    }
}

/// Unified HTTP server that exposes only the streamable HTTP endpoint
pub struct UnifiedHttpServer {
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

    /// Start the unified HTTP server with the streamable HTTP endpoint
    pub async fn start<F, S>(
        &self,
        service_factory: F,
    ) -> Result<()>
    where
        F: Fn() -> S + Clone + Send + Sync + 'static,
        S: Service<RoleServer> + Send + Sync + 'static,
    {
        tracing::info!(
            "Starting unified HTTP server on {} with Streamable HTTP at {}",
            self.config.bind_address,
            self.config.streamable_http_path,
        );

        let streamable_http_config = StreamableHttpServerConfig {
            sse_keep_alive: self.config.keep_alive_interval,
            sse_retry: Some(std::time::Duration::from_secs(3)),
            stateful_mode: true,
            json_response: false,
            cancellation_token: self.config.cancellation_token.clone(),
        };

        let session_manager = std::sync::Arc::new(LocalSessionManager::default());

        let service_factory_clone = service_factory.clone();
        let streamable_http_service = StreamableHttpService::new(
            move || Ok(service_factory_clone()),
            session_manager,
            streamable_http_config,
        );

        let combined_router =
            axum::Router::new().route_service(&self.config.streamable_http_path, streamable_http_service);

        let listener = tokio::net::TcpListener::bind(self.config.bind_address)
            .await
            .context(format!("Failed to bind to address {}", self.config.bind_address))?;

        let ct = self.config.cancellation_token.child_token();

        let server = axum::serve(listener, combined_router).with_graceful_shutdown(async move {
            ct.cancelled().await;
            tracing::info!("Unified HTTP server cancelled");
        });

        let _ = service_factory;

        tokio::spawn(async move {
            if let Err(e) = server.await {
                tracing::error!(error = %e, "Unified HTTP server shutdown with error");
            }
        });

        tracing::info!("Unified HTTP server started successfully with the following endpoint:");
        tracing::info!(
            "  - Streamable HTTP: {}{}",
            self.config.bind_address,
            self.config.streamable_http_path
        );

        Ok(())
    }
}
