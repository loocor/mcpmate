//! MCPMate Runtime Module
//!
//! This module provides MCPMate's runtime environment management functionality, including:
//! - Environment detection: detect current system environment and architecture
//! - Runtime download: download and manage various runtime environments
//! - Path management: manage runtime environment installation paths
//! - Version control: manage different versions of runtime environments

pub mod cache; // Runtime state caching
pub mod cli; // Command-line interface handlers
pub mod config; // Runtime configuration management
pub mod constants; // Path constants and utilities
mod detection; // Environment detection related functionality
mod download; // Download management related functionality
mod installers; // Installer related functionality
pub mod integration; // Runtime integration utilities (database, events)
mod list; // List installed runtime environments
pub mod migration; // Runtime configuration migration utilities
mod paths; // Path management related functionality
mod types; // Type definitions

// Re-export main types and functions
pub use cache::{RuntimeCache, RuntimeCacheStats, RuntimeState};
pub use constants::*; // Export all path constants
pub use detection::{Architecture, Environment, OperatingSystem, detect_environment};
pub use download::{
    InlineProgressBar, InteractiveHandler, MultiLineProgress, NetworkDiagnostics,
    NetworkDiagnosticsRunner, RuntimeDownloader, TimeoutAction, download_runtime,
    download_runtime_with_config, get_diagnostic_suggestions, get_user_confirmation,
    quick_connectivity_test, supports_inline_progress, supports_interactive,
};
pub use installers::{bun::BunInstaller, node::NodeInstaller, uv::UvInstaller};
pub use list::list_runtime;
pub use paths::{RuntimePaths, get_runtime_path, show_runtime_path};
pub use types::{
    Commands, DownloadConfig, DownloadProgress, DownloadStage, ExecutionContext, RuntimeError,
    RuntimeType,
};

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Main structure of runtime manager
#[derive(Debug)]
pub struct RuntimeManager {
    environment: Environment,
}

impl RuntimeManager {
    /// Create a new runtime manager instance
    pub fn new() -> Result<Self> {
        let environment = detect_environment()?;

        Ok(Self { environment })
    }

    /// Verify runtime availability, and ensure it's installed if not available
    /// This is a simplified version that directly uses the existing ensure method
    pub async fn verify_and_ensure_runtime(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
        database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    ) -> Result<PathBuf> {
        // Use the existing ensure method, which already includes check and install logic
        let path = self.ensure(runtime_type, version).await?;

        // If a database pool is provided, always save the configuration
        // This ensures that even existing runtimes are recorded in the database
        tracing::debug!("Database pool provided: {}", database_pool.is_some());
        if let Some(pool) = database_pool {
            tracing::debug!("Saving runtime config for {:?} to database", runtime_type);
            // Save runtime config to database
            if let Err(e) = self
                .save_runtime_config_to_db(pool, runtime_type, &path)
                .await
            {
                tracing::warn!("Failed to save runtime config to database: {}", e);
                // Continue execution, this is not a critical error
            } else {
                tracing::info!(
                    "Successfully saved runtime config for {:?} to database",
                    runtime_type
                );
            }

            // Send runtime ready event to notify stdio to reconnect
            let version_str = version.unwrap_or_else(|| runtime_type.default_version());
            integration::send_runtime_ready_event(runtime_type, version_str, &path);
        }

        Ok(path)
    }

    /// Ensure runtime is available for the given command
    /// Maps commands (npx, uvx, bunx) to their required runtimes and ensures they're available
    pub async fn ensure_runtime_for_command(
        &self,
        command: &str,
        database_pool: Option<&sqlx::Pool<sqlx::Sqlite>>,
    ) -> Result<Option<PathBuf>> {
        use crate::conf::constants::commands;

        // Determine if this command needs a runtime
        let runtime_type = match command {
            commands::NPX => RuntimeType::Node,
            commands::UVX => RuntimeType::Uv,
            commands::BUNX => RuntimeType::Bun,
            _ => {
                tracing::debug!("Command '{}' does not require runtime management", command);
                return Ok(None);
            }
        };

        tracing::debug!("Ensuring runtime availability for command: {}", command);

        // Use the simplified verify_and_ensure_runtime method
        match self
            .verify_and_ensure_runtime(runtime_type, None, database_pool)
            .await
        {
            Ok(install_path) => {
                tracing::debug!(
                    "Runtime {:?} is available at: {}",
                    runtime_type,
                    install_path.display()
                );
                Ok(Some(install_path))
            }
            Err(e) => {
                tracing::warn!("Failed to ensure runtime {:?}: {}", runtime_type, e);
                // Return error but caller should handle gracefully
                Err(e)
            }
        }
    }

