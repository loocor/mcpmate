// Stdio transport implementation for core
// Contains functions for connecting to stdio-based MCP servers

use anyhow::{Context, Result};
use rmcp::{
    RoleClient,
    model::{ServerCapabilities, Tool},
    service::{RunningService, serve_client_with_ct},
    transport::TokioChildProcess,
};
use sysinfo;
use tokio::{io::AsyncReadExt, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::core::foundation::utils::{
    get_connection_timeout, // connection timeout
    get_tools_timeout,      // tools timeout
    prepare_command,        // prepare command
};
use crate::core::models::MCPServerConfig;

/// Prepare and configure command with environment variables
async fn prepare_server_command(
    server_config: &MCPServerConfig,
    runtime_cache: Option<&crate::runtime::RuntimeCache>,
) -> Result<(tokio::process::Command, String)> {
    let command = server_config
        .command
        .as_ref()
        .context("Command not specified for stdio server")?;

    let transformed_command = crate::core::foundation::utils::transform_command(command);

    tracing::debug!(
        "Executing command: {} (transformed from: {})",
        transformed_command,
        command
    );

    let mut cmd = prepare_command(&transformed_command, server_config.args.as_ref());

    // Handle runtime cache if available
    if let Some(cache) = runtime_cache {
        if let Some(runtime_path) = cache.get_runtime_for_command(&transformed_command).await {
            tracing::debug!(
                "Using MCPMate managed runtime for command '{}' (transformed from '{}'): {}",
                transformed_command,
                command,
                runtime_path.display()
            );

            let runtime_command = runtime_path.to_string_lossy();
            cmd = prepare_command(&runtime_command, server_config.args.as_ref());
        } else {
            tracing::warn!(
                "MCPMate runtime for command '{}' (transformed from '{}') not available in .mcpmate/runtimes, falling back to system runtime",
                transformed_command,
                command
            );
        }
    } else {
        tracing::warn!("Runtime cache not available, using system runtime for command '{command}'");

        // Create necessary directories for runtime
        let paths = crate::common::paths::global_paths();
        paths
            .ensure_directories()
            .context("Failed to create necessary directories")?;
    }

    Ok((cmd, transformed_command))
}

/// Apply environment variables to command
async fn setup_command_environment(
    cmd: &mut tokio::process::Command,
    server_config: &MCPServerConfig,
    transformed_command: &str,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
) -> Result<()> {
    // Add environment variables if any
    if let Some(env) = &server_config.env {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    // Prepare environment variables based on runtime configuration
    if let Err(e) = crate::config::runtime::prepare_command_env_with_db(cmd, transformed_command, database_pool).await {
        tracing::warn!(
            "Failed to prepare environment for command '{}': {}",
            transformed_command,
            e
        );
        tracing::info!("Attempting to continue with default environment");
    } else {
        tracing::debug!("Environment prepared for command '{}'", transformed_command);
    }

    Ok(())
}

/// Connect to server with timeout handling
async fn connect_with_timeout(
    cmd: tokio::process::Command,
    ct: CancellationToken,
    server_name: &str,
    connection_timeout: std::time::Duration,
) -> Result<RunningService<RoleClient, ()>> {
    // Use builder to capture stderr for logging
    let (child_process, stderr_handle) = TokioChildProcess::builder(cmd)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            ct.cancel();
            anyhow::anyhow!("Failed to create child process: {e}")
        })?;

    // Spawn stderr monitoring task if stderr is available
    if let Some(mut stderr) = stderr_handle {
        let server_name = server_name.to_string();
        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            loop {
                match stderr.read(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]);
                        for line in output.lines() {
                            if !line.trim().is_empty() {
                                tracing::info!("Log from {}: {}", server_name, line.trim());
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Stderr read error for server '{}': {}", server_name, e);
                        break;
                    }
                }
            }
        });
    }

    timeout(connection_timeout, async {
        serve_client_with_ct((), child_process, ct.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to server: {e}"))
    })
    .await
    .map_err(|_| {
        ct.cancel();
        anyhow::anyhow!(
            "Connection timeout for server '{server_name}' after {}s",
            connection_timeout.as_secs()
        )
    })?
}

/// Get tools from service with timeout handling
async fn get_tools_with_timeout(
    service: &RunningService<RoleClient, ()>,
    server_name: &str,
    tools_timeout: std::time::Duration,
    ct: CancellationToken,
) -> Result<Vec<Tool>> {
    timeout(tools_timeout, service.list_all_tools())
        .await
        .map_err(|_| {
            ct.cancel();
            anyhow::anyhow!(
                "Timeout listing tools for server '{server_name}' after {}s",
                tools_timeout.as_secs()
            )
        })?
        .map_err(|e| anyhow::anyhow!("Failed to list tools: {e}"))
}

/// Cancel service with timeout to prevent hanging
async fn cancel_service_safely(service: RunningService<RoleClient, ()>) {
    match tokio::time::timeout(std::time::Duration::from_secs(3), service.cancel()).await {
        Ok(Ok(_)) => {
            tracing::debug!("Service cancelled successfully");
        }
        Ok(Err(cancel_err)) => {
            tracing::warn!("Error cancelling service: {}", cancel_err);
        }
        Err(_) => {
            tracing::warn!("Service cancellation timeout, resources may be leaked");
        }
    }
}

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
    // Prepare command and handle runtime cache
    let (mut cmd, transformed_command) = prepare_server_command(server_config, runtime_cache).await?;

    // Setup environment variables
    setup_command_environment(&mut cmd, server_config, &transformed_command, database_pool).await?;

    // Determine appropriate timeouts
    let command = server_config.command.as_ref().unwrap(); // Safe because prepare_server_command already checked
    let connection_timeout = get_connection_timeout(command);
    let tools_timeout = get_tools_timeout(command);

    tracing::debug!(
        "Using timeouts for '{}': connection={}s, tools={}s",
        server_name,
        connection_timeout.as_secs(),
        tools_timeout.as_secs()
    );

    // Connect to server with timeout handling
    let service = connect_with_timeout(cmd, ct.clone(), server_name, connection_timeout).await?;

    // Get tools with timeout handling
    let tools = match get_tools_with_timeout(&service, server_name, tools_timeout, ct.clone()).await {
        Ok(tools) => tools,
        Err(e) => {
            cancel_service_safely(service).await;
            return Err(e);
        }
    };

    // Get process ID and capabilities
    let pid = get_process_id_for_server(server_name, server_config).await;
    let capabilities = service.peer_info().map(|info| info.capabilities.clone());

    tracing::debug!(
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

/// Universal stdio server connection function with optional database and runtime cache support
pub async fn connect_stdio_server(
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
    connect_stdio_server_core(server_name, server_config, ct, database_pool, runtime_cache).await
}
