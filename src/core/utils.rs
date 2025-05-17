use std::{env, future::Future, time::Duration};

use tokio::{process::Command, time::timeout};

use super::error::{ProxyError, Result};

/// Prepare command environment variables for different command types
pub fn prepare_command_env(
    command: &mut Command,
    command_str: &str,
) {
    tracing::debug!("Preparing environment for command: {}", command_str);

    // 1. bin path
    let bin_var = match command_str {
        "npx" => "NPX_BIN_PATH",
        "uvx" => "UVX_BIN_PATH",
        _ => "MCP_RUNTIME_BIN",
    };

    tracing::debug!("Looking for binary path in env var: {}", bin_var);

    let bin_path = env::var(bin_var)
        .or_else(|_| {
            tracing::debug!("Env var {} not found, trying MCP_RUNTIME_BIN", bin_var);
            env::var("MCP_RUNTIME_BIN")
        })
        .ok();

    if let Some(bin_path) = bin_path {
        let old_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{bin_path}:{old_path}");
        tracing::debug!("Setting PATH to include binary path: {}", bin_path);
        command.env("PATH", new_path);
    } else {
        tracing::warn!(
            "No binary path found for {} (tried {} and MCP_RUNTIME_BIN)",
            command_str,
            bin_var
        );
    }

    // 2. cache env
    let cache_var = match command_str {
        "npx" => "NPM_CONFIG_CACHE",
        "uvx" => "UV_CACHE_DIR",
        _ => "",
    };

    if !cache_var.is_empty() {
        tracing::debug!("Looking for cache directory in env var: {}", cache_var);

        if let Ok(cache_val) = env::var(cache_var) {
            tracing::debug!("Setting cache env var {}={}", cache_var, cache_val);
            command.env(cache_var, cache_val);
        } else {
            tracing::warn!("Cache env var {} not found", cache_var);
        }
    }

    // Log relevant environment variables for debugging
    if tracing::enabled!(tracing::Level::DEBUG) {
        let relevant_vars = [
            "NPX_BIN_PATH",
            "NPM_CONFIG_CACHE",
            "UVX_BIN_PATH",
            "UV_CACHE_DIR",
            "PATH",
        ];

        let mut env_info = Vec::new();
        for var in relevant_vars {
            if let Ok(value) = env::var(var) {
                env_info.push(format!("{var}={value}"));
            } else {
                env_info.push(format!("{var}=<NOT SET>"));
            }
        }

        tracing::debug!("Environment for {}: {}", command_str, env_info.join(", "));
    }
}

/// Determine appropriate connection timeout based on command type
pub fn get_connection_timeout(command: &str) -> std::time::Duration {
    match command {
        "docker" => std::time::Duration::from_secs(120), // Docker operations can take longer
        "npx" => std::time::Duration::from_secs(60),     // npm operations can take time
        _ => std::time::Duration::from_secs(60),         // Increased default timeout to 60 seconds
    }
}

/// Determine appropriate tools listing timeout based on command type
pub fn get_tools_timeout(command: &str) -> Duration {
    match command {
        "docker" => Duration::from_secs(60), // Docker operations can take longer
        "npx" => Duration::from_secs(60),    // Increased npm timeout to 60 seconds
        _ => Duration::from_secs(60),        // Increased default timeout to 60 seconds
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
                TimeoutErrorType::Tools =>
                    Err(ProxyError::tools_error(server_name, &error_msg, Some(e))),
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
                        "Connection timeout for server '{server_name}' after {seconds}s"
                    )
                }
                TimeoutErrorType::Tools => {
                    format!(
                        "Tools request timeout for server '{server_name}' after {seconds}s"
                    )
                }
                TimeoutErrorType::Service => {
                    format!(
                        "Service creation timeout for server '{server_name}' after {seconds}s"
                    )
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
                | TimeoutErrorType::Transport =>
                    Err(ProxyError::connection_timeout(server_name, seconds)),
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
