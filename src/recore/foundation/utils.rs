//! Recore Utility Functions
//!
//! utility functions for timeout configuration and command preparation

use std::{future::Future, time::Duration};

use tokio::process::Command;
use tokio::time::timeout;

use super::error::{RecoreError, RecoreResult};
use crate::config::constants::commands;

/// determine appropriate connection timeout based on command type
pub fn get_connection_timeout(command: &str) -> Duration {
    match command {
        commands::DOCKER => Duration::from_secs(120), // Docker operations can take longer
        commands::NPX => Duration::from_secs(60),     // npm operations can take time
        commands::UVX | commands::BUNX => Duration::from_secs(30), // Runtime commands may need more time
        _ => Duration::from_secs(10),                              // Regular commands
    }
}

/// determine appropriate tools listing timeout based on command type
pub fn get_tools_timeout(command: &str) -> Duration {
    match command {
        commands::DOCKER => Duration::from_secs(60), // Docker operations can take longer
        commands::NPX => Duration::from_secs(60),    // npm timeout
        commands::UVX | commands::BUNX => Duration::from_secs(20), // Runtime commands may need more time
        _ => Duration::from_secs(10),                              // Regular commands
    }
}

/// determine appropriate SSE connection timeout
pub fn get_sse_connection_timeout() -> Duration {
    Duration::from_secs(60)
}

/// determine appropriate SSE service timeout
pub fn get_sse_service_timeout() -> Duration {
    Duration::from_secs(60)
}

/// determine appropriate SSE tools timeout
pub fn get_sse_tools_timeout() -> Duration {
    Duration::from_secs(60)
}

/// determine appropriate health check interval
pub fn get_health_check_interval() -> Duration {
    Duration::from_secs(30)
}

/// determine appropriate periodic reconnect interval
pub fn get_periodic_reconnect_interval() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

/// determine appropriate failed reconnect interval
pub fn get_failed_reconnect_interval() -> Duration {
    Duration::from_secs(60) // 1 minute
}

/// execute a future with timeout and return a Result with appropriate error
pub async fn with_timeout<T, F>(
    future: F,
    duration: Duration,
    server_name: &str,
    error_type: TimeoutErrorType,
) -> RecoreResult<T>
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
                TimeoutErrorType::Connection => Err(RecoreError::connection_error(
                    server_name,
                    &error_msg,
                    Some(e),
                )),
                TimeoutErrorType::Tools => {
                    Err(RecoreError::tools_error(server_name, &error_msg, Some(e)))
                }
                TimeoutErrorType::Service => Err(RecoreError::connection_error(
                    server_name,
                    &error_msg,
                    Some(e),
                )),
                TimeoutErrorType::Transport => Err(RecoreError::connection_error(
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
                    Err(RecoreError::connection_timeout(server_name, seconds))
                }
                TimeoutErrorType::Tools => Err(RecoreError::tools_timeout(server_name, seconds)),
            }
        }
    }
}

/// type of timeout error
pub enum TimeoutErrorType {
    /// connection timeout
    Connection,
    /// tools request timeout
    Tools,
    /// service creation timeout
    Service,
    /// transport creation timeout
    Transport,
}

/// prepare a command for cross-platform execution
/// on Windows, this may wrap the command with appropriate shell prefixes
/// on Unix-like systems, this returns the command as-is
pub fn prepare_command(
    command_str: &str,
    args: Option<&Vec<String>>,
) -> Command {
    let mut cmd = Command::new(command_str);

    // Add arguments if provided
    if let Some(args) = args {
        cmd.args(args);
    }

    // Set up environment for cross-platform compatibility
    #[cfg(target_os = "windows")]
    {
        // On Windows, ensure proper PATH and environment setup
        cmd.env("PYTHONIOENCODING", "utf-8");
        cmd.env("PYTHONUNBUFFERED", "1");
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Unix-like systems, ensure proper locale
        cmd.env("LC_ALL", "C.UTF-8");
        cmd.env("LANG", "C.UTF-8");
    }

    cmd
}

/// check if a command needs runtime setup (node, python, etc.)
pub fn needs_runtime_setup(command: &str) -> bool {
    matches!(
        command,
        commands::NPX | commands::UVX | commands::BUNX | commands::DOCKER
    )
}
