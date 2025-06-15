// SSE transport implementation for recore
// Contains functions for connecting to SSE-based MCP servers

use anyhow::{Context, Result};
use rmcp::{
    RoleClient,
    model::{ServerCapabilities, Tool},
    service::{RunningService, ServiceExt},
    transport::SseClientTransport,
};
use tokio::time::timeout;

use crate::core::models::MCPServerConfig;
use crate::recore::foundation::utils::{
    get_sse_connection_timeout, // SSE connection timeout
    get_sse_service_timeout,    // SSE service timeout
    get_sse_tools_timeout,      // SSE tools timeout
};

/// Connect to an SSE server with timeout
pub async fn connect_sse_server(
    server_name: &str,
    server_config: &MCPServerConfig,
) -> Result<(
    RunningService<RoleClient, ()>,
    Vec<Tool>,
    Option<ServerCapabilities>,
)> {
    // Get URL
    let url = server_config
        .url
        .as_deref()
        .context("URL not specified for SSE server")?;

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

    // Connect to the server with timeout
    let transport_result = match timeout(connection_timeout, SseClientTransport::start(url)).await {
        Ok(Ok(transport)) => Ok(transport),
        Ok(Err(e)) => {
            let error_msg = format!("Failed to create SSE transport: {e}");
            Err(anyhow::anyhow!(error_msg))
        }
        Err(_) => {
            let error_msg = format!("Timeout creating SSE transport for server '{server_name}'");
            tracing::warn!("{}", error_msg);
            Err(anyhow::anyhow!(error_msg))
        }
    };

    // If transport creation was successful, serve and get tools with timeout
    match transport_result {
        Ok(transport) => {
            // Set a timeout for serving the transport
            let service_result = match timeout(service_timeout, async {
                match ().serve(transport).await {
                    Ok(service) => Ok(service),
                    Err(e) => {
                        let error_msg = format!("Failed to connect to server: {e}");
                        Err(anyhow::anyhow!(error_msg))
                    }
                }
            })
            .await
            {
                Ok(result) => result,
                Err(_) => {
                    let error_msg = format!("Connection timeout for server '{server_name}'");
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
                            // Get server capabilities from peer info
                            let capabilities =
                                service.peer_info().map(|info| info.capabilities.clone());

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
                        Ok(Err(e)) => {
                            let error_msg = format!("Failed to list tools: {e}");
                            Err(anyhow::anyhow!(error_msg))
                        }
                        Err(_) => {
                            let error_msg =
                                format!("Timeout listing tools for server '{server_name}'");
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
