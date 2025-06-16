// Unified transport interface for core
// Provides a single interface for connecting to any type of MCP server

use anyhow::Result;
use rmcp::{
    RoleClient,
    model::{ServerCapabilities, Tool},
    service::RunningService,
};
use tokio_util::sync::CancellationToken;

use super::{TransportType, http, sse, stdio};
use crate::common::server::ServerType;
use crate::core::models::MCPServerConfig;

/// Connect to any type of MCP server using the appropriate transport
pub async fn connect_server(
    server_name: &str,
    server_config: &MCPServerConfig,
    server_type: ServerType,
    transport_type: TransportType,
    ct: Option<CancellationToken>,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    runtime_cache: Option<&crate::runtime::RuntimeCache>,
) -> Result<(
    RunningService<RoleClient, ()>,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>, // Process ID (only for stdio)
)> {
    match server_type {
        ServerType::Stdio => {
            let ct = ct.unwrap_or_default();

            let result = if let Some(cache) = runtime_cache {
                stdio::connect_stdio_server_with_runtime_cache(
                    server_name,
                    server_config,
                    ct,
                    database_pool,
                    cache,
                )
                .await?
            } else {
                stdio::connect_stdio_server_with_ct_and_db(
                    server_name,
                    server_config,
                    ct,
                    database_pool,
                )
                .await?
            };

            Ok(result)
        }
        ServerType::Sse => {
            let (service, tools, capabilities) =
                sse::connect_sse_server(server_name, server_config).await?;
            Ok((service, tools, capabilities, None))
        }
        ServerType::StreamableHttp => {
            let (service, tools, capabilities) =
                http::connect_http_server(server_name, server_config, transport_type).await?;
            Ok((service, tools, capabilities, None))
        }
    }
}

/// Connect to a server with simplified interface (no runtime cache)
pub async fn connect_server_simple(
    server_name: &str,
    server_config: &MCPServerConfig,
    server_type: ServerType,
    transport_type: TransportType,
) -> Result<(
    RunningService<RoleClient, ()>,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>,
)> {
    connect_server(
        server_name,
        server_config,
        server_type,
        transport_type,
        None,
        None,
        None,
    )
    .await
}
