// MCP Proxy module
// Contains functions and utilities for the MCP proxy server

use anyhow::{Context, Result};
use rmcp::{
    model::Tool,
    service::{RunningService, ServiceExt},
    transport::{sse::SseTransport, TokioChildProcess},
    RoleClient,
};
use std::{env, time::Duration};
use tokio::{process::Command, time::timeout};

use crate::config::ServerConfig;

/// Connection status for an upstream server
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Server is connected and operational
    Connected,
    /// Server is disconnected
    Disconnected,
    /// Server is in the process of connecting
    Connecting,
    /// Server connection failed with an error
    Failed(String),
}

/// Prepare command environment variables for different command types
pub fn prepare_command_env(command: &mut Command, command_str: &str) {
    // 1. bin path
    let bin_var = match command_str {
        "npx" => "NPX_BIN_PATH",
        "uvx" => "UVX_BIN_PATH",
        "docker" => "DOCKER_BIN_PATH",
        _ => "MCP_RUNTIME_BIN",
    };
    let bin_path = env::var(bin_var)
        .or_else(|_| env::var("MCP_RUNTIME_BIN"))
        .ok();
    if let Some(bin_path) = bin_path {
        let old_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_path, old_path);
        command.env("PATH", new_path);
    }

    // 2. cache env
    let cache_var = match command_str {
        "npx" => "NPM_CONFIG_CACHE",
        "uvx" => "UV_CACHE_DIR",
        _ => "",
    };
    if !cache_var.is_empty() {
        if let Ok(cache_val) = env::var(cache_var) {
            command.env(cache_var, cache_val);
        }
    }

    // 3. Docker specific settings
    if command_str == "docker" {
        // Set DOCKER_HOST if available
        if let Ok(docker_host) = env::var("DOCKER_HOST") {
            command.env("DOCKER_HOST", docker_host);
        }
    }
}

/// Connect to a stdio server with timeout
pub async fn connect_stdio_server(
    server_name: &str,
    server_config: &ServerConfig,
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
    let connection_timeout = match command.as_str() {
        "docker" => Duration::from_secs(120), // Docker operations can take longer
        "npx" => Duration::from_secs(60),     // npm operations can take time
        _ => Duration::from_secs(30),         // Default timeout
    };

    let tools_timeout = match command.as_str() {
        "docker" => Duration::from_secs(60), // Docker operations can take longer
        "npx" => Duration::from_secs(30),    // npm operations can take time
        _ => Duration::from_secs(20),        // Default timeout
    };

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
                match ().serve(child_process).await {
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
            match timeout(tools_timeout, service.list_tools(Default::default())).await {
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
