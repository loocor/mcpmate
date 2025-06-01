//! Unified path management for MCPMate
//!
//! This module provides centralized path management for all MCPMate components,
//! eliminating duplication across runtime, conf, and other modules.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// MCPMate directory structure constants
pub mod constants {
    pub const MCPMATE_DIR_NAME: &str = ".mcpmate";
    pub const RUNTIMES_DIR_NAME: &str = "runtimes";
    pub const CACHE_DIR_NAME: &str = "cache";
    pub const CONFIG_DIR_NAME: &str = "config";
    pub const TMP_DIR_NAME: &str = "tmp";
    pub const DOWNLOADS_DIR_NAME: &str = "downloads";
    pub const BIN_DIR_NAME: &str = "bin";
    pub const DATABASE_FILE_NAME: &str = "mcpmate.db";
}

/// Centralized path manager for MCPMate
#[derive(Debug, Clone)]
pub struct MCPMatePaths {
    base_dir: PathBuf,
}

impl MCPMatePaths {
    /// Create a new path manager instance
    pub fn new() -> Result<Self> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot get user home directory"))?;
        let base_dir = home_dir.join(constants::MCPMATE_DIR_NAME);

        Ok(Self { base_dir })
    }

    /// Get the base MCPMate directory (~/.mcpmate)
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Get the runtimes directory (~/.mcpmate/runtimes)
    pub fn runtimes_dir(&self) -> PathBuf {
        self.base_dir.join(constants::RUNTIMES_DIR_NAME)
    }

    /// Get the cache directory (~/.mcpmate/cache)
    pub fn cache_dir(&self) -> PathBuf {
        self.base_dir.join(constants::CACHE_DIR_NAME)
    }

    /// Get the config directory (~/.mcpmate/config)
    pub fn config_dir(&self) -> PathBuf {
        self.base_dir.join(constants::CONFIG_DIR_NAME)
    }

    /// Get the temporary directory (~/.mcpmate/tmp)
    pub fn tmp_dir(&self) -> PathBuf {
        self.base_dir.join(constants::TMP_DIR_NAME)
    }

    /// Get the downloads directory (system temp dir)
    pub fn downloads_dir(&self) -> PathBuf {
        std::env::temp_dir().join("mcpmate-downloads")
    }

    /// Get the database file path (~/.mcpmate/mcpmate.db)
    pub fn database_path(&self) -> PathBuf {
        self.base_dir.join(constants::DATABASE_FILE_NAME)
    }

    /// Get the database URL for SQLite
    pub fn database_url(&self) -> String {
        format!("sqlite:{}", self.database_path().display())
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
            .join(constants::BIN_DIR_NAME)
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
        let dirs = [
            self.base_dir.clone(),
            self.runtimes_dir(),
            self.cache_dir(),
            self.config_dir(),
            self.tmp_dir(),
        ];

        for dir in &dirs {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
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
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }

        tracing::debug!(
            "Created runtime directories for {} {}",
            runtime_type,
            version
        );
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
    GLOBAL_PATHS
        .get_or_init(|| MCPMatePaths::new().expect("Failed to initialize global path manager"))
}

/// Convenience functions for backward compatibility
pub fn get_mcpmate_dir() -> Result<PathBuf> {
    Ok(global_paths().base_dir().to_path_buf())
}

pub fn get_runtimes_base_dir() -> Result<PathBuf> {
    Ok(global_paths().runtimes_dir())
}

pub fn get_cache_dir() -> Result<PathBuf> {
    Ok(global_paths().cache_dir())
}

/// Get the cache directory for a specific runtime
pub fn get_runtime_cache_dir(runtime_type: &str) -> PathBuf {
    global_paths().runtime_cache_dir(runtime_type)
}
