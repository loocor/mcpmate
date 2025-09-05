// HTTP transport implementation for core
// Provides abstractions for HTTP-based transport types (SSE and Streamable HTTP)

use super::TransportType;
use crate::core::foundation::utils::{get_sse_connection_timeout, get_sse_service_timeout, get_sse_tools_timeout};
use crate::core::models::MCPServerConfig;
use anyhow::{Context, Result};
use rmcp::{
    RoleClient,
    model::{ServerCapabilities, Tool},
    service::{RunningService, ServiceExt},
    transport::{SseClientTransport, StreamableHttpClientTransport},
};
use tokio::time::timeout;

/// Internal helper used by both SSE and Streamable HTTP connections
async fn connect_http_internal(
    server_name: &str,
    server_config: &MCPServerConfig,
    transport_type: TransportType,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>, Option<ServerCapabilities>)> {
    // Reuse previous implementation (moved from old connect_http_server body)
    // Get URL
    let url = server_config
        .url
        .as_ref()
        .context("URL not specified for HTTP server")?;

    // Get timeouts
    let connection_timeout = get_sse_connection_timeout();
    let service_timeout = get_sse_service_timeout();
    let tools_timeout = get_sse_tools_timeout();

    tracing::debug!(
        "Using timeouts for '{}': connection={}s, service={}s, tools={}s",
        server_name,
        connection_timeout.as_secs(),
        service_timeout.as_secs(),
        tools_timeout.as_secs()
    );

    // Branch per transport type to build service and tools
    let (service, tools, capabilities) = match transport_type {
        TransportType::Sse => {
            let transport = timeout(connection_timeout, SseClientTransport::start(url.as_str()))
                .await
                .map_err(|_| anyhow::anyhow!(format!("Timeout creating SSE transport for server '{server_name}'")))?
                .map_err(|e| anyhow::anyhow!(format!("Failed to create SSE transport: {e}")))?;

            build_service_tools(server_name, transport, service_timeout, tools_timeout).await?
        }
        TransportType::StreamableHttp => {
            // Create transport immediately (no async connect needed)
            let transport = StreamableHttpClientTransport::<reqwest::Client>::from_uri(url.as_str());
            build_service_tools(server_name, transport, service_timeout, tools_timeout).await?
        }
        TransportType::Stdio => {
            return Err(anyhow::anyhow!("Stdio transport not supported by this function"));
        }
    };

    Ok((service, tools, capabilities))
}

/// Build RunningService and fetch tools with standard timeout handling
async fn build_service_tools<T>(
    server_name: &str,
    transport: T,
    service_timeout: std::time::Duration,
    tools_timeout: std::time::Duration,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>, Option<ServerCapabilities>)>
where
    T: rmcp::transport::Transport<RoleClient> + Send + 'static,
{
    // Serve transport with timeout
    let service = timeout(service_timeout, async { ().serve(transport).await })
        .await
        .map_err(|_| anyhow::anyhow!(format!("Connection timeout for server '{server_name}'")))??;

    // Fetch tools
    let tools = timeout(tools_timeout, service.list_all_tools())
        .await
        .map_err(|_| anyhow::anyhow!(format!("Timeout listing tools for server '{server_name}'")))??;

    let capabilities = service.peer_info().map(|info| info.capabilities.clone());

    tracing::info!(
        "Connected to server '{}', found {} tools, capabilities: {:?}",
        server_name,
        tools.len(),
        capabilities
            .as_ref()
            .map(|c| format!("resources={}", c.resources.is_some()))
    );

    Ok((service, tools, capabilities))
}

/// Connect to an HTTP-based server (SSE or Streamable HTTP) with timeout
pub async fn connect_http_server(
    server_name: &str,
    server_config: &MCPServerConfig,
    transport_type: TransportType,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>, Option<ServerCapabilities>)> {
    connect_http_internal(server_name, server_config, transport_type).await
}

/// Connect specifically to an SSE server – maintained for backward compatibility
pub async fn connect_sse_server(
    server_name: &str,
    server_config: &MCPServerConfig,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>, Option<ServerCapabilities>)> {
    connect_http_internal(server_name, server_config, TransportType::Sse).await
}
