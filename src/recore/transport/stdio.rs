// Stdio transport implementation for recore
// Contains functions for connecting to stdio-based MCP servers

use anyhow::{Context, Result};
use rmcp::{
    RoleClient,
    model::{ServerCapabilities, Tool},
    service::{RunningService, serve_client_with_ct},
    transport::TokioChildProcess,
};
use sysinfo;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::recore::foundation::utils::{
    get_connection_timeout, // connection timeout
    get_tools_timeout,      // tools timeout
    prepare_command,        // prepare command
};
use crate::recore::models::MCPServerConfig;

/// Core connection function with customizable environment preparation
async fn connect_stdio_server_core(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    runtime_cache: Option<&crate::runtime::RuntimeCache>,
) -> Result<(
    RunningService<RoleClient, ()>,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>,
)> {
    // Get command and arguments
    let command = server_config
        .command
        .as_ref()
        .context("Command not specified for stdio server")?;

    // Log the command being executed
    tracing::debug!("Executing command: {}", command);

    // Create command with cross-platform support
    let mut cmd = prepare_command(command, server_config.args.as_ref());

    // Add environment variables if any
    if let Some(env) = &server_config.env {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    // Use RuntimeCache for fast runtime queries if available
    if let Some(cache) = runtime_cache {
        // Check if runtime is available in cache
        if let Some(runtime_path) = cache.get_runtime_for_command(command).await {
            tracing::info!(
                "Using MCPMate managed runtime for command '{}': {}",
                command,
                runtime_path.display()
            );

            // Update command to use the cached runtime path with cross-platform support
            let runtime_command = runtime_path.to_string_lossy();
            cmd = prepare_command(&runtime_command, server_config.args.as_ref());

            // Re-add environment variables if any
            if let Some(env) = &server_config.env {
                for (key, value) in env {
                    cmd.env(key, value);
                }
            }
        } else {
            tracing::info!(
                "MCPMate runtime for command '{}' not available in cache, falling back to system runtime",
                command
            );
        }
    } else {
        // Fallback to basic runtime detection
        tracing::info!("Runtime cache not available, using system runtime for command '{command}'");

        // Create necessary directories for runtime
        let paths = crate::common::paths::global_paths();
        paths
            .ensure_directories()
            .context("Failed to create necessary directories")?;
    }

    // Prepare environment variables based on runtime configuration
    if let Err(e) =
        crate::config::runtime::prepare_command_env_with_db(&mut cmd, command, database_pool).await
    {
        tracing::warn!("Failed to prepare environment for command '{command}': {e}");
        tracing::info!("Attempting to continue with default environment");
    } else {
        tracing::debug!("Environment prepared for command '{command}'");
    }

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
    let connect_result = match TokioChildProcess::new(cmd) {
        Ok(child_process) => {
            // Set a timeout for the connection process
            match timeout(connection_timeout, async {
                match serve_client_with_ct((), child_process, ct.clone()).await {
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
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create child process: {e}");
            return Err(anyhow::anyhow!(error_msg));
        }
    };

    // If connection was successful, get tools with timeout
    match connect_result {
        Ok(service) => {
            // Set a timeout for listing tools
            match timeout(tools_timeout, service.list_all_tools()).await {
                Ok(Ok(tools)) => {
                    // Try to get the process ID
                    let pid = get_process_id_for_server(server_name, server_config).await;

                    // Get server capabilities from peer info
                    let capabilities = service.peer_info().map(|info| info.capabilities.clone());

                    tracing::info!(
                        "Connected to server '{}', found {} tools, capabilities: {:?}, process ID: {:?}",
                        server_name,
                        tools.len(),
                        capabilities
                            .as_ref()
                            .map(|c| format!("resources={}", c.resources.is_some())),
                        pid
                    );

                    Ok((service, tools, capabilities, pid))
                }
                Ok(Err(e)) => {
                    let error_msg = format!("Failed to list tools: {e}");
                    // Make sure to cancel the service to avoid resource leaks
                    if let Err(cancel_err) = service.cancel().await {
                        tracing::warn!("Error cancelling service: {}", cancel_err);
                    }
                    Err(anyhow::anyhow!(error_msg))
                }
                Err(_) => {
                    let error_msg = format!("Timeout listing tools for server '{server_name}'");
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

/// Helper function to get process ID for a server
async fn get_process_id_for_server(
    server_name: &str,
    server_config: &MCPServerConfig,
) -> Option<u32> {
    // Wait a short time for the process to fully start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    if let Some(command) = &server_config.command {
        // Get the command name (last part of the path)
        let cmd_name = command.split('/').next_back().unwrap_or(command);

        // Create a new System instance
        let mut system = sysinfo::System::new_all();
        system.refresh_all();

        // Find the process by name
        for (pid, process) in system.processes() {
            if process.name() == cmd_name {
                tracing::debug!(
                    "Found process for server '{}': PID={}, name={}",
                    server_name,
                    pid,
                    process.name()
                );
                return Some(pid.as_u32());
            }
        }

        tracing::debug!(
            "Process not found for server '{}' with command '{}'",
            server_name,
            cmd_name
        );
    }

    None
}

/// Connect to a stdio server with database and runtime cache support
pub async fn connect_stdio_server_with_ct_and_db(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
) -> Result<(
    RunningService<RoleClient, ()>,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>,
)> {
    connect_stdio_server_core(server_name, server_config, ct, database_pool, None).await
}

/// Connect to a stdio server with runtime cache support
pub async fn connect_stdio_server_with_runtime_cache(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    runtime_cache: &crate::runtime::RuntimeCache,
) -> Result<(
    RunningService<RoleClient, ()>,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>,
)> {
    connect_stdio_server_core(
        server_name,
        server_config,
        ct,
        database_pool,
        Some(runtime_cache),
    )
    .await
}

/// Connect to a stdio server with basic cancellation token support
pub async fn connect_stdio_server_with_ct(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
) -> Result<(
    RunningService<RoleClient, ()>,
    Vec<Tool>,
    Option<ServerCapabilities>,
    Option<u32>,
)> {
    connect_stdio_server_core(server_name, server_config, ct, None, None).await
}
