//! Runtime environment configuration
//!
//! This module provides functions for preparing command environment variables
//! based on runtime configurations stored in the database.
//!
//! Now uses the shared environment management system for consistency.

use anyhow::Result;
use sqlx::{Pool, Sqlite};

use tokio::process::Command;

use crate::common::env::{apply_default_runtime_cache_environment, prepare_command_environment};
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

        let runtime_type = RuntimeType::from_command(command_str).expect("get_command_path matched a runtime type");
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

    apply_default_runtime_cache_environment(command)?;

    Ok(())
}
