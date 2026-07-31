// Unified transport interface for core
// Provides a single interface for connecting to any type of MCP server

use anyhow::{Context, Result};
use rmcp::{
    RoleClient,
    model::{ServerCapabilities, Tool},
    service::ServiceExt,
    transport::IntoTransport,
};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use super::{TransportType, http, stdio};
use crate::common::server::ServerType;
use crate::core::models::MCPServerConfig;

pub(crate) async fn initialize_client_service<T, E, A>(
    server_name: &str,
    transport: T,
    cancellation: CancellationToken,
    connection_timeout: std::time::Duration,
) -> Result<crate::core::transport::ClientService>
where
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let handler = crate::core::transport::client::UpstreamClientHandler::new(server_name.to_string());
    let service = timeout(
        connection_timeout,
        handler.serve_with_ct(transport, cancellation.clone()),
    )
    .await
    .map_err(|_| {
        cancellation.cancel();
        anyhow::Error::new(super::timeout_policy::ServerStartupTimeout {
            timeout_ms: connection_timeout.as_millis(),
        })
        .context(format!("Failed to start server '{server_name}'"))
    })?
    .map_err(anyhow::Error::new)
    .with_context(|| format!("Failed to initialize server '{server_name}'"))?;

    anyhow::ensure!(
        service.peer_info().is_some(),
        "Server '{server_name}' completed initialize without peer information"
    );
    service
        .set_response_cache_config(rmcp::ClientCacheConfig::disabled())
        .await;

    Ok(service)
}

pub(crate) async fn connect_server_initialized_for_validation(
    server_name: &str,
    server_config: &MCPServerConfig,
    server_type: ServerType,
    transport_type: TransportType,
    ct: Option<CancellationToken>,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    startup_timeout: std::time::Duration,
) -> Result<crate::core::transport::ClientService> {
    match server_type {
        ServerType::Stdio => {
            stdio::connect_stdio_initialized_for_validation(
                server_name,
                server_config,
                ct.unwrap_or_default(),
                database_pool,
                startup_timeout,
            )
            .await
        }
        ServerType::Sse | ServerType::StreamableHttp => {
            http::connect_http_initialized_for_validation(
                server_name,
                server_config,
                transport_type,
                ct.unwrap_or_default(),
                startup_timeout,
            )
            .await
        }
    }
}

/// Connect to any type of MCP server using the appropriate transport
pub async fn connect_server(
    server_name: &str,
    server_config: &MCPServerConfig,
    server_type: ServerType,
    transport_type: TransportType,
    ct: Option<CancellationToken>,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>, // Process ID (only for stdio)
)> {
    match server_type {
        ServerType::Stdio => {
            let ct = ct.unwrap_or_default();

            let result = stdio::connect_stdio_server(server_name, server_config, ct, database_pool).await?;

            Ok(result)
        }
        ServerType::Sse | ServerType::StreamableHttp => {
            let (service, tools, capabilities) =
                http::connect_http_server(server_name, server_config, transport_type).await?;
            Ok((service, tools, capabilities, None))
        }
    }
}

/// Connect to a server with simplified interface
pub async fn connect_server_simple(
    server_name: &str,
    server_config: &MCPServerConfig,
    server_type: ServerType,
    transport_type: TransportType,
) -> Result<(
    crate::core::transport::ClientService,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>,
)> {
    connect_server(server_name, server_config, server_type, transport_type, None, None).await
}

#[cfg(test)]
mod tests {
    use rmcp::{
        ClientCacheConfig, ServerHandler, ServiceExt,
        model::{ServerCapabilities, ServerInfo},
    };
    use tokio_util::sync::CancellationToken;

    use super::initialize_client_service;

    #[derive(Clone)]
    struct TestServer;

    impl ServerHandler for TestServer {
        fn get_info(&self) -> ServerInfo {
            ServerInfo::new(ServerCapabilities::default())
        }
    }

    #[tokio::test]
    async fn initialized_client_disables_rmcp_response_cache() {
        let (server_transport, client_transport) = tokio::io::duplex(4096);
        let server = tokio::spawn(async move {
            let service = TestServer.serve(server_transport).await.expect("serve test server");
            service.waiting().await.expect("wait for test server");
        });

        let service = initialize_client_service(
            "cache-policy",
            client_transport,
            CancellationToken::new(),
            std::time::Duration::from_secs(1),
        )
        .await
        .expect("initialize client");

        assert_eq!(service.response_cache_config().await, ClientCacheConfig::disabled());

        service.cancel().await.expect("cancel client");
        server.await.expect("join test server");
    }
}
