//! Runtime Manager - Unified runtime management service
//!
//! This module provides a simplified, file-system based runtime management service
//! that replaces the complex database-driven approach with direct file detection.

use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::common::{MCPMatePaths, RuntimeType, constants::commands, paths::global_paths};

/// Unified runtime management service
///
/// Provides simple, reliable runtime management without database dependencies.
/// All operations are based on direct file system checks.
#[derive(Debug, Clone)]
pub struct RuntimeManager {
    /// Base runtimes directory (~/.mcpmate/runtimes)
    runtimes_dir: PathBuf,
}

impl RuntimeManager {
    /// Create a new runtime manager using global paths
    pub fn new() -> Self {
        let runtimes_dir = global_paths().runtimes_dir();
        Self { runtimes_dir }
    }

    /// Create a new runtime manager with explicit paths
    pub fn with_paths(paths: &MCPMatePaths) -> Self {
        Self {
            runtimes_dir: paths.runtimes_dir(),
        }
    }

    /// Check if a runtime is installed
    pub fn is_installed(
        &self,
        runtime_type: RuntimeType,
    ) -> bool {
        self.get_executable_path(runtime_type).is_some()
    }

    /// Get the executable path for a runtime.
    pub fn get_executable_path(
        &self,
        runtime_type: RuntimeType,
    ) -> Option<PathBuf> {
        let candidates: &[&str] = match runtime_type {
            RuntimeType::Uv => &[commands::UV, commands::UVX],
            RuntimeType::Bun => &[commands::BUN, commands::BUNX],
            RuntimeType::Node => &[commands::NODE, commands::NPM, commands::NPX],
        };

        candidates
            .iter()
            .find_map(|command| self.resolve_command_path(runtime_type, command))
    }

    /// Get the executable path for an exact command alias.
    pub fn get_command_path(
        &self,
        command: &str,
    ) -> Option<PathBuf> {
        let runtime_type = RuntimeType::from_command(command)?;
        self.resolve_command_path(runtime_type, command)
    }

    fn resolve_command_path(
        &self,
        runtime_type: RuntimeType,
        command: &str,
    ) -> Option<PathBuf> {
        let runtime_dir = self.runtimes_dir.join(runtime_type.as_str());
        let executable_name = runtime_type.executable_name_for_command(command);
        let mut candidates = vec![runtime_dir.join(&executable_name)];

        if matches!(runtime_type, RuntimeType::Node) {
            candidates.push(runtime_dir.join("bin").join(&executable_name));
        }

        if let Some(path) = candidates.into_iter().find(|path| path.is_file()) {
            return Some(path);
        }

        self.find_nested_executable(&runtime_dir, &executable_name)
    }

    fn find_nested_executable(
        &self,
        runtime_dir: &Path,
        executable_name: &str,
    ) -> Option<PathBuf> {
        if !runtime_dir.exists() {
            return None;
        }

        for entry in WalkDir::new(runtime_dir)
            .min_depth(1)
            .max_depth(6)
            .into_iter()
            .filter_map(|entry| match entry {
                Ok(e) => Some(e),
                Err(e) => {
                    tracing::debug!("WalkDir error scanning runtimes: {}", e);
                    None
                }
            })
        {
            if !entry.file_type().is_file() {
                continue;
            }

            if entry.file_name().to_string_lossy() == executable_name {
                return Some(entry.path().to_path_buf());
            }
        }

        None
    }

    /// List all installed runtimes
    pub fn list_installed(&self) -> Vec<RuntimeInfo> {
        let mut runtimes = Vec::new();

        for &runtime_type in RuntimeType::all() {
            let info = RuntimeInfo {
                runtime_type,
                available: self.is_installed(runtime_type),
                path: self.get_executable_path(runtime_type),
                message: self.get_status_message(runtime_type),
            };
            runtimes.push(info);
        }

        runtimes
    }

    /// Get status message for a runtime with source information
    fn get_status_message(
        &self,
        runtime_type: RuntimeType,
    ) -> String {
        if let Some(path) = self.get_executable_path(runtime_type) {
            format!(
                "✓ {} is available (MCPMate managed at {})",
                runtime_type.as_str(),
                path.display()
            )
        } else {
            format!("✗ {} is not installed", runtime_type.as_str())
        }
    }

    /// Ensure runtimes directory exists
    pub fn ensure_runtimes_dir(&self) -> Result<()> {
        if !self.runtimes_dir.exists() {
            std::fs::create_dir_all(&self.runtimes_dir)?;
            tracing::info!("Created runtimes directory: {}", self.runtimes_dir.display());
        }
        Ok(())
    }

    /// Get the runtimes directory path
    pub fn runtimes_dir(&self) -> &PathBuf {
        &self.runtimes_dir
    }

    /// Set custom cache directory (for future cache management)
    pub fn with_cache_dir(
        self,
        cache_dir: PathBuf,
    ) -> Self {
        // For now, we don't use cache_dir, but this provides the interface
        // for future cache directory management
        tracing::info!("Custom cache directory set: {}", cache_dir.display());
        self
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Runtime information
#[derive(Debug, Clone)]
pub struct RuntimeInfo {
    /// Runtime type
    pub runtime_type: RuntimeType,
    /// Whether the runtime is available
    pub available: bool,
    /// Path to the executable (if available)
    pub path: Option<PathBuf>,
    /// Status message
    pub message: String,
}
