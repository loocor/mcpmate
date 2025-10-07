// HTTP transport implementation for core
// Provides abstractions for HTTP-based transport types (SSE and Streamable HTTP)

use super::TransportType;
use crate::core::foundation::utils::{get_sse_connection_timeout, get_sse_service_timeout, get_sse_tools_timeout};
use crate::core::models::MCPServerConfig;
use anyhow::{Context, Result};
use rmcp::{
    model::{ServerCapabilities, Tool},
    service::ServiceExt,
    transport::sse_client::SseClientConfig,
    transport::streamable_http_client::StreamableHttpClientTransportConfig,
    transport::{SseClientTransport, StreamableHttpClientTransport},
};
use std::time::Duration;
use tokio::time::timeout;

/// Internal helper used by both SSE and Streamable HTTP connections
async fn connect_http_internal(
    server_name: &str,
    server_config: &MCPServerConfig,
    transport_type: TransportType,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
)> {
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
        "Using timeouts for server '{}': connection={}s, service={}s, tools={}s",
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
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
)>
where
    T: rmcp::transport::Transport<rmcp::RoleClient> + Send + 'static,
{
    // Serve transport with timeout
    // server_name is a display label (e.g., "Gitmcp (SERVxxxx)") provided by the caller
    let handler = crate::core::transport::client::UpstreamClientHandler::new(server_name.to_string());
    let service = timeout(service_timeout, async { handler.serve(transport).await })
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
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
)> {
    let began = std::time::Instant::now();

    // If default headers are configured, build a client with those headers and reuse the with_client path
    if let Some(hdrs) = server_config.headers.as_ref() {
        if !hdrs.is_empty() {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (k, v) in hdrs.iter() {
                if let (Ok(name), Ok(value)) = (
                    reqwest::header::HeaderName::from_bytes(k.as_bytes()),
                    reqwest::header::HeaderValue::from_str(v),
                ) {
                    // Skip controlled headers that transport layer will manage itself
                    let controlled = matches!(name.as_str().to_ascii_lowercase().as_str(),
                        "accept" | "content-length" | "host" | "connection" | "transfer-encoding" | "mcp-protocol-version"
                    );
                    if controlled { continue; }
                    header_map.insert(name, value);
                }
            }
            let client = reqwest::Client::builder().default_headers(header_map).build()?;
            return connect_http_server_with_client(server_name, server_config, client, transport_type).await;
        }
    }

    let res = connect_http_internal(server_name, server_config, transport_type).await;
    if let Ok((_, ref tools, _)) = res {
        tracing::debug!(
            "[HTTP CONNECT][no-reuse] server={} tools={} elapsed_ms={}",
            server_name,
            tools.len(),
            began.elapsed().as_millis()
        );
    }
    res
}

/// Connect to an HTTP-based server (SSE or Streamable HTTP) with provided reqwest client
pub async fn connect_http_server_with_client(
    server_name: &str,
    server_config: &MCPServerConfig,
    client: reqwest::Client,
    transport_type: TransportType,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
)> {
    let began = std::time::Instant::now();
    let url = server_config
        .url
        .as_ref()
        .context("URL not specified for HTTP server")?;

    let connection_timeout = get_sse_connection_timeout();
    let service_timeout = get_sse_service_timeout();
    let tools_timeout = get_sse_tools_timeout();

    let (service, tools, capabilities) = match transport_type {
        TransportType::Sse => {
            // Start SSE with injected client
            let transport = tokio::time::timeout(connection_timeout, async move {
                SseClientTransport::start_with_client(
                    client,
                    SseClientConfig {
                        sse_endpoint: url.clone().into(),
                        ..Default::default()
                    },
                )
                .await
            })
            .await
            .map_err(|_| anyhow::anyhow!(format!("Timeout creating SSE transport for server '{server_name}'")))??;
            build_service_tools(server_name, transport, service_timeout, tools_timeout).await?
        }
        TransportType::StreamableHttp => {
            // Create Streamable HTTP transport with injected client
            let config = StreamableHttpClientTransportConfig {
                uri: url.clone().into(),
                ..Default::default()
            };
            let transport = StreamableHttpClientTransport::<reqwest::Client>::with_client(client, config);
            build_service_tools(server_name, transport, service_timeout, tools_timeout).await?
        }
        TransportType::Stdio => {
            return Err(anyhow::anyhow!("Stdio transport not supported by this function"));
        }
    };

    let elapsed = began.elapsed().as_millis();
    tracing::debug!(
        "[HTTP CONNECT][reuse] server={} transport={:?} tools={} elapsed_ms={}",
        server_name,
        transport_type,
        tools.len(),
        elapsed
    );
    Ok((service, tools, capabilities))
}

/// Connect to an HTTP-based server (SSE or Streamable HTTP) with custom timeouts
pub async fn connect_http_server_with_client_timeouts(
    server_name: &str,
    server_config: &MCPServerConfig,
    client: reqwest::Client,
    transport_type: TransportType,
    connection_timeout: Duration,
    service_timeout: Duration,
    tools_timeout: Duration,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
)> {
    let began = std::time::Instant::now();
    let url = server_config
        .url
        .as_ref()
        .context("URL not specified for HTTP server")?;

    let (service, tools, capabilities) = match transport_type {
        TransportType::Sse => {
            let transport = tokio::time::timeout(connection_timeout, async move {
                SseClientTransport::start_with_client(
                    client,
                    SseClientConfig {
                        sse_endpoint: url.clone().into(),
                        ..Default::default()
                    },
                )
                .await
            })
            .await
            .map_err(|_| anyhow::anyhow!(format!("Timeout creating SSE transport for server '{server_name}'")))??;
            build_service_tools(server_name, transport, service_timeout, tools_timeout).await?
        }
        TransportType::StreamableHttp => {
            let config = StreamableHttpClientTransportConfig {
                uri: url.clone().into(),
                ..Default::default()
            };
            let transport = StreamableHttpClientTransport::<reqwest::Client>::with_client(client, config);
            build_service_tools(server_name, transport, service_timeout, tools_timeout).await?
        }
        TransportType::Stdio => {
            anyhow::bail!("HTTP timeouts not applicable for stdio transport");
        }
    };

    tracing::debug!(
        "[HTTP CONNECT][custom] server={} tools={} elapsed_ms={}",
        server_name,
        tools.len(),
        began.elapsed().as_millis()
    );

    Ok((service, tools, capabilities))
}

/// Connect specifically to an SSE server – maintained for backward compatibility
pub async fn connect_sse_server(
    server_name: &str,
    server_config: &MCPServerConfig,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
)> {
    connect_http_internal(server_name, server_config, TransportType::Sse).await
}
