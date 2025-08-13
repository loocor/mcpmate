//! Runtime environment configuration
//!
//! This module provides functions for preparing command environment variables
//! based on runtime configurations stored in the database.
//!
//! Now uses the shared environment management system for consistency.

use anyhow::Result;
use sqlx::{Pool, Sqlite};

use tokio::process::Command;

use crate::common::env::prepare_command_environment;
use crate::common::paths::global_paths;
use crate::common::constants::commands;
use crate::runtime::RuntimeType;

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
    _pool: Option<&Pool<Sqlite>>, // Database parameter kept for API compatibility but ignored
) -> Result<()> {
    use crate::runtime::RuntimeManager;

    // Log the command we're preparing environment for
    tracing::info!(
        "Preparing environment for command: {} (executable: {})",
        command_str,
        command.as_std().get_program().to_string_lossy()
    );

    // 1. Determine runtime type (npx is handled by command transformation, not here)
    let runtime_type = match command_str {
        commands::UVX => Some(RuntimeType::Uv),
        commands::BUNX => Some(RuntimeType::Bun),
        _ => None,
    };

    // 2. Use RuntimeManager to find runtime paths
    if let Some(rt_type) = runtime_type {
        let manager = RuntimeManager::new();

        if let Some(runtime_path) = manager.get_executable_path(rt_type) {
            tracing::info!(
                "Using MCPMate managed runtime for '{}': {}",
                command_str,
                runtime_path.display()
            );

            // Use shared environment management system
            let runtime_type_str = match rt_type {
                RuntimeType::Uv => "uv",
                RuntimeType::Bun => "bun",
            };

            if let Err(e) =
                prepare_command_environment(command, runtime_type_str, &runtime_path, "latest")
            {
                tracing::warn!("Failed to prepare runtime environment: {}, falling back", e);
                prepare_command_env_fallback(command, command_str);
            } else {
                tracing::debug!(
                    "Successfully prepared {} environment using simplified system",
                    runtime_type_str
                );
            }
        } else {
            tracing::debug!("No runtime found for {}, falling back", command_str);
            prepare_command_env_fallback(command, command_str);
        }
    } else {
        tracing::debug!("No runtime type mapping for {}, falling back", command_str);
        prepare_command_env_fallback(command, command_str);
    }

    Ok(())
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

    // Set basic cache directories based on command type (npx handled by transformation)
    match command_str {
        commands::UVX => {
            // Since we now use system uvx, we only need basic cache setup
            let cache_dir = paths.runtime_cache_dir("uv");
            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                tracing::warn!("Failed to create uv cache directory: {}", e);
            } else {
                command.env("UV_CACHE_DIR", cache_dir.to_string_lossy().as_ref());
                tracing::debug!("Set UV_CACHE_DIR to: {}", cache_dir.display());
            }
        }
        commands::BUNX => {
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
