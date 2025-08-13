//! Unified environment variable management for MCPMate
//!
//! This module provides centralized environment variable management,
//! eliminating duplication across runtime and conf modules.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use tokio::process::Command;

use super::{paths::global_paths, types::RuntimeType};

// Re-export constants from the central constants module
pub use super::constants::env_vars as constants;
pub use super::constants::separators::get_path_separator;

/// System environment information (from runtime/detection.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub os: OperatingSystem,
    pub arch: Architecture,
}

/// Operating system type (from runtime/detection.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatingSystem {
    Windows,
    MacOS,
    Linux,
}

/// System architecture (from runtime/detection.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Architecture {
    X86_64,
    Aarch64,
}

impl OperatingSystem {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperatingSystem::Windows => "windows",
            OperatingSystem::MacOS => "macos",
            OperatingSystem::Linux => "linux",
        }
    }

    /// Get file extension
    pub fn archive_extension(&self) -> &'static str {
        match self {
            OperatingSystem::Windows => "zip",
            OperatingSystem::MacOS | OperatingSystem::Linux => "tar.gz",
        }
    }
}

impl Architecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::Aarch64 => "aarch64",
        }
    }

    /// Get Node.js architecture name
    pub fn node_arch(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x64",
            Architecture::Aarch64 => "arm64",
        }
    }
}

/// Detect current system environment (from runtime/detection.rs)
pub fn detect_environment() -> Result<Environment> {
    let os = detect_os()?;
    let arch = detect_arch()?;

    Ok(Environment { os, arch })
}

/// Detect operating system (from runtime/detection.rs)
fn detect_os() -> Result<OperatingSystem> {
    match env::consts::OS {
        "windows" => Ok(OperatingSystem::Windows),
        "macos" => Ok(OperatingSystem::MacOS),
        "linux" => Ok(OperatingSystem::Linux),
        other => Err(anyhow::anyhow!("Unsupported operating system: {}", other)),
    }
}

/// Detect system architecture (from runtime/detection.rs)
fn detect_arch() -> Result<Architecture> {
    match env::consts::ARCH {
        "x86_64" => Ok(Architecture::X86_64),
        "aarch64" => Ok(Architecture::Aarch64),
        other => Err(anyhow::anyhow!(
            "Unsupported system architecture: {}",
            other
        )),
    }
}

/// Environment manager for runtime commands
#[derive(Debug, Clone)]
pub struct EnvironmentManager {
    base_env: HashMap<String, String>,
}

impl EnvironmentManager {
    /// Create a new environment manager
    pub fn new() -> Self {
        Self {
            base_env: HashMap::new(),
        }
    }

    /// Add environment variable
    pub fn set_var<K, V>(
        &mut self,
        key: K,
        value: V,
    ) -> &mut Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.base_env.insert(key.into(), value.into());
        self
    }

    /// Add multiple environment variables
    pub fn set_vars(
        &mut self,
        vars: HashMap<String, String>,
    ) -> &mut Self {
        self.base_env.extend(vars);
        self
    }

    /// Prepend to PATH environment variable
    pub fn prepend_path<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> &mut Self {
        let new_path = path.as_ref().to_string_lossy().to_string();
        let current_path = self
            .base_env
            .get(constants::PATH)
            .cloned()
            .or_else(|| env::var(constants::PATH).ok())
            .unwrap_or_default();

        let separator = get_path_separator();
        let updated_path = if current_path.is_empty() {
            new_path
        } else {
            format!("{}{}{}", new_path, separator, current_path)
        };

        self.set_var(constants::PATH, updated_path);
        self
    }

    /// Apply environment to a command
    pub fn apply_to_command(
        &self,
        command: &mut Command,
    ) {
        for (key, value) in &self.base_env {
            command.env(key, value);
        }
    }

    /// Get environment variables as HashMap
    pub fn as_map(&self) -> &HashMap<String, String> {
        &self.base_env
    }
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create runtime-specific environment for uv
pub fn create_uv_environment(
    bin_path: &Path,
    version: &str,
) -> Result<EnvironmentManager> {
    let paths = global_paths();
    let mut env = EnvironmentManager::new();

    // Add runtime bin directory to PATH
    let bin_dir = bin_path.parent().unwrap_or(bin_path);
    env.prepend_path(bin_dir);

    // Set uv specific environment variables (simplified for system uvx)
    let cache_dir = paths.runtime_cache_dir(RuntimeType::Uv.as_str());

    // Ensure cache directory exists
    std::fs::create_dir_all(&cache_dir)?;

    env.set_var(constants::UV_CACHE_DIR, cache_dir.to_string_lossy());

    // Set runtime bin path for reference
    env.set_var(constants::MCP_RUNTIME_BIN, bin_path.to_string_lossy());

    // Set specific tool paths
    let uvx_path = bin_dir.join(if cfg!(windows) { "uvx.exe" } else { "uvx" });
    if uvx_path.exists() {
        env.set_var(constants::UVX_BIN_PATH, uvx_path.to_string_lossy());
    }

    tracing::debug!(
        "Created uv environment for version {}: PATH includes {}, cache at {}",
        version,
        bin_dir.display(),
        cache_dir.display()
    );

    Ok(env)
}

/// Create runtime-specific environment for Bun
pub fn create_bun_environment(
    bin_path: &Path,
    version: &str,
) -> Result<EnvironmentManager> {
    let paths = global_paths();
    let mut env = EnvironmentManager::new();

    // Add runtime bin directory to PATH
    let bin_dir = bin_path.parent().unwrap_or(bin_path);
    env.prepend_path(bin_dir);

    // Set Bun specific environment variables
    let cache_dir = paths.runtime_cache_dir(RuntimeType::Bun.as_str());

    // Ensure cache directory exists
    std::fs::create_dir_all(&cache_dir)?;

    env.set_var(
        constants::BUN_INSTALL_CACHE_DIR,
        cache_dir.to_string_lossy(),
    );

    // Set runtime bin path for reference
    env.set_var(constants::MCP_RUNTIME_BIN, bin_path.to_string_lossy());

    // Set specific tool paths
    let bunx_path = bin_dir.join(if cfg!(windows) { "bunx.exe" } else { "bunx" });
    if bunx_path.exists() {
        env.set_var(constants::BUNX_BIN_PATH, bunx_path.to_string_lossy());
    }

    tracing::debug!(
        "Created Bun environment for version {}: PATH includes {}, cache at {}",
        version,
        bin_dir.display(),
        cache_dir.display()
    );

    Ok(env)
}

/// Create environment for a specific runtime type
pub fn create_runtime_environment(
    runtime_type: &str,
    bin_path: &Path,
    version: &str,
) -> Result<EnvironmentManager> {
    use super::types::RuntimeType;
    use std::str::FromStr;
    
    if let Ok(rt) = RuntimeType::from_str(runtime_type) {
        match rt {
            RuntimeType::Uv => create_uv_environment(bin_path, version),
            RuntimeType::Bun => create_bun_environment(bin_path, version),
        }
    } else {
        // Generic runtime environment
        let mut env = EnvironmentManager::new();
        let bin_dir = bin_path.parent().unwrap_or(bin_path);
        env.prepend_path(bin_dir);
        env.set_var(constants::MCP_RUNTIME_BIN, bin_path.to_string_lossy());
        Ok(env)
    }
}

/// Prepare command environment with runtime-specific settings
pub fn prepare_command_environment(
    command: &mut Command,
    runtime_type: &str,
    bin_path: &Path,
    version: &str,
) -> Result<()> {
    let env = create_runtime_environment(runtime_type, bin_path, version)?;
    env.apply_to_command(command);
    Ok(())
}
