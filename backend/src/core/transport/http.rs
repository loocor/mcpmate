// HTTP transport implementation for core
// Provides abstractions for streamable HTTP transport

use super::TransportType;
use crate::common::constants::protocol;
use crate::common::http::make_streamable_config;
use crate::core::foundation::utils::{get_sse_service_timeout, get_sse_tools_timeout};
use crate::core::models::MCPServerConfig;
use anyhow::{Context, Result};
use rmcp::{
    RoleClient,
    model::{ServerCapabilities, Tool},
    transport::{IntoTransport, StreamableHttpClientTransport},
};
use std::time::Duration;
use tokio::time::timeout;

fn annotate_operation<T>(
    result: Result<T>,
    operation: &str,
    server_name: &str,
) -> Result<T> {
    result.with_context(|| format!("{operation} failed for server '{server_name}'"))
}

fn build_configured_http_client(server_config: &MCPServerConfig) -> Result<reqwest::Client> {
    let mut header_map = reqwest::header::HeaderMap::new();
    if let Some(headers) = server_config.headers.as_ref() {
        for (key, value) in headers {
            if let (Ok(name), Ok(value)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                let controlled = matches!(
                    name.as_str().to_ascii_lowercase().as_str(),
                    "accept"
                        | "content-length"
                        | "host"
                        | "connection"
                        | "transfer-encoding"
                        | protocol::MCP_PROTOCOL_VERSION_HEADER_LOWER
                );
                if !controlled {
                    header_map.insert(name, value);
                }
            }
        }
    }
    reqwest::Client::builder()
        .default_headers(header_map)
        .build()
        .context("Failed to build configured HTTP client")
}

pub(crate) async fn connect_http_initialized_for_validation(
    server_name: &str,
    server_config: &MCPServerConfig,
    transport_type: TransportType,
    cancellation: tokio_util::sync::CancellationToken,
) -> Result<crate::core::transport::ClientService> {
    let url = server_config
        .url
        .as_ref()
        .context("URL not specified for HTTP server")?;

    match transport_type {
        TransportType::StreamableHttp => {
            let config = make_streamable_config(url, &server_config.headers);
            let transport = StreamableHttpClientTransport::<reqwest::Client>::with_client(
                build_configured_http_client(server_config)?,
                config,
            );
            build_initialized_service_with_cancellation(server_name, transport, cancellation, get_sse_service_timeout())
                .await
        }
        TransportType::Stdio => anyhow::bail!("Stdio transport not supported by this function"),
    }
}

/// Build RunningService and fetch tools with standard timeout handling
async fn build_service_tools<T, E, A>(
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
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let service = build_initialized_service(server_name, transport, service_timeout).await?;

    // Fetch tools
    let tools = timeout(tools_timeout, service.list_all_tools())
        .await
        .map_err(|_| anyhow::anyhow!(format!("Timeout listing tools for server '{server_name}'")))?;
    let tools = annotate_operation(tools.map_err(anyhow::Error::from), "tools/list", server_name)?;

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

async fn build_initialized_service<T, E, A>(
    server_name: &str,
    transport: T,
    service_timeout: std::time::Duration,
) -> Result<crate::core::transport::ClientService>
where
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    build_initialized_service_with_cancellation(server_name, transport, Default::default(), service_timeout).await
}

async fn build_initialized_service_with_cancellation<T, E, A>(
    server_name: &str,
    transport: T,
    cancellation: tokio_util::sync::CancellationToken,
    service_timeout: std::time::Duration,
) -> Result<crate::core::transport::ClientService>
where
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let service = crate::core::transport::unified::initialize_client_service(
        server_name,
        transport,
        cancellation,
        service_timeout,
    )
    .await;

    annotate_operation(service, "initialize/connect", server_name)
}

/// Connect to a streamable HTTP server with timeout
pub async fn connect_http_server(
    server_name: &str,
    server_config: &MCPServerConfig,
    transport_type: TransportType,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
)> {
    let client = build_configured_http_client(server_config)?;
    connect_http_server_with_client(server_name, server_config, client, transport_type).await
}

