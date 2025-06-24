//! Core Utility Functions
//!
//! utility functions for timeout configuration and command preparation

use std::{future::Future, time::Duration};

use tokio::process::Command;
use tokio::time::timeout;

use super::error::{CoreError, CoreResult};
use crate::config::constants::commands;

/// determine appropriate connection timeout based on command type
pub fn get_connection_timeout(command: &str) -> Duration {
    match command {
        commands::DOCKER => Duration::from_secs(120), // Docker operations can take longer
        commands::NPX => Duration::from_secs(60), // npm operations can take time (handled by transformation)
        commands::UVX | commands::BUNX => Duration::from_secs(30), // Runtime commands may need more time
        _ => Duration::from_secs(10),                              // Regular commands
    }
}

/// determine appropriate tools listing timeout based on command type
pub fn get_tools_timeout(command: &str) -> Duration {
    match command {
        commands::DOCKER => Duration::from_secs(60), // Docker operations can take longer
        commands::NPX => Duration::from_secs(60),    // npm timeout (handled by transformation)
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
) -> CoreResult<T>
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
                TimeoutErrorType::Connection => Err(CoreError::connection_error(
                    server_name,
                    &error_msg,
                    Some(e),
                )),
                TimeoutErrorType::Tools => {
                    Err(CoreError::tools_error(server_name, &error_msg, Some(e)))
                }
                TimeoutErrorType::Service => Err(CoreError::connection_error(
                    server_name,
                    &error_msg,
                    Some(e),
                )),
                TimeoutErrorType::Transport => Err(CoreError::connection_error(
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
                    Err(CoreError::connection_timeout(server_name, seconds))
                }
                TimeoutErrorType::Tools => Err(CoreError::tools_timeout(server_name, seconds)),
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

/// check if a command needs runtime setup (python, etc.)
/// Note: npx is handled by command transformation to bunx
pub fn needs_runtime_setup(command: &str) -> bool {
    matches!(command, commands::UVX | commands::BUNX | commands::DOCKER)
}

/// Transform npx commands to bunx for better performance and compatibility
pub fn transform_command(command: &str) -> String {
    match command {
        "npx" => {
            tracing::info!("Transforming npx command to bunx for better performance");
            "bunx".to_string()
        }
        _ => command.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_command() {
        // Test npx transformation
        assert_eq!(transform_command("npx"), "bunx");

        // Test other commands remain unchanged
        assert_eq!(transform_command("uvx"), "uvx");
        assert_eq!(transform_command("bunx"), "bunx");
        assert_eq!(transform_command("docker"), "docker");
        assert_eq!(transform_command("node"), "node");
        assert_eq!(transform_command("python"), "python");
    }
}
