use crate::runtime::constants::*;
use crate::runtime::types::RuntimeType;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Runtime path manager
#[derive(Debug)]
pub struct RuntimePaths {
    base_dir: PathBuf,
}

impl RuntimePaths {
    /// Create a new path manager
    pub fn new() -> Result<Self> {
        let base_dir = get_mcpmate_dir()?;

        // Ensure base directory exists
        std::fs::create_dir_all(&base_dir)?;

        Ok(Self { base_dir })
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
        get_runtime_version_dir(runtime_type, version).unwrap_or_else(|_| {
            // if error, fallback to the old implementation
            let version = version.unwrap_or_else(|| get_default_version(runtime_type));
            self.base_dir
                .join(RUNTIMES_DIR_NAME)
                .join(get_runtime_dir_name(runtime_type))
                .join(version)
        })
    }

    /// Get the cache directory
    pub fn get_cache_dir(
        &self,
        runtime_type: RuntimeType,
    ) -> PathBuf {
        get_cache_dir(runtime_type).unwrap_or_else(|_| {
            // if error, fallback to the old implementation
            self.base_dir
                .join(CACHE_DIR_NAME)
                .join(get_runtime_dir_name(runtime_type))
        })
    }

    /// Get the temporary download directory
    pub fn get_temp_dir(&self) -> PathBuf {
        get_temp_download_dir().unwrap_or_else(|_| {
            // if error, fallback to the old implementation
            self.base_dir.join(TMP_DIR_NAME).join(DOWNLOADS_DIR_NAME)
        })
    }

    /// Create all necessary directories
    pub fn create_directories(
        &self,
        runtime_type: RuntimeType,
        version: Option<&str>,
    ) -> Result<()> {
        let runtime_dir = self.get_runtime_dir(runtime_type, version);
        let cache_dir = self.get_cache_dir(runtime_type);
        let temp_dir = self.get_temp_dir();

        std::fs::create_dir_all(&runtime_dir)?;
        std::fs::create_dir_all(&cache_dir)?;
        std::fs::create_dir_all(&temp_dir)?;

        Ok(())
    }

    /// Get the base directory
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

/// Get MCPMate's user directory
pub fn get_mcpmate_dir() -> Result<PathBuf> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot get user home directory"))?;
    Ok(home_dir.join(MCPMATE_DIR_NAME))
}

/// Convenience function: get runtime path
pub fn get_runtime_path(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<PathBuf> {
    get_runtime_executable_path(runtime_type, version)
}