/// Connect to a streamable HTTP server with provided reqwest client
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

    let service_timeout = get_sse_service_timeout();
    let tools_timeout = get_sse_tools_timeout();

    let (service, tools, capabilities) = match transport_type {
        TransportType::StreamableHttp => {
            let config = make_streamable_config(url, &server_config.headers);
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

/// Connect to a streamable HTTP server with custom timeouts
pub async fn connect_http_server_with_client_timeouts(
    server_name: &str,
    server_config: &MCPServerConfig,
    client: reqwest::Client,
    transport_type: TransportType,
    connection_timeout: Duration,
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
        TransportType::StreamableHttp => {
            let config = make_streamable_config(url, &server_config.headers);
            let transport = StreamableHttpClientTransport::<reqwest::Client>::with_client(client, config);
            build_service_tools(server_name, transport, connection_timeout, tools_timeout).await?
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

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use rmcp::{
        ErrorData, RoleServer, ServerHandler, ServiceExt,
        model::{ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo},
        service::RequestContext,
    };
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    use super::{
        TransportType, annotate_operation, build_initialized_service, build_service_tools,
        connect_http_initialized_for_validation,
    };
    use crate::{common::server::ServerType, core::models::MCPServerConfig};

    #[derive(Clone)]
    struct FailingToolsServer {
        tools_list_calls: Arc<AtomicUsize>,
    }

    impl ServerHandler for FailingToolsServer {
        fn get_info(&self) -> ServerInfo {
            ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
        }

        async fn list_tools(
            &self,
            _request: Option<PaginatedRequestParams>,
            _context: RequestContext<RoleServer>,
        ) -> Result<ListToolsResult, ErrorData> {
            self.tools_list_calls.fetch_add(1, Ordering::SeqCst);
            Err(ErrorData::internal_error("tools/list is unavailable", None))
        }
    }

    async fn spawn_failing_tools_server(
        tools_list_calls: Arc<AtomicUsize>
    ) -> (tokio::io::DuplexStream, tokio::task::JoinHandle<anyhow::Result<()>>) {
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server_handle = tokio::spawn(async move {
            let service = FailingToolsServer { tools_list_calls }.serve(server_transport).await?;
            service.waiting().await?;
            Ok(())
        });

        (client_transport, server_handle)
    }

    #[test]
    fn protocol_errors_include_the_preview_operation_name() {
        for operation in ["initialize/connect", "tools/list"] {
            let error = annotate_operation::<()>(Err(anyhow::anyhow!("protocol failure")), operation, "docs")
                .expect_err("protocol failure must remain visible");

            assert!(error.to_string().contains(operation));
            assert!(error.to_string().contains("docs"));
        }
    }

    #[tokio::test]
    async fn validation_initialization_skips_tools_while_production_bootstrap_lists_once() {
        let tools_list_calls = Arc::new(AtomicUsize::new(0));
        let (validation_transport, validation_server) = spawn_failing_tools_server(tools_list_calls.clone()).await;

        let validation_service =
            build_initialized_service("validation", validation_transport, std::time::Duration::from_secs(1))
                .await
                .expect("initialize validation owner");

        assert!(validation_service.peer_info().is_some());
        assert_eq!(tools_list_calls.load(Ordering::SeqCst), 0);

        validation_service.cancel().await.expect("cancel validation owner");
        validation_server
            .await
            .expect("join validation server")
            .expect("validation server shutdown");

        let (production_transport, production_server) = spawn_failing_tools_server(tools_list_calls.clone()).await;
        let result = build_service_tools(
            "production",
            production_transport,
            std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(tools_list_calls.load(Ordering::SeqCst), 1);
        production_server
            .await
            .expect("join production server")
            .expect("production server shutdown");
    }

    #[tokio::test]
    async fn validation_initialization_preserves_configured_http_headers() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let config = MCPServerConfig {
            kind: ServerType::StreamableHttp,
            command: None,
            args: None,
            url: Some(server.uri()),
            env: None,
            headers: Some(HashMap::from([
                ("Authorization".to_string(), "Basic dXNlcjpwYXNz".to_string()),
                ("X-API-Key".to_string(), "api-secret".to_string()),
                ("X-Tenant-ID".to_string(), "tenant-a".to_string()),
            ])),
        };

        let result = connect_http_initialized_for_validation(
            "header-fixture",
            &config,
            TransportType::StreamableHttp,
            Default::default(),
        )
        .await;

        assert!(result.is_err(), "fixture intentionally rejects initialize");
        let requests = server.received_requests().await.expect("read captured requests");
        let request = requests.first().expect("initialize request");
        let has_header = |expected_name: &str, expected_value: &str| {
            request.headers.iter().any(|(name, values)| {
                name.as_str() == expected_name && values.iter().any(|value| value.as_str() == expected_value)
            })
        };
        assert!(has_header("authorization", "Basic dXNlcjpwYXNz"));
        assert!(has_header("x-api-key", "api-secret"));
        assert!(has_header("x-tenant-id", "tenant-a"));
    }
}
