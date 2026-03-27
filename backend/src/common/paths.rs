//! Unified path management for MCPMate
//!
//! This module provides centralized path management for all MCPMate components,
//! eliminating duplication across runtime, conf, and other modules.

use anyhow::{Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
};

use super::constants;
use super::types::RuntimeType;

/// Centralized path manager for MCPMate
#[derive(Debug, Clone)]
pub struct MCPMatePaths {
    base_dir: PathBuf,
}

impl MCPMatePaths {
    /// Create a new path manager instance
    pub fn new() -> Result<Self> {
        if let Ok(override_dir) = env::var("MCPMATE_DATA_DIR") {
            let trimmed = override_dir.trim();
            if !trimmed.is_empty() {
                return Self::from_base_dir(trimmed);
            }
        }

        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot get user home directory"))?;
        let base_dir = home_dir.join(constants::paths::MCPMATE_DIR_NAME);

        Ok(Self { base_dir })
    }

    /// Construct path manager from a custom base directory
    pub fn from_base_dir<P: Into<PathBuf>>(base_dir: P) -> Result<Self> {
        let mut base_dir = base_dir.into();
        if !base_dir.is_absolute() {
            let cwd = env::current_dir().context("Cannot determine current working directory")?;
            base_dir = cwd.join(base_dir);
        }

        Ok(Self { base_dir })
    }

    /// Get the base MCPMate directory (~/.mcpmate)
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get the runtimes directory (~/.mcpmate/runtimes)
    pub fn runtimes_dir(&self) -> PathBuf {
        self.base_dir.join(constants::paths::RUNTIMES_DIR_NAME)
    }

    /// Get the cache directory (~/.mcpmate/cache)
    pub fn cache_dir(&self) -> PathBuf {
        self.base_dir.join(constants::paths::CACHE_DIR_NAME)
    }

    /// Get the logs directory (~/.mcpmate/logs)
    pub fn logs_dir(&self) -> PathBuf {
        self.base_dir.join("logs")
    }

    /// Get the downloads directory (system temp dir)
    pub fn downloads_dir(&self) -> PathBuf {
        std::env::temp_dir().join("mcpmate-downloads")
    }

    /// Get the database file path (~/.mcpmate/mcpmate.db)
    pub fn database_path(&self) -> PathBuf {
        self.base_dir.join(constants::paths::DATABASE_FILE_NAME)
    }

    /// Get the database URL for SQLite
    pub fn database_url(&self) -> String {
        format!("sqlite:{}", self.database_path().display())
    }

    pub fn audit_database_path(&self) -> PathBuf {
        self.base_dir.join(constants::paths::AUDIT_DATABASE_FILE_NAME)
    }

    pub fn audit_database_url(&self) -> String {
        format!("sqlite:{}", self.audit_database_path().display())
    }

    /// Get runtime-specific directory (~/.mcpmate/runtimes/{runtime_type})
    pub fn runtime_type_dir(
        &self,
        runtime_type: &str,
    ) -> PathBuf {
        self.runtimes_dir().join(runtime_type)
    }

    /// Get runtime version directory (~/.mcpmate/runtimes/{runtime_type}/{version})
    pub fn runtime_version_dir(
        &self,
        runtime_type: &str,
        version: &str,
    ) -> PathBuf {
        self.runtime_type_dir(runtime_type).join(version)
    }

    /// Get runtime bin directory (~/.mcpmate/runtimes/{runtime_type}/{version}/bin)
    pub fn runtime_bin_dir(
        &self,
        runtime_type: &str,
        version: &str,
    ) -> PathBuf {
        self.runtime_version_dir(runtime_type, version)
            .join(constants::paths::BIN_DIR_NAME)
    }

    /// Get runtime cache directory (~/.mcpmate/cache/{runtime_type})
    pub fn runtime_cache_dir(
        &self,
        runtime_type: &str,
    ) -> PathBuf {
        self.cache_dir().join(runtime_type)
    }

    /// Create all necessary directories
    pub fn ensure_directories(&self) -> Result<()> {
        let dirs = [self.base_dir.clone(), self.runtimes_dir(), self.cache_dir()];

        for dir in &dirs {
            std::fs::create_dir_all(dir).with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }

        std::fs::create_dir_all(self.downloads_dir()).with_context(|| {
            format!(
                "Failed to create downloads directory: {}",
                self.downloads_dir().display()
            )
        })?;

        tracing::debug!("Created MCPMate directory structure");
        Ok(())
    }

