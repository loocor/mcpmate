// MCP Proxy connection functions
// Contains functions for connecting to different types of MCP servers

use anyhow::{Context, Result};
use rmcp::{
    model::Tool,
    service::{serve_client_with_ct, RunningService},
    transport::TokioChildProcess,
    RoleClient,
};
use tokio::{process::Command, time::timeout};
use tokio_util::sync::CancellationToken;

use super::utils::{get_connection_timeout, get_tools_timeout, prepare_command_env};
use crate::config::ServerConfig;

/// Connect to a stdio server with timeout and cancellation token
pub async fn connect_stdio_server_with_ct(
    server_name: &str,
    server_config: &ServerConfig,
    ct: CancellationToken,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>)> {
    // Get command and arguments
    let command = server_config
        .command
        .as_ref()
        .context("Command not specified for stdio server")?;

    // Create command
    let mut cmd = Command::new(command);

    // Add arguments if any
    if let Some(args) = &server_config.args {
        cmd.args(args);
    }

    // Add environment variables if any
    if let Some(env) = &server_config.env {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    // Prepare command environment (important for Docker, npx, etc.)
    prepare_command_env(&mut cmd, command);

    // Determine appropriate timeout based on command type
    let connection_timeout = get_connection_timeout(command);
    let tools_timeout = get_tools_timeout(command);

    tracing::info!(
        "Using timeouts for '{}': connection={}s, tools={}s",
        server_name,
        connection_timeout.as_secs(),
        tools_timeout.as_secs()
    );

    // Connect to the server with timeout
    let connect_result = match TokioChildProcess::new(&mut cmd) {
        Ok(child_process) => {
            // Set a timeout for the connection process
            match timeout(connection_timeout, async {
                match serve_client_with_ct((), child_process, ct.clone()).await {
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
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create child process: {}", e);
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    // If connection was successful, get tools with timeout
    match connect_result {
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
                    // Make sure to cancel the service to avoid resource leaks
                    if let Err(cancel_err) = service.cancel().await {
                        tracing::warn!("Error cancelling service: {}", cancel_err);
                    }
                    Err(anyhow::anyhow!(error_msg))
                }
                Err(_) => {
                    let error_msg = format!("Timeout listing tools for server '{}'", server_name);
                    tracing::warn!("{}", error_msg);
                    // Make sure to cancel the service to avoid resource leaks
                    if let Err(cancel_err) = service.cancel().await {
                        tracing::warn!("Error cancelling service: {}", cancel_err);
                    }
                    Err(anyhow::anyhow!(error_msg))
                }
            }
        }
        Err(e) => Err(e),
    }
}

/// Connect to a stdio server with timeout (backward compatibility wrapper)
pub async fn connect_stdio_server(
    server_name: &str,
    server_config: &ServerConfig,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>)> {
    connect_stdio_server_with_ct(server_name, server_config, CancellationToken::new()).await
}
