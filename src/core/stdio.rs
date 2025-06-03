// MCP Proxy connection functions
// Contains functions for connecting to different types of MCP servers

use anyhow::{Context, Result};
use rmcp::{
    RoleClient,
    model::Tool,
    service::{RunningService, serve_client_with_ct},
    transport::TokioChildProcess,
};
use sysinfo;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use super::utils::{get_connection_timeout, get_tools_timeout};
use crate::core::models::MCPServerConfig;

/// Core connection function with customizable environment preparation
async fn connect_stdio_server_core(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    runtime_cache: Option<&crate::runtime::RuntimeCache>,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>, Option<u32>)> {
    // Get command and arguments
    let command = server_config
        .command
        .as_ref()
        .context("Command not specified for stdio server")?;

    // Log the command being executed
    tracing::debug!("Executing command: {}", command);

    // Create command with cross-platform support
    let mut cmd = crate::core::utils::prepare_command(command, server_config.args.as_ref());

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
            cmd =
                crate::core::utils::prepare_command(&runtime_command, server_config.args.as_ref());

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

                    tracing::info!(
                        "Connected to server '{}', found {} tools, process ID: {:?}",
                        server_name,
                        tools.len(),
                        pid
                    );

                    Ok((service, tools, pid))
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

        // Find processes that were started recently (within the last 5 seconds)
        // and match our command name or command line
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let matching_processes: Vec<_> = system
            .processes()
            .iter()
            .filter(|(_, process)| {
                // Check if process was started recently (within 5 seconds)
                let process_start_time = process.start_time();
                let age_secs = now.saturating_sub(process_start_time);
                if age_secs > 5 {
                    return false;
                }

                // Check if process name or command line contains our command
                let process_name = process.name().to_lowercase();
                let cmd_name_lower = cmd_name.to_lowercase();

                // Check process name
                if process_name.contains(&cmd_name_lower) {
                    return true;
                }

                // Check command line
                let cmd_line = process.cmd();
                for arg in cmd_line {
                    if arg.to_lowercase().contains(&cmd_name_lower) {
                        return true;
                    }
                }

                false
            })
            .collect();

        // Log all matching processes for debugging
        for (pid, process) in &matching_processes {
            tracing::debug!(
                "Found matching process: PID={}, name={}, cmd={:?}",
                pid.as_u32(),
                process.name(),
                process.cmd()
            );
        }

        // If we found matching processes, use the most recently started one
        if !matching_processes.is_empty() {
            // Sort by start time (newest first)
            let mut sorted_processes = matching_processes.clone();
            sorted_processes.sort_by(|(_, a), (_, b)| b.start_time().cmp(&a.start_time()));

            // Use the newest process
            let (pid, process) = sorted_processes[0];
            tracing::info!(
                "Using process PID={} name={} for server '{}'",
                pid.as_u32(),
                process.name(),
                server_name
            );
            Some(pid.as_u32())
        } else {
            tracing::warn!(
                "No matching processes found for command '{}' for server '{}'",
                cmd_name,
                server_name
            );
            None
        }
    } else {
        tracing::warn!(
            "No command specified for server '{}', cannot determine PID",
            server_name
        );
        None
    }
}

/// Connect to a stdio server with timeout, cancellation token, and optional database pool
/// This version allows database-assisted runtime environment configuration
pub async fn connect_stdio_server_with_ct_and_db(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>, Option<u32>)> {
    connect_stdio_server_core(server_name, server_config, ct, database_pool, None).await
}

/// Connect to a stdio server with runtime cache support
/// This version uses RuntimeCache for fast runtime queries
pub async fn connect_stdio_server_with_runtime_cache(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
    database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    runtime_cache: &crate::runtime::RuntimeCache,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>, Option<u32>)> {
    connect_stdio_server_core(
        server_name,
        server_config,
        ct,
        database_pool,
        Some(runtime_cache),
    )
    .await
}

/// Connect to a stdio server with timeout and cancellation token
pub async fn connect_stdio_server_with_ct(
    server_name: &str,
    server_config: &MCPServerConfig,
    ct: CancellationToken,
) -> Result<(RunningService<RoleClient, ()>, Vec<Tool>, Option<u32>)> {
    connect_stdio_server_core(server_name, server_config, ct, None, None).await
}