    /// Create runtime-specific directories
    pub fn ensure_runtime_directories(
        &self,
        runtime_type: &str,
        version: &str,
    ) -> Result<()> {
        let dirs = [
            self.runtime_type_dir(runtime_type),
            self.runtime_version_dir(runtime_type, version),
            self.runtime_bin_dir(runtime_type, version),
            self.runtime_cache_dir(runtime_type),
        ];

        for dir in &dirs {
            std::fs::create_dir_all(dir).with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }

        tracing::debug!("Created runtime directories for {} {}", runtime_type, version);
        Ok(())
    }

    /// Convert absolute path to relative path from MCPMate base directory
    pub fn to_relative_path(
        &self,
        absolute_path: &Path,
    ) -> Result<PathBuf> {
        if absolute_path.starts_with(&self.base_dir) {
            absolute_path
                .strip_prefix(&self.base_dir)
                .map(|p| PathBuf::from(".mcpmate").join(p))
                .context("Failed to create relative path")
        } else {
            // For system paths, return as-is
            Ok(absolute_path.to_path_buf())
        }
    }

    /// Convert relative path to absolute path
    pub fn to_absolute_path(
        &self,
        relative_path: &Path,
    ) -> PathBuf {
        if relative_path.starts_with(".mcpmate") {
            // Remove .mcpmate prefix and join with base directory
            if let Ok(stripped) = relative_path.strip_prefix(".mcpmate") {
                self.base_dir.join(stripped)
            } else {
                self.base_dir.join(relative_path)
            }
        } else if relative_path.is_absolute() {
            // Already absolute
            relative_path.to_path_buf()
        } else {
            // Relative to base directory
            self.base_dir.join(relative_path)
        }
    }
}

/// Global path manager instance
static GLOBAL_PATHS: std::sync::OnceLock<MCPMatePaths> = std::sync::OnceLock::new();

/// Get the global path manager instance
pub fn global_paths() -> &'static MCPMatePaths {
    GLOBAL_PATHS.get_or_init(|| MCPMatePaths::new().expect("Failed to initialize global path manager"))
}

/// Initialize the global path manager with a custom instance.
///
/// This must be called before [`global_paths`] is used anywhere else.
pub fn set_global_paths(paths: MCPMatePaths) -> Result<()> {
    GLOBAL_PATHS
        .set(paths)
        .map_err(|_| anyhow::anyhow!("Global MCPMate paths already initialized"))
}

/// Get the bridge component path dynamically
///
/// Resolves the bridge executable path based on the current server executable location.
/// This replaces hardcoded paths with dynamic resolution.
///
/// Priority order:
/// 1. MCPMATE_BRIDGE_PATH environment variable (if set)
/// 2. Same directory as current executable
/// 3. Return error with helpful guidance
pub fn get_bridge_path() -> Result<String> {
    // Check for environment variable override first
    if let Ok(env_path) = std::env::var("MCPMATE_BRIDGE_PATH") {
        if !env_path.is_empty() {
            return Ok(env_path);
        }
    }

    let bridge_name = format!("bridge{}", RuntimeType::executable_extension());

    // Try to find bridge executable in the same directory as current executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let bridge_path = exe_dir.join(&bridge_name);
            if bridge_path.exists() {
                tracing::debug!("Found bridge at: {}", bridge_path.display());
                return Ok(bridge_path.to_string_lossy().to_string());
            }
        }
    }

    // If we can't find the bridge, provide helpful error message
    Err(anyhow::anyhow!(
        "Bridge executable '{}' not found.\n\n\
        The bridge component should be located in the same directory as the current executable.\n\
        To resolve this issue:\n\
        1. Ensure MCPMate is properly installed with all components\n\
        2. Set MCPMATE_BRIDGE_PATH environment variable to specify the bridge location\n\
        3. Verify the bridge executable exists and has proper permissions",
        bridge_name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    #[test]
    fn from_base_dir_uses_absolute_path() {
        let tmp = tempdir().expect("tmp dir");
        let nested = tmp.path().join("nested");
        let paths = MCPMatePaths::from_base_dir(&nested).expect("construct");
        assert_eq!(paths.base_dir(), nested.as_path());
    }

    #[test]
    fn env_override_selected_when_present() {
        let tmp = tempdir().expect("tmp dir");
        let desired = tmp.path().join("custom");
        unsafe {
            env::set_var("MCPMATE_DATA_DIR", &desired);
        }
        let paths = MCPMatePaths::new().expect("construct");
        unsafe {
            env::remove_var("MCPMATE_DATA_DIR");
        }
        assert_eq!(paths.base_dir(), desired.as_path());
    }
}
