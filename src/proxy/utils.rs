use std::{env, future::Future, time::Duration};
use tokio::{process::Command, time::timeout};

use super::error::{ProxyError, Result};

/// Prepare command environment variables for different command types
pub fn prepare_command_env(command: &mut Command, command_str: &str) {
    // 1. bin path
    let bin_var = match command_str {
        "npx" => "NPX_BIN_PATH",
        "uvx" => "UVX_BIN_PATH",
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
}

/// Determine appropriate connection timeout based on command type
pub fn get_connection_timeout(command: &str) -> std::time::Duration {
    match command {
        "docker" => std::time::Duration::from_secs(120), // Docker operations can take longer
        "npx" => std::time::Duration::from_secs(60),     // npm operations can take time
        _ => std::time::Duration::from_secs(30),         // Default timeout
    }
}

/// Determine appropriate tools listing timeout based on command type
pub fn get_tools_timeout(command: &str) -> Duration {
    match command {
        "docker" => Duration::from_secs(60), // Docker operations can take longer
        "npx" => Duration::from_secs(30),    // npm operations can take time
        _ => Duration::from_secs(20),        // Default timeout
    }
}

/// Determine appropriate SSE connection timeout
pub fn get_sse_connection_timeout() -> Duration {
    Duration::from_secs(30)
}

/// Determine appropriate SSE service timeout
pub fn get_sse_service_timeout() -> Duration {
    Duration::from_secs(30)
}

/// Determine appropriate SSE tools timeout
pub fn get_sse_tools_timeout() -> Duration {
    Duration::from_secs(20)
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
                TimeoutErrorType::Connection => format!("Failed to connect to server: {}", e),
                TimeoutErrorType::Tools => format!("Failed to list tools: {}", e),
                TimeoutErrorType::Service => format!("Failed to create service: {}", e),
                TimeoutErrorType::Transport => format!("Failed to create transport: {}", e),
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
                    format!(
                        "Connection timeout for server '{}' after {}s",
                        server_name, seconds
                    )
                }
                TimeoutErrorType::Tools => {
                    format!(
                        "Tools request timeout for server '{}' after {}s",
                        server_name, seconds
                    )
                }
                TimeoutErrorType::Service => {
                    format!(
                        "Service creation timeout for server '{}' after {}s",
                        server_name, seconds
                    )
                }
                TimeoutErrorType::Transport => {
                    format!(
                        "Transport creation timeout for server '{}' after {}s",
                        server_name, seconds
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
