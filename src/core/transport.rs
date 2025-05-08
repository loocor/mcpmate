// Transport factory for MCP proxy
// Provides abstractions for different transport types

use anyhow::{Context, Result};
use rmcp::{
    model::Tool,
    service::{RunningService, ServiceExt},
    transport::sse::SseTransport,
    RoleClient,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::time::timeout;

use super::config::ServerConfig;
use super::utils::{get_sse_connection_timeout, get_sse_service_timeout, get_sse_tools_timeout};

/// Transport types supported by the proxy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    /// Server-Sent Events transport
    Sse,
    /// Streamable HTTP transport
    StreamableHttp,
    /// Standard input/output transport
    Stdio,
}

impl Default for TransportType {
    fn default() -> Self {
        Self::Sse // Default to SSE for backward compatibility
    }
}

impl FromStr for TransportType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sse" => Ok(Self::Sse),
            "streamable_http" | "streamablehttp" => Ok(Self::StreamableHttp),
            "stdio" => Ok(Self::Stdio),
            _ => Err(anyhow::anyhow!("Unknown transport type: {}", s)),
        }
    }
}

/// Connect to an HTTP-based server (SSE or Streamable HTTP) with timeout
pub async fn connect_http_server(
    server_name: &str,
    server_config: &ServerConfig,
    transport_type: TransportType,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>)> {
    // Get URL
    let url = server_config
        .url
        .as_ref()
        .context("URL not specified for HTTP server")?;

    // Get timeouts
    let connection_timeout = get_sse_connection_timeout();
    let service_timeout = get_sse_service_timeout();
    let tools_timeout = get_sse_tools_timeout();

    tracing::info!(
        "Using timeouts for '{}': connection={}s, service={}s, tools={}s",
        server_name,
        connection_timeout.as_secs(),
        service_timeout.as_secs(),
        tools_timeout.as_secs()
    );

    // Connect to the server with timeout based on transport type
    let transport_result = match transport_type {
        TransportType::Sse => match timeout(connection_timeout, SseTransport::start(url)).await {
            Ok(Ok(transport)) => Ok(transport),
            Ok(Err(e)) => {
                let error_msg = format!("Failed to create SSE transport: {}", e);
                Err(anyhow::anyhow!(error_msg))
            }
            Err(_) => {
                let error_msg = format!(
                    "Timeout creating SSE transport for server '{}'",
                    server_name
                );
                tracing::warn!("{}", error_msg);
                Err(anyhow::anyhow!(error_msg))
            }
        },
        TransportType::StreamableHttp => {
            // Currently, we don't have a separate StreamableHttpTransport in the client
            // So we use SseTransport for both SSE and Streamable HTTP
            match timeout(connection_timeout, SseTransport::start(url)).await {
                Ok(Ok(transport)) => Ok(transport),
                Ok(Err(e)) => {
                    let error_msg = format!("Failed to create Streamable HTTP transport: {}", e);
                    Err(anyhow::anyhow!(error_msg))
                }
                Err(_) => {
                    let error_msg = format!(
                        "Timeout creating Streamable HTTP transport for server '{}'",
                        server_name
                    );
                    tracing::warn!("{}", error_msg);
                    Err(anyhow::anyhow!(error_msg))
                }
            }
        }
        TransportType::Stdio => Err(anyhow::anyhow!(
            "Stdio transport not supported by this function"
        )),
    };

    // If transport creation was successful, serve and get tools with timeout
    match transport_result {
        Ok(transport) => {
            // Set a timeout for serving the transport
            let service_result = match timeout(service_timeout, async {
                match ().serve(transport).await {
                    Ok(service) => Ok(service),
                    Err(e) => {
                        let error_msg = format!("Failed to connect to server: {}", e);
                        Err(anyhow::anyhow!(error_msg))
                    }
                }
            })
            .await
            {
                Ok(result) => result,
                Err(_) => {
                    let error_msg = format!("Connection timeout for server '{}'", server_name);
                    tracing::warn!("{}", error_msg);
                    return Err(anyhow::anyhow!(error_msg));
                }
            };

            // If service creation was successful, get tools with timeout
            match service_result {
                Ok(service) => {
                    // Set a timeout for listing tools
                    match timeout(tools_timeout, service.list_all_tools()).await {
                        Ok(Ok(tools)) => {
                            tracing::info!(
                                "Connected to server '{}', found {} tools",
                                server_name,
                                tools.len()
                            );
                            Ok((service, tools))
                        }
                        Ok(Err(e)) => {
                            let error_msg = format!("Failed to list tools: {}", e);
                            Err(anyhow::anyhow!(error_msg))
                        }
                        Err(_) => {
                            let error_msg =
                                format!("Timeout listing tools for server '{}'", server_name);
                            tracing::warn!("{}", error_msg);
                            Err(anyhow::anyhow!(error_msg))
                        }
                    }
                }
                Err(e) => Err(e),
            }
        }
        Err(e) => Err(e),
    }
}
