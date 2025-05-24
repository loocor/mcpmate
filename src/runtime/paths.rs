//! Runtime path management
//!
//! This module provides path management for runtime installations,
//! now using the shared path management system.

use crate::common::paths::{MCPMatePaths, global_paths};
use crate::runtime::types::RuntimeType;
use crate::runtime::{RuntimeManager, constants::*};
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Runtime path manager
///
/// This is now a thin wrapper around the shared path management system
#[derive(Debug)]
pub struct RuntimePaths {
    paths: &'static MCPMatePaths,
}

impl RuntimePaths {
    /// Create a new path manager
    pub fn new() -> Result<Self> {
        let paths = global_paths();

        // Ensure base directory exists
        paths.ensure_directories()?;

        Ok(Self { paths })
    }

    /// Get the installation path of the runtime
    pub fn get_runtime_path(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<PathBuf> {
        get_runtime_executable_path(runtime_type, version)
    }

    /// Get the installation directory of the runtime
    pub fn get_runtime_dir(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> PathBuf {
        let version = version.unwrap_or_else(|| get_default_version(runtime_type));
        let runtime_name = get_runtime_dir_name(runtime_type);
        self.paths.runtime_version_dir(runtime_name, version)
    }

    /// Get the cache directory
    pub fn get_cache_dir(
        &self,
        runtime_type: RuntimeType,
    ) -> PathBuf {
        let runtime_name = get_runtime_dir_name(runtime_type);
        self.paths.runtime_cache_dir(runtime_name)
    }

    /// Get the temporary download directory
    pub fn get_temp_dir(&self) -> PathBuf {
        self.paths.downloads_dir()
    }

    /// Create all necessary directories
    pub fn create_directories(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<()> {
        let version = version.unwrap_or_else(|| get_default_version(runtime_type));
        let runtime_name = get_runtime_dir_name(runtime_type);

        self.paths
            .ensure_runtime_directories(runtime_name, version)?;

        Ok(())
    }

    /// Get the base directory
    pub fn base_dir(&self) -> &Path {
        self.paths.base_dir()
    }

    /// Get the bin directory for a specific runtime version
    pub fn get_bin_dir(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> PathBuf {
        let version = version.unwrap_or_else(|| get_default_version(runtime_type));
        let runtime_name = get_runtime_dir_name(runtime_type);
        self.paths.runtime_bin_dir(runtime_name, version)
    }
}

impl Default for RuntimePaths {
    fn default() -> Self {
        Self::new().expect("Failed to create RuntimePaths")
    }
}

/// Convenience function: get runtime path
pub fn get_runtime_path(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<PathBuf> {
    get_runtime_executable_path(runtime_type, version)
}

/// Show the path to a runtime installation
pub fn show_runtime_path(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<()> {
    let runtime_manager = RuntimeManager::new()?;
    let path = runtime_manager.get_runtime_path(runtime_type, version)?;
    println!("{}", path.display());
    Ok(())
}
