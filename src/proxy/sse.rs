// MCP Proxy connection functions
// Contains functions for connecting to different types of MCP servers

use anyhow::{Context, Result};
use rmcp::{
    model::Tool,
    service::{RunningService, ServiceExt},
    transport::sse::SseTransport,
    RoleClient,
};
use std::time::Duration;
use tokio::time::timeout;

use crate::config::ServerConfig;

/// Connect to an SSE server with timeout
pub async fn connect_sse_server(
    server_name: &str,
    server_config: &ServerConfig,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>)> {
    // Get URL
    let url = server_config
        .url
        .as_ref()
        .context("URL not specified for SSE server")?;

    // Connect to the server with timeout
    let transport_result = match timeout(Duration::from_secs(30), SseTransport::start(url)).await {
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
    };

    // If transport creation was successful, serve and get tools with timeout
    match transport_result {
        Ok(transport) => {
            // Set a timeout for serving the transport
            let service_result = match timeout(Duration::from_secs(30), async {
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
                    match timeout(
                        Duration::from_secs(20),
                        service.list_tools(Default::default()),
                    )
                    .await
                    {
                        Ok(Ok(tools_result)) => {
                            tracing::info!(
                                "Connected to server '{}', found {} tools",
                                server_name,
                                tools_result.tools.len()
                            );
                            Ok((service, tools_result.tools))
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
