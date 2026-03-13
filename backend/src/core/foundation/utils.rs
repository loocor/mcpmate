//! Core Utility Functions
//!
//! utility functions for timeout configuration, command preparation, and common patterns

use std::{collections::HashSet, future::Future, hash::Hash, time::Duration};

use tokio::process::Command;
use tokio::time::timeout;

use super::error::{CoreError, CoreResult};
use crate::common::constants::commands;

/// determine appropriate connection timeout based on command type
pub fn get_connection_timeout(command: &str) -> Duration {
    match command {
        commands::DOCKER => Duration::from_secs(120), // Docker operations can take longer
        commands::NPX => Duration::from_secs(60),     // npm operations can take time (handled by transformation)
        commands::UVX | commands::BUNX => Duration::from_secs(30), // Runtime commands may need more time
        _ => Duration::from_secs(10),                 // Regular commands
    }
}

/// determine appropriate tools listing timeout based on command type
pub fn get_tools_timeout(command: &str) -> Duration {
    match command {
        commands::DOCKER => Duration::from_secs(60), // Docker operations can take longer
        commands::NPX => Duration::from_secs(60),    // npm timeout (handled by transformation)
        commands::UVX | commands::BUNX => Duration::from_secs(20), // Runtime commands may need more time
        _ => Duration::from_secs(10),                // Regular commands
    }
}

/// determine appropriate SSE connection timeout
pub fn get_sse_connection_timeout() -> Duration {
    if let Ok(v) = std::env::var("MCPMATE_SSE_CONNECT_TIMEOUT_MS") {
        if let Ok(ms) = v.parse::<u64>() {
            return Duration::from_millis(ms);
        }
    }
    Duration::from_secs(60)
}

/// determine appropriate SSE service timeout
pub fn get_sse_service_timeout() -> Duration {
    if let Ok(v) = std::env::var("MCPMATE_SSE_SERVICE_TIMEOUT_MS") {
        if let Ok(ms) = v.parse::<u64>() {
            return Duration::from_millis(ms);
        }
    }
    Duration::from_secs(60)
}

/// determine appropriate SSE tools timeout
pub fn get_sse_tools_timeout() -> Duration {
    if let Ok(v) = std::env::var("MCPMATE_SSE_TOOLS_TIMEOUT_MS") {
        if let Ok(ms) = v.parse::<u64>() {
            return Duration::from_millis(ms);
        }
    }
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
                TimeoutErrorType::Connection => Err(CoreError::connection_error(server_name, &error_msg, Some(e))),
                TimeoutErrorType::Tools => Err(CoreError::tools_error(server_name, &error_msg, Some(e))),
                TimeoutErrorType::Service => Err(CoreError::connection_error(server_name, &error_msg, Some(e))),
                TimeoutErrorType::Transport => Err(CoreError::connection_error(server_name, &error_msg, Some(e))),
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
                    format!("Transport creation timeout for server '{server_name}' after {seconds}s")
                }
            };

            tracing::warn!("{}", error_msg);

            match error_type {
                TimeoutErrorType::Connection | TimeoutErrorType::Service | TimeoutErrorType::Transport => {
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
    if command.eq_ignore_ascii_case("npx") {
        tracing::debug!("Respecting user-provided 'npx' command without bunx rewrite");
    }
    command.to_string()
}

/// Generic deduplication helper that removes duplicates based on a key function
///
/// This helper reduces code duplication across the codebase for common
/// deduplication patterns using HashSet.
///
/// # Arguments
/// * `items` - Vector of items to deduplicate
/// * `key_fn` - Function that extracts the key for deduplication
///
/// # Returns
/// * `Vec<T>` - Deduplicated vector maintaining original order for first occurrence
pub fn deduplicate_by_key<T, K, F>(
    items: Vec<T>,
    key_fn: F,
) -> Vec<T>
where
    K: Hash + Eq,
    F: Fn(&T) -> K,
{
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for item in items {
        let key = key_fn(&item);
        if seen.insert(key) {
            result.push(item);
        }
    }

    result
}

/// Database fallback helper for operations that require database access
///
/// This helper implements the early return pattern for database operations,
/// providing a consistent way to handle missing database connections.
///
/// # Arguments
/// * `db_option` - Optional reference to database
/// * `operation` - Async operation to perform with database
/// * `fallback_value` - Value to return if database is not available
///
/// # Returns
/// * `Result<T>` - Operation result or fallback value
pub async fn with_db_or_fallback<T, F, Fut>(
    db_option: Option<&crate::config::database::Database>,
    operation: F,
    fallback_value: T,
) -> anyhow::Result<T>
where
    F: FnOnce(&crate::config::database::Database) -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    match db_option {
        Some(db) => operation(db).await,
        None => {
            tracing::warn!("Database not available, using fallback value");
            Ok(fallback_value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_command() {
        // Ensure user-provided commands are preserved
        assert_eq!(transform_command("npx"), "npx");

        // Test other commands remain unchanged
        assert_eq!(transform_command("uvx"), "uvx");
        assert_eq!(transform_command("bunx"), "bunx");
        assert_eq!(transform_command("docker"), "docker");
        assert_eq!(transform_command("node"), "node");
        assert_eq!(transform_command("python"), "python");
    }

    #[test]
    fn test_deduplicate_by_key() {
        // Test with integers using identity function
        let items = vec![1, 2, 2, 3, 1, 4];
        let result = deduplicate_by_key(items, |x| *x);
        assert_eq!(result, vec![1, 2, 3, 4]);

        // Test with tuples using first element as key
        let items = vec![("a", 1), ("b", 2), ("a", 3), ("c", 4)];
        let result = deduplicate_by_key(items, |x| x.0);
        assert_eq!(result, vec![("a", 1), ("b", 2), ("c", 4)]);

        // Test empty vector
        let items: Vec<i32> = vec![];
        let result = deduplicate_by_key(items, |x| *x);
        assert_eq!(result, Vec::<i32>::new());
    }
}
