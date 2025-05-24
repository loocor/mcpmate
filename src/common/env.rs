//! Unified environment variable management for MCPMate
//!
//! This module provides centralized environment variable management,
//! eliminating duplication across runtime and conf modules.

use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use tokio::process::Command;

use super::paths::global_paths;

/// Environment variable constants
pub mod constants {
    pub const MCP_RUNTIME_BIN: &str = "MCP_RUNTIME_BIN";
    pub const NPX_BIN_PATH: &str = "NPX_BIN_PATH";
    pub const UVX_BIN_PATH: &str = "UVX_BIN_PATH";
    pub const BUNX_BIN_PATH: &str = "BUNX_BIN_PATH";
    pub const NPM_CONFIG_CACHE: &str = "NPM_CONFIG_CACHE";
    pub const UV_CACHE_DIR: &str = "UV_CACHE_DIR";
    pub const UV_PYTHON_CACHE_DIR: &str = "UV_PYTHON_CACHE_DIR";
    pub const UV_PYTHON_INSTALL_DIR: &str = "UV_PYTHON_INSTALL_DIR";
    pub const BUN_INSTALL_CACHE_DIR: &str = "BUN_INSTALL_CACHE_DIR";
    pub const PATH: &str = "PATH";
}

/// Platform-specific path separator
pub fn get_path_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
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

/// Create runtime-specific environment for Node.js
pub fn create_node_environment(
    bin_path: &Path,
    version: &str,
) -> Result<EnvironmentManager> {
    let paths = global_paths();
    let mut env = EnvironmentManager::new();

    // Add runtime bin directory to PATH
    let bin_dir = bin_path.parent().unwrap_or(bin_path);
    env.prepend_path(bin_dir);

    // Set Node.js specific environment variables
    let cache_dir = paths.runtime_cache_dir("npm");
    env.set_var(constants::NPM_CONFIG_CACHE, cache_dir.to_string_lossy());

    // Set runtime bin path for reference
    env.set_var(constants::MCP_RUNTIME_BIN, bin_path.to_string_lossy());

    // Set specific tool paths
    let npx_path = bin_dir.join(if cfg!(windows) { "npx.cmd" } else { "npx" });
    if npx_path.exists() {
        env.set_var(constants::NPX_BIN_PATH, npx_path.to_string_lossy());
    }

    tracing::debug!(
        "Created Node.js environment for version {}: PATH includes {}, cache at {}",
        version,
        bin_dir.display(),
        cache_dir.display()
    );

    Ok(env)
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

    // Set uv specific environment variables
    let cache_dir = paths.runtime_cache_dir("uv");
    let python_cache_dir = paths.cache_dir().join("uv").join("python");
    let python_install_dir = paths.runtimes_dir().join("uv").join("python");

    // Ensure directories exist
    std::fs::create_dir_all(&cache_dir)?;
    std::fs::create_dir_all(&python_cache_dir)?;
    std::fs::create_dir_all(&python_install_dir)?;

    env.set_var(constants::UV_CACHE_DIR, cache_dir.to_string_lossy())
        .set_var(
            constants::UV_PYTHON_CACHE_DIR,
            python_cache_dir.to_string_lossy(),
        )
        .set_var(
            constants::UV_PYTHON_INSTALL_DIR,
            python_install_dir.to_string_lossy(),
        );

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
    let cache_dir = paths.runtime_cache_dir("bun");
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
    match runtime_type.to_lowercase().as_str() {
        "node" | "nodejs" => create_node_environment(bin_path, version),
        "uv" => create_uv_environment(bin_path, version),
        "bun" | "bunjs" => create_bun_environment(bin_path, version),
        _ => {
            // Generic runtime environment
            let mut env = EnvironmentManager::new();
            let bin_dir = bin_path.parent().unwrap_or(bin_path);
            env.prepend_path(bin_dir);
            env.set_var(constants::MCP_RUNTIME_BIN, bin_path.to_string_lossy());
            Ok(env)
        }
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