    /// Save runtime configuration to database after installation
    async fn save_runtime_config_to_db(
        &self,
        pool: &sqlx::Pool<sqlx::Sqlite>,
        runtime_type: RuntimeType,
        runtime_path: &std::path::Path,
    ) -> Result<()> {
        // The runtime_path could be either:
        // 1. An installation directory (for managed runtimes)
        // 2. An executable file path (for system runtimes)

        let executable_path = if runtime_path.is_file() {
            // If it's already a file, use it directly
            runtime_path.to_path_buf()
        } else {
            // If it's a directory, find the appropriate executable
            match runtime_type {
                RuntimeType::Node => {
                    // Check if npx exists in the bin directory
                    let npx_path = runtime_path.join("bin").join("npx");
                    if npx_path.exists() {
                        npx_path
                    } else {
                        // Fall back to node executable
                        let node_path = runtime_path.join("bin").join("node");
                        if node_path.exists() {
                            node_path
                        } else {
                            runtime_path.to_path_buf()
                        }
                    }
                }
                RuntimeType::Uv => {
                    // For uv, point to the uv executable
                    let uv_path = runtime_path.join("bin").join("uv");
                    if uv_path.exists() {
                        uv_path
                    } else {
                        runtime_path.to_path_buf()
                    }
                }
                RuntimeType::Bun => {
                    // For bun, point to the bun executable
                    let bun_path = runtime_path.join("bin").join("bun");
                    if bun_path.exists() {
                        bun_path
                    } else {
                        runtime_path.to_path_buf()
                    }
                }
            }
        };

        // For system runtimes, store the absolute path directly
        // For managed runtimes, store relative path from MCPMate directory
        let mcpmate_dir = get_mcpmate_dir()?;
        let relative_path = if executable_path.starts_with(&mcpmate_dir) {
            // Managed runtime - store relative path
            executable_path
                .strip_prefix(&mcpmate_dir)
                .unwrap_or(&executable_path)
                .to_string_lossy()
                .to_string()
        } else {
            // System runtime - store absolute path
            executable_path.to_string_lossy().to_string()
        };

        // Create runtime config
        let config = config::RuntimeConfig::new(runtime_type, "latest", &relative_path);

        // Save to database
        config::save_config(pool, &config)
            .await
            .context("Failed to save runtime config to database")?;

        tracing::debug!(
            "Saved runtime config for {:?} to database: {}",
            runtime_type,
            relative_path
        );
        Ok(())
    }

    /// Ensure the specified runtime environment is available, return executable file path
    pub async fn ensure(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<PathBuf> {
        // check if the runtime is installed
        if is_runtime_installed(runtime_type, version)? {
            return get_runtime_executable_path(runtime_type, version);
        }

        // download and install the runtime
        let downloader = RuntimeDownloader::new(self.environment.clone())?;
        let installed_path = downloader
            .download_and_install(runtime_type, version)
            .await?;

        Ok(installed_path)
    }

    /// Get the installation path of the specified type of runtime
    pub fn get_runtime_path(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<PathBuf> {
        // use the centralized constant function to get the runtime executable path
        get_runtime_executable_path(runtime_type, version)
    }

    /// Get the installation directory of the specified type of runtime
    pub fn get_runtime_dir(
        &self,
        runtime_type: RuntimeType,
    ) -> Result<PathBuf> {
        // use the centralized constant function to get the runtime directory
        get_runtime_type_dir(runtime_type)
    }

    /// Check if the specified runtime environment is available
    pub fn is_runtime_available(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<bool> {
        // use the centralized constant function to check if the runtime is installed
        is_runtime_installed(runtime_type, version)
    }
}
