use std::{future::Future, time::Duration};

use tokio::process::Command;
use tokio::time::timeout;

use super::error::{ProxyError, Result};
use crate::config::constants::commands;

/// Determine appropriate connection timeout based on command type
pub fn get_connection_timeout(command: &str) -> std::time::Duration {
    match command {
        commands::DOCKER => std::time::Duration::from_secs(120), // Docker operations can take longer
        commands::NPX => std::time::Duration::from_secs(60),     // npm operations can take time
        commands::UVX | commands::BUNX => std::time::Duration::from_secs(30), // Runtime commands may need more time
        _ => std::time::Duration::from_secs(10),                              // Regular commands
    }
}

/// Determine appropriate tools listing timeout based on command type
pub fn get_tools_timeout(command: &str) -> Duration {
    match command {
        commands::DOCKER => Duration::from_secs(60), // Docker operations can take longer
        commands::NPX => Duration::from_secs(60),    // npm timeout
        commands::UVX | commands::BUNX => Duration::from_secs(20), // Runtime commands may need more time
        _ => Duration::from_secs(10),                              // Regular commands
    }
}

/// Determine appropriate SSE connection timeout
pub fn get_sse_connection_timeout() -> Duration {
    Duration::from_secs(60)
}

/// Determine appropriate SSE service timeout
pub fn get_sse_service_timeout() -> Duration {
    Duration::from_secs(60)
}

/// Determine appropriate SSE tools timeout
pub fn get_sse_tools_timeout() -> Duration {
    Duration::from_secs(60)
}

/// Determine appropriate health check interval
pub fn get_health_check_interval() -> Duration {
    Duration::from_secs(30)
}

/// Determine appropriate periodic reconnect interval
pub fn get_periodic_reconnect_interval() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

/// Determine appropriate failed reconnect interval
pub fn get_failed_reconnect_interval() -> Duration {
    Duration::from_secs(60) // 1 minute
}

/// Execute a future with timeout and return a Result with appropriate error
pub async fn with_timeout<T, F>(
    future: F,
    duration: Duration,
    server_name: &str,
    error_type: TimeoutErrorType,
) -> Result<T>
where
    F: Future<Output = std::result::Result<T, anyhow::Error>>,
{
    match timeout(duration, future).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => {
            let error_msg = match error_type {
                TimeoutErrorType::Connection => format!("Failed to connect to server: {e}"),
                TimeoutErrorType::Tools => format!("Failed to list tools: {e}"),
                TimeoutErrorType::Service => format!("Failed to create service: {e}"),
                TimeoutErrorType::Transport => format!("Failed to create transport: {e}"),
            };

            match error_type {
                TimeoutErrorType::Connection => Err(ProxyError::connection_error(
                    server_name,
                    &error_msg,
                    Some(e),
                )),
                TimeoutErrorType::Tools => {
                    Err(ProxyError::tools_error(server_name, &error_msg, Some(e)))
                }
                TimeoutErrorType::Service => Err(ProxyError::connection_error(
                    server_name,
                    &error_msg,
                    Some(e),
                )),
                TimeoutErrorType::Transport => Err(ProxyError::connection_error(
                    server_name,
                    &error_msg,
                    Some(e),
                )),
            }
        }
        Err(_) => {
            let seconds = duration.as_secs();
            let error_msg = match error_type {
                TimeoutErrorType::Connection => {
                    format!("Connection timeout for server '{server_name}' after {seconds}s")
                }
                TimeoutErrorType::Tools => {
                    format!("Tools request timeout for server '{server_name}' after {seconds}s")
                }
                TimeoutErrorType::Service => {
                    format!("Service creation timeout for server '{server_name}' after {seconds}s")
                }
                TimeoutErrorType::Transport => {
                    format!(
                        "Transport creation timeout for server '{server_name}' after {seconds}s"
                    )
                }
            };

            tracing::warn!("{}", error_msg);

            match error_type {
                TimeoutErrorType::Connection
                | TimeoutErrorType::Service
                | TimeoutErrorType::Transport => {
                    Err(ProxyError::connection_timeout(server_name, seconds))
                }
                TimeoutErrorType::Tools => Err(ProxyError::tools_timeout(server_name, seconds)),
            }
        }
    }
}

/// Type of timeout error
pub enum TimeoutErrorType {
    /// Connection timeout
    Connection,
    /// Tools request timeout
    Tools,
    /// Service creation timeout
    Service,
    /// Transport creation timeout
    Transport,
}

/// Prepare a command for cross-platform execution
/// On Windows, this may wrap the command with appropriate shell prefixes
/// On Unix-like systems, this returns the command as-is
pub fn prepare_command(
    command_str: &str,
    args: Option<&Vec<String>>,
) -> Command {
    #[cfg(windows)]
    {
        // Check if this is a runtime command that might need special handling
        let needs_shell_wrapper = matches!(command_str, "npx" | "uvx" | "bunx")
            || command_str.ends_with(".js")
            || command_str.ends_with(".py")
            || command_str.ends_with(".ts");

        if needs_shell_wrapper {
            // Use PowerShell for better compatibility with modern Windows
            let mut cmd = Command::new("powershell");
            cmd.arg("-NoProfile").arg("-NonInteractive").arg("-Command");

            // Build the command string
            let mut full_command = command_str.to_string();
            if let Some(args) = args {
                for arg in args {
                    // Escape arguments that contain spaces
                    if arg.contains(' ') {
                        full_command.push_str(&format!(" \"{}\"", arg));
                    } else {
                        full_command.push_str(&format!(" {}", arg));
                    }
                }
            }

            cmd.arg(full_command);
            tracing::debug!("Windows: Wrapping command with PowerShell: {}", command_str);
            cmd
        } else {
            // For regular executables, use them directly
            let mut cmd = Command::new(command_str);
            if let Some(args) = args {
                cmd.args(args);
            }
            cmd
        }
    }

    #[cfg(not(windows))]
    {
        let mut cmd = Command::new(command_str);
        if let Some(args) = args {
            cmd.args(args);
        }
        cmd
    }
}

/// Check if a command needs runtime environment setup
pub fn needs_runtime_setup(command: &str) -> bool {
    matches!(command, "npx" | "uvx" | "bunx")
}
