//! MCPMate Runtime Module
//!
//! This module provides MCPMate's runtime environment management functionality, including:
//! - Environment detection: detect current system environment and architecture
//! - Runtime download: download and manage various runtime environments
//! - Path management: manage runtime environment installation paths
//! - Version control: manage different versions of runtime environments

pub mod constants; // Path constants and utilities
mod detection; // Environment detection related functionality
mod download; // Download management related functionality
mod installers; // Installer related functionality
mod list; // List installed runtime environments
mod paths; // Path management related functionality
mod types; // Type definitions

// Re-export main types and functions
pub use constants::*; // Export all path constants
pub use detection::{Environment, detect_environment};
pub use download::{
    InlineProgressBar, MultiLineProgress, RuntimeDownloader, download_runtime,
    download_runtime_with_config, supports_inline_progress,
};
pub use installers::{bun::BunInstaller, node::NodeInstaller, uv::UvInstaller};
pub use list::list_runtime;
pub use paths::{RuntimePaths, get_runtime_path, show_runtime_path};
pub use types::{
    Commands, DownloadConfig, DownloadProgress, DownloadStage, RuntimeError, RuntimeType,
};

use anyhow::Result;
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
