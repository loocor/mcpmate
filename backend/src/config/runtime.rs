//! Runtime environment configuration
//!
//! This module provides functions for preparing command environment variables
//! based on runtime configurations stored in the database.
//!
//! Now uses the shared environment management system for consistency.

use anyhow::Result;
use sqlx::{Pool, Sqlite};

use tokio::process::Command;

use crate::common::constants::commands;
use crate::common::env::prepare_command_environment;
use crate::common::paths::global_paths;
use crate::runtime::RuntimeType;

/// Prepare command environment variables based on runtime configurations in the database
///
/// This function:
/// 1. Determines the runtime type based on the command string
/// 2. Queries the database for the default configuration for that runtime type
/// 3. Uses the shared environment management system to set appropriate variables
/// 4. Prepares cache/runtime environment using the canonical MCPMate data directory
pub async fn prepare_command_env_with_db(
    command: &mut Command,
    command_str: &str,
    _pool: Option<&Pool<Sqlite>>, // Database parameter kept for API compatibility but ignored
) -> Result<()> {
    use crate::runtime::RuntimeManager;

    // Log the command we're preparing environment for
    tracing::debug!(
        "Preparing environment for command: {} (executable: {})",
        command_str,
        command.as_std().get_program().to_string_lossy()
    );

    // Determine runtime type and check for managed installation.
    // `get_command_path` internally calls `RuntimeType::from_command`,
    // so no separate type check is needed here.
    let manager = RuntimeManager::new();

    if let Some(runtime_path) = manager.get_command_path(command_str) {
        tracing::debug!(
            "Using MCPMate managed runtime for '{}': {}",
            command_str,
            runtime_path.display()
        );

        let runtime_type = RuntimeType::from_command(command_str)
            .expect("get_command_path matched a runtime type");
        let runtime_type_str = runtime_type.as_str();

        prepare_command_environment(command, runtime_type_str, &runtime_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to prepare {} runtime environment for '{}': {}",
                runtime_type_str,
                command_str,
                e
            )
        })?;

        tracing::debug!(
            "Successfully prepared {} environment using simplified system",
            runtime_type_str
        );
    } else {
        tracing::debug!(
            "No MCPMate-managed runtime found for {}, preparing basic environment",
            command_str
        );
        prepare_basic_command_env(command, command_str)?;
    }

    Ok(())
}

/// Prepare basic environment variables without selecting a managed runtime binary.
fn prepare_basic_command_env(
    command: &mut Command,
    command_str: &str,
) -> Result<()> {
    tracing::debug!("Preparing basic environment for: {}", command_str);

    let paths = global_paths();

    // Set basic cache directories based on command type.
    match command_str {
        commands::UV | commands::UVX => {
            let cache_dir = paths.runtime_cache_dir("uv");
            std::fs::create_dir_all(&cache_dir)?;
            command.env("UV_CACHE_DIR", cache_dir.to_string_lossy().as_ref());
            tracing::debug!("Set UV_CACHE_DIR to: {}", cache_dir.display());
        }
        commands::BUN | commands::BUNX => {
            let cache_dir = paths.runtime_cache_dir("bun");
            std::fs::create_dir_all(&cache_dir)?;
            command.env("BUN_INSTALL_CACHE_DIR", cache_dir.to_string_lossy().as_ref());
            tracing::debug!("Set BUN_INSTALL_CACHE_DIR to: {}", cache_dir.display());
        }
        commands::NODE | commands::NPM | commands::NPX => {
            let cache_dir = paths.runtime_cache_dir("node");
            std::fs::create_dir_all(&cache_dir)?;
            command.env("NPM_CONFIG_CACHE", cache_dir.to_string_lossy().as_ref());
            tracing::debug!("Set NPM_CONFIG_CACHE to: {}", cache_dir.display());
        }
        _ => {
            tracing::debug!("No specific environment setup for command: {}", command_str);
        }
    }

    Ok(())
}
