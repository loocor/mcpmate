//! Runtime environment configuration
//!
//! This module provides functions for preparing command environment variables
//! based on runtime configurations stored in the database.
//!
//! Now uses the shared environment management system for consistency.

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::path::PathBuf;
use tokio::process::Command;

use crate::common::env::prepare_command_environment;
use crate::common::paths::global_paths;
use crate::runtime::{RuntimeType, config::get_config_by_type};

/// Prepare command environment variables based on runtime configurations in the database
///
/// This function:
/// 1. Determines the runtime type based on the command string
/// 2. Queries the database for the default configuration for that runtime type
/// 3. Uses the shared environment management system to set appropriate variables
/// 4. Falls back to environment variables if database query fails
pub async fn prepare_command_env_with_db(
    command: &mut Command,
    command_str: &str,
    pool: Option<&Pool<Sqlite>>,
) -> Result<()> {
    // Log the command we're preparing environment for
    tracing::info!(
        "Preparing environment for command: {} (executable: {})",
        command_str,
        command.as_std().get_program().to_string_lossy()
    );

    // 1. Determine runtime type
    let runtime_type = match command_str {
        "npx" => Some(RuntimeType::Node),
        "uvx" => Some(RuntimeType::Uv),
        "bunx" => Some(RuntimeType::Bun),
        _ => None,
    };

    // 2. If we have a runtime type and a database pool, try to get configuration from database
    if let (Some(rt_type), Some(pool)) = (runtime_type, pool) {
        if let Ok(config) = get_config_by_type(pool, rt_type).await {
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

            // Convert relative path to absolute path
            let bin_path = if config.relative_bin_path.starts_with('/') {
                // Absolute path (system runtime) - use as-is
                PathBuf::from(&config.relative_bin_path)
            } else if config.relative_bin_path.starts_with(".mcpmate/") {
                // Already properly formatted relative path
                home_dir.join(&config.relative_bin_path)
            } else {
                // Relative path that needs .mcpmate prefix
                home_dir.join(format!(".mcpmate/{}", config.relative_bin_path))
            };

            // Check if the binary file exists
            if bin_path.exists() {
                let (actual_bin_path, version) = if bin_path.is_file() {
                    // The bin_path points directly to the executable file
                    (bin_path, "latest")
                } else {
                    // bin_path is a directory, find the appropriate executable
                    let executable_path = find_runtime_executable(&bin_path, command_str)?;
                    (executable_path, "latest")
                };

                // Use shared environment management system
                let runtime_type_str = match rt_type {
                    RuntimeType::Node => "node",
                    RuntimeType::Uv => "uv",
                    RuntimeType::Bun => "bun",
                };

                if let Err(e) = prepare_command_environment(
                    command,
                    runtime_type_str,
                    &actual_bin_path,
                    version,
                ) {
                    tracing::warn!("Failed to prepare runtime environment: {}, falling back", e);
                    prepare_command_env_fallback(command, command_str);
                } else {
                    tracing::debug!(
                        "Successfully prepared {} environment using shared system",
                        runtime_type_str
                    );
                }
            } else {
                tracing::warn!(
                    "Binary path does not exist: {}, falling back",
                    bin_path.display()
                );
                prepare_command_env_fallback(command, command_str);
            }
        } else {
            tracing::debug!(
                "No database configuration found for {}, falling back",
                command_str
            );
            prepare_command_env_fallback(command, command_str);
        }
    } else {
        tracing::debug!(
            "No runtime type or database pool, falling back for {}",
            command_str
        );
        prepare_command_env_fallback(command, command_str);
    }

    Ok(())
}

/// Find the appropriate executable in a runtime directory
fn find_runtime_executable(
    bin_dir: &PathBuf,
    command_str: &str,
) -> Result<PathBuf> {
    let executable_name = match command_str {
        "npx" => {
            // Try npx first, then fall back to node
            let npx_path = bin_dir.join(if cfg!(windows) { "npx.cmd" } else { "npx" });
            if npx_path.exists() {
                return Ok(npx_path);
            }
            if cfg!(windows) { "node.exe" } else { "node" }
        }
        "uvx" => {
            // Try uvx first, then fall back to uv
            let uvx_path = bin_dir.join(if cfg!(windows) { "uvx.exe" } else { "uvx" });
            if uvx_path.exists() {
                return Ok(uvx_path);
            }
            if cfg!(windows) { "uv.exe" } else { "uv" }
        }
        "bunx" => {
            // Try bunx first, then fall back to bun
            let bunx_path = bin_dir.join(if cfg!(windows) { "bunx.exe" } else { "bunx" });
            if bunx_path.exists() {
                return Ok(bunx_path);
            }
            if cfg!(windows) { "bun.exe" } else { "bun" }
        }
        _ => return Err(anyhow::anyhow!("Unknown command: {}", command_str)),
    };

    let executable_path = bin_dir.join(executable_name);
    if executable_path.exists() {
        Ok(executable_path)
    } else {
        Err(anyhow::anyhow!(
            "Executable {} not found in {}",
            executable_name,
            bin_dir.display()
        ))
    }
}

/// Fallback environment preparation when database configuration is not available
///
/// This function provides basic environment setup without database configuration
fn prepare_command_env_fallback(
    command: &mut Command,
    command_str: &str,
) {
    tracing::debug!(
        "Using fallback environment preparation for: {}",
        command_str
    );

    let paths = global_paths();

    // Set basic cache directories based on command type
    match command_str {
        "npx" => {
            let cache_dir = paths.runtime_cache_dir("npm");
            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                tracing::warn!("Failed to create npm cache directory: {}", e);
            } else {
                command.env("NPM_CONFIG_CACHE", cache_dir.to_string_lossy().as_ref());
                tracing::debug!("Set NPM_CONFIG_CACHE to: {}", cache_dir.display());
            }
        }
        "uvx" => {
            let cache_dir = paths.runtime_cache_dir("uv");
            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                tracing::warn!("Failed to create uv cache directory: {}", e);
            } else {
                command.env("UV_CACHE_DIR", cache_dir.to_string_lossy().as_ref());
                tracing::debug!("Set UV_CACHE_DIR to: {}", cache_dir.display());

                // Additional uv-specific environment variables
                let python_cache_dir = paths.cache_dir().join("uv").join("python");
                let python_install_dir = paths.runtimes_dir().join("uv").join("python");

                for dir in [&python_cache_dir, &python_install_dir] {
                    if let Err(e) = std::fs::create_dir_all(dir) {
                        tracing::warn!("Failed to create directory {}: {}", dir.display(), e);
                    }
                }

                command.env(
                    "UV_PYTHON_CACHE_DIR",
                    python_cache_dir.to_string_lossy().as_ref(),
                );
                command.env(
                    "UV_PYTHON_INSTALL_DIR",
                    python_install_dir.to_string_lossy().as_ref(),
                );
            }
        }
        "bunx" => {
            let cache_dir = paths.runtime_cache_dir("bun");
            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                tracing::warn!("Failed to create bun cache directory: {}", e);
            } else {
                command.env(
                    "BUN_INSTALL_CACHE_DIR",
                    cache_dir.to_string_lossy().as_ref(),
                );
                tracing::debug!("Set BUN_INSTALL_CACHE_DIR to: {}", cache_dir.display());
            }
        }
        _ => {
            tracing::debug!("No specific environment setup for command: {}", command_str);
        }
    }
}
