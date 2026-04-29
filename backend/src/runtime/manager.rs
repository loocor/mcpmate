//! Runtime Manager - Unified runtime management service
//!
//! This module provides a simplified, file-system based runtime management service
//! that replaces the complex database-driven approach with direct file detection.

use anyhow::Result;
use std::path::PathBuf;

use crate::common::{RuntimeType, constants::commands, paths::global_paths};

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
    /// Create a new runtime manager
    pub fn new() -> Self {
        let runtimes_dir = global_paths().runtimes_dir();
        Self { runtimes_dir }
    }

    /// Check if a runtime is installed
    pub fn is_installed(
        &self,
        runtime_type: RuntimeType,
    ) -> bool {
        self.get_executable_path(runtime_type).is_some()
    }

    /// Get the executable path for a runtime
    pub fn get_executable_path(
        &self,
        runtime_type: RuntimeType,
    ) -> Option<PathBuf> {
        // Check for MCPMate managed executables (preferred for version consistency and security)
        match runtime_type {
            RuntimeType::Uv => {
                let runtime_dir = self.runtimes_dir.join(runtime_type.as_str());

                // Check for uvx first (preferred for MCP servers)
                let uvx_name = runtime_type.executable_name_for_command(commands::UVX);
                let uvx_path = runtime_dir.join(uvx_name);
                if uvx_path.exists() {
                    return Some(uvx_path);
                }

                // Fall back to uv
                let runtime_name = runtime_type.executable_name();
                let runtime_path = runtime_dir.join(runtime_name);
                if runtime_path.exists() {
                    Some(runtime_path)
                } else {
                    None
                }
            }
            RuntimeType::Bun => {
                let runtime_dir = self.runtimes_dir.join(runtime_type.as_str());

                // Check for bunx first (preferred)
                let bunx_name = runtime_type.executable_name_for_command(commands::BUNX);
                let bunx_path = runtime_dir.join(bunx_name);
                if bunx_path.exists() {
                    return Some(bunx_path);
                }

                // Fall back to bun
                let runtime_name = runtime_type.executable_name();
                let runtime_path = runtime_dir.join(runtime_name);
                if runtime_path.exists() {
                    Some(runtime_path)
                } else {
                    None
                }
            }
        }
    }

    /// Get runtime path for a command (uvx -> Uv, bunx -> Bun)
    /// Note: npx is handled by command transformation to bunx
    pub fn get_runtime_for_command(
        &self,
        command: &str,
    ) -> Option<PathBuf> {
        let runtime_type = match command {
            commands::UVX => RuntimeType::Uv,
            commands::BUNX => RuntimeType::Bun,
            _ => return None,
        };

        self.get_executable_path(runtime_type)
    }

    /// List all installed runtimes
    pub fn list_installed(&self) -> Vec<RuntimeInfo> {
        let mut runtimes = Vec::new();

        for runtime_type in [RuntimeType::Uv, RuntimeType::Bun] {
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

impl RuntimeInfo {
    /// Get display name for the runtime
    pub fn display_name(&self) -> &str {
        self.runtime_type.as_str()
    }
}

/// Simple runtime cache for backward compatibility
/// This is a simplified version that wraps RuntimeManager
#[derive(Debug, Clone)]
pub struct RuntimeCache {
    manager: RuntimeManager,
}

impl RuntimeCache {
    /// Create a new runtime cache
    pub fn new() -> Self {
        Self {
            manager: RuntimeManager::new(),
        }
    }

    /// Get runtime path for a command (npx -> Bun, uvx -> Uv, etc.)
    pub async fn get_runtime_for_command(
        &self,
        command: &str,
    ) -> Option<std::path::PathBuf> {
        self.manager.get_runtime_for_command(command)
    }
}

impl Default for RuntimeCache {
    fn default() -> Self {
        Self::new()
    }
}
