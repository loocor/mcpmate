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
pub mod executable; // Runtime executable path utilities
// Detection now in common/env.rs
mod download; // Download management related functionality
pub mod init; // Runtime database initialization
mod installers; // Installer related functionality
pub mod integration; // Runtime integration utilities (database, events)
pub mod list; // List available runtimes
pub mod migration; // Runtime migration utilities
// paths.rs merged into common/paths.rs
pub mod types; // Type definitions

// Re-export common types from common::env
pub use crate::common::env::{Architecture, Environment, OperatingSystem, detect_environment};

// Re-export runtime downloader and related types
pub use crate::runtime::{
    download::{RuntimeDownloader, download_runtime, download_runtime_with_config},
    installers::{bun::BunInstaller, node::NodeInstaller, uv::UvInstaller},
    types::{DownloadConfig, DownloadProgress, DownloadStage, RuntimeError, RuntimeType},
};

// Re-export cache/config types
pub use crate::runtime::cache::RuntimeCache;
pub use crate::runtime::config::RuntimeConfig;

use crate::common::paths::global_paths;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Get the installation path of the runtime
pub fn get_runtime_path(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<PathBuf> {
    executable::get_runtime_executable_path(runtime_type, version)
}

/// Get runtime config for specific path
pub fn get_runtime_config_for_path(executable_path: &Path) -> Result<RuntimeConfig> {
    let _paths = global_paths();

    // 导入 FromStr trait
    use std::str::FromStr;

    let runtime_type = RuntimeType::from_str(&executable_path.to_string_lossy())
        .map_err(|e| anyhow::anyhow!("Failed to determine runtime type: {}", e))?;

    let version = extract_version_from_path(executable_path)?;
    let relative_bin_path = executable_path.to_string_lossy().to_string();

    // Create config with proper types
    Ok(RuntimeConfig::new(
        runtime_type,
        &version,
        &relative_bin_path,
    ))
}

/// Extract version from runtime path
fn extract_version_from_path(path: &Path) -> Result<String> {
    // Typical path pattern:
    // ~/.mcpmate/runtimes/node/v18.0.0/bin/node
    // ~/.mcpmate/runtimes/bun/v1.0.0/bin/bun

    let path_str = path.to_string_lossy();

    // Check for version directory pattern (v12.0.0)
    if let Some(version_pos) = path_str.find("/v") {
        if let Some(end_pos) = path_str[version_pos + 1..].find('/') {
            return Ok(path_str[version_pos + 1..version_pos + 1 + end_pos].to_string());
        }
    }

    // Fallback: extract from parent directory name
    if let Some(parent) = path.parent() {
        if let Some(parent_parent) = parent.parent() {
            if let Some(version_dir) = parent_parent.file_name() {
                let version = version_dir.to_string_lossy();
                if version.starts_with('v') || version.chars().next().unwrap_or('x').is_ascii_digit() {
                    return Ok(version.to_string());
                }
            }
        }
    }

    // Default to "unknown" if we couldn't determine version
    Ok("unknown".to_string())
}
