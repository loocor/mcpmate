//! MCPMate Runtime Module
//!
//! Simplified runtime management with file-system based detection.
//! Provides unified runtime management through RuntimeManager.

pub mod downloader; // Simplified downloader
pub mod installer; // Simplified installer
pub mod manager; // Unified runtime manager
pub mod types; // Core type definitions

// Re-export common types from common::env
pub use crate::common::env::{Architecture, Environment, OperatingSystem, detect_environment};

// Re-export core types and services
pub use crate::runtime::{
    downloader::RuntimeDownloader,
    installer::RuntimeInstaller,
    manager::{RuntimeCache, RuntimeInfo, RuntimeManager},
    types::{RuntimeError, RuntimeType},
};

use std::path::PathBuf;

/// Get the installation path of the runtime
/// Simplified wrapper around RuntimeManager
pub fn get_runtime_path(
    runtime_type: RuntimeType,
    _version: Option<&str>, // Version parameter kept for API compatibility but ignored
) -> anyhow::Result<PathBuf> {
    let manager = RuntimeManager::new();
    manager
        .get_executable_path(runtime_type)
        .ok_or_else(|| anyhow::anyhow!("Runtime {} not found", runtime_type.as_str()))
}
