//! Runtime environment configuration
//!
//! This module provides functions for preparing command environment variables
//! based on runtime configurations stored in the database.

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::{env, path::PathBuf};
use tokio::process::Command;

use crate::runtime::{RuntimeType, config::get_config_by_type};

/// Prepare command environment variables based on runtime configurations in the database
///
/// This function:
/// 1. Determines the runtime type based on the command string
/// 2. Queries the database for the default configuration for that runtime type
/// 3. Sets the appropriate environment variables (PATH, cache directories, etc.)
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

            // Set PATH to include the runtime binary directory
            // Handle both absolute paths (system runtimes) and relative paths (managed runtimes)
            let bin_path = if config.relative_bin_path.starts_with('/') {
                // Absolute path (system runtime) - use as-is
                std::path::PathBuf::from(&config.relative_bin_path)
            } else if config.relative_bin_path.starts_with(".mcpmate/") {
                // Already properly formatted relative path
                home_dir.join(&config.relative_bin_path)
            } else {
                // Relative path that needs .mcpmate prefix
                home_dir.join(format!(".mcpmate/{}", config.relative_bin_path))
            };

            // Check if the binary file exists
            if bin_path.exists() {
                if bin_path.is_file() {
                    // The bin_path points directly to the executable file
                    // We need to add the parent directory to PATH
                    if let Some(bin_dir) = bin_path.parent() {
                        let old_path = env::var("PATH").unwrap_or_default();
                        let new_path = if cfg!(windows) {
                            format!("{};{}", bin_dir.display(), old_path)
                        } else {
                            format!("{}:{}", bin_dir.display(), old_path)
                        };
                        command.env("PATH", new_path);
                        tracing::debug!(
                            "Set PATH to include: {} (for executable: {})",
                            bin_dir.display(),
                            bin_path.display()
                        );
                    } else {
                        tracing::warn!(
                            "Could not get parent directory for executable: {}",
                            bin_path.display()
                        );
                    }
                } else {
                    // bin_path is a directory, use the old logic
                    let executable_exists = match command_str {
                        "npx" => {
                            let npx_path = bin_path.join("npx");
                            let node_path = bin_path.join("node");
                            let exists = npx_path.exists() || node_path.exists();
                            if !exists {
                                tracing::warn!(
                                    "Neither npx nor node executable found in: {}",
                                    bin_path.display()
                                );
                            }
                            exists
                        }
                        "uvx" => {
                            let uv_path = bin_path.join("uv");
                            let exists = uv_path.exists();
                            if !exists {
                                tracing::warn!(
                                    "uv executable not found in: {}",
                                    bin_path.display()
                                );
                            }
                            exists
                        }
                        "bunx" => {
                            let bun_path = bin_path.join("bun");
                            let exists = bun_path.exists();
                            if !exists {
                                tracing::warn!(
                                    "bun executable not found in: {}",
                                    bin_path.display()
                                );
                            }
                            exists
                        }
                        _ => true,
                    };

                    if executable_exists {
                        let old_path = env::var("PATH").unwrap_or_default();
                        let new_path = if cfg!(windows) {
                            format!("{};{}", bin_path.display(), old_path)
                        } else {
                            format!("{}:{}", bin_path.display(), old_path)
                        };
                        command.env("PATH", new_path);
                        tracing::debug!("Set PATH to include: {}", bin_path.display());
                    }
                }
            } else {
                tracing::warn!("Binary path does not exist: {}", bin_path.display());
            }

            // Set cache path based on runtime type
            let cache_dir = match rt_type {
                RuntimeType::Node => home_dir.join(".mcpmate/cache/npm"),
                RuntimeType::Uv => home_dir.join(".mcpmate/cache/uv"),
                RuntimeType::Bun => home_dir.join(".mcpmate/cache/bun"),
            };

            // Create cache directory if it doesn't exist
            if !cache_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                    tracing::warn!(
                        "Failed to create cache directory {}: {}",
                        cache_dir.display(),
                        e
                    );
                }
            }

            match rt_type {
                RuntimeType::Node => {
                    command.env("NPM_CONFIG_CACHE", cache_dir.display().to_string());
                    tracing::debug!("Set NPM_CONFIG_CACHE to: {}", cache_dir.display());
                }
                RuntimeType::Uv => {
                    command.env("UV_CACHE_DIR", cache_dir.display().to_string());
                    tracing::debug!("Set UV_CACHE_DIR to: {}", cache_dir.display());

                    // Additional uv-specific environment variables
                    let mcpmate_dir = home_dir.join(".mcpmate");
                    let uv_python_cache_dir = mcpmate_dir.join("cache").join("python");
                    let uv_python_install_dir = mcpmate_dir.join("runtimes").join("python");

                    // Create directories
                    for dir in [&uv_python_cache_dir, &uv_python_install_dir] {
                        if !dir.exists() {
                            if let Err(e) = std::fs::create_dir_all(dir) {
                                tracing::warn!(
                                    "Failed to create directory {}: {}",
                                    dir.display(),
                                    e
                                );
                            }
                        }
                    }

                    command.env(
                        "UV_PYTHON_CACHE_DIR",
                        uv_python_cache_dir.display().to_string(),
                    );
                    command.env(
                        "UV_PYTHON_INSTALL_DIR",
                        uv_python_install_dir.display().to_string(),
                    );

                    tracing::debug!(
                        "Set uv environment variables: UV_CACHE_DIR={}, UV_PYTHON_CACHE_DIR={}, UV_PYTHON_INSTALL_DIR={}",
                        cache_dir.display(),
                        uv_python_cache_dir.display(),
                        uv_python_install_dir.display()
                    );
                }
                RuntimeType::Bun => {
                    command.env("BUN_INSTALL_CACHE_DIR", cache_dir.display().to_string());
                    tracing::debug!("Set BUN_INSTALL_CACHE_DIR to: {}", cache_dir.display());
                }
            }

            return Ok(());
        } else {
            tracing::debug!(
                "Failed to get runtime config from database, falling back to environment variables"
            );
        }
    }

    // 3. Fall back to environment variables
    prepare_command_env_fallback(command, command_str);
    Ok(())
}

/// Fall back to environment variables for command environment preparation
fn prepare_command_env_fallback(
    command: &mut Command,
    command_str: &str,
) {
    // Set binary path
    let bin_var = match command_str {
        "npx" => "NPX_BIN_PATH",
        "uvx" => "UVX_BIN_PATH",
        "bunx" => "BUNX_BIN_PATH",
        _ => "MCP_RUNTIME_BIN",
    };

    if let Ok(bin_path) = env::var(bin_var) {
        let old_path = env::var("PATH").unwrap_or_default();
        let new_path = if cfg!(windows) {
            format!("{};{}", bin_path, old_path)
        } else {
            format!("{}:{}", bin_path, old_path)
        };
        command.env("PATH", new_path);
        tracing::debug!("Fallback: Set PATH to include: {}", bin_path);
    }

    // Set cache directory
    let cache_var = match command_str {
        "npx" => "NPM_CONFIG_CACHE",
        "uvx" => "UV_CACHE_DIR",
        "bunx" => "BUN_INSTALL_CACHE_DIR",
        _ => "",
    };

    if !cache_var.is_empty() {
        if let Ok(cache_path) = env::var(cache_var) {
            command.env(cache_var, &cache_path);
            tracing::debug!("Fallback: Set {} to: {}", cache_var, cache_path);
        }
    }
}
