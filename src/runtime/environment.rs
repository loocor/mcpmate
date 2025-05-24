//! Environment management for runtime environments

use anyhow::{Context, Result};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};
use tokio::process::Command as TokioCommand;

use super::constants::*;
use super::types::{RuntimeType, RuntimeVersion};

/// Get the user's home directory
pub fn get_user_home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Get the MCPMate runtime directory
pub fn get_runtime_dir() -> PathBuf {
    get_user_home().join(".mcpmate").join("runtimes")
}

/// Get the MCPMate cache directory
pub fn get_cache_dir() -> PathBuf {
    get_user_home().join(".mcpmate").join("cache")
}

/// Get the MCPMate temporary directory
pub fn get_temp_dir() -> PathBuf {
    get_user_home().join(".mcpmate").join("tmp")
}

/// Get the MCPMate downloads directory
pub fn get_downloads_dir() -> PathBuf {
    get_temp_dir().join("downloads")
}

/// Create the necessary directory structure for MCPMate runtime environments
pub fn create_directories() -> Result<()> {
    let dirs = [
        get_runtimes_base_dir()?,
        get_runtime_type_dir(RuntimeType::Node)?,
        get_runtime_type_dir(RuntimeType::Bun)?,
        get_runtime_type_dir(RuntimeType::Uv)?,
        get_cache_dir(RuntimeType::Node)?,
        get_cache_dir(RuntimeType::Bun)?,
        get_cache_dir(RuntimeType::Uv)?,
        get_temp_download_dir()?,
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
    }

    tracing::debug!("Created MCPMate runtime directories");
    Ok(())
}

/// Get the runtime binary directory for a specific runtime type and version
pub fn get_runtime_bin_dir(
    runtime_type: RuntimeType,
    version: &str,
) -> PathBuf {
    match runtime_type {
        RuntimeType::Node => get_runtime_dir().join("node").join(version).join("bin"),
        RuntimeType::Bun => get_runtime_dir().join("bun").join(version).join("bin"),
        RuntimeType::Uv => get_runtime_dir().join("uv").join(version).join("bin"),
    }
}

/// Get the runtime cache directory for a specific runtime type
pub fn get_runtime_cache_dir(runtime_type: RuntimeType) -> PathBuf {
    match runtime_type {
        RuntimeType::Node => get_cache_dir().join("npm"),
        RuntimeType::Bun => get_cache_dir().join("bun"),
        RuntimeType::Uv => get_cache_dir().join("uv"),
    }
}

/// Check if a runtime is installed
pub fn is_runtime_installed(
    runtime_type: RuntimeType,
    version: &str,
) -> bool {
    let bin_dir = get_runtime_bin_dir(runtime_type, version);

    match runtime_type {
        RuntimeType::Node => bin_dir.join("node").exists(),
        RuntimeType::Bun => bin_dir.join("bun").exists(),
        RuntimeType::Uv => bin_dir.join("uv").exists(),
    }
}

/// Get the path separator for the current platform
fn get_path_separator() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

/// Prepare environment variables for a command based on runtime type and version
pub fn prepare_environment(
    command: &mut TokioCommand,
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<()> {
    let bin_dir = get_runtime_bin_dir(runtime_type, version)?;
    let cache_dir = get_cache_dir(runtime_type)?;

    // Set PATH to include the runtime binary directory
    let old_path = env::var("PATH").unwrap_or_default();
    let path_separator = get_path_separator();
    let new_path = format!("{}{}{}", bin_dir.display(), path_separator, old_path);
    command.env("PATH", new_path);

    // Set runtime-specific environment variables
    match runtime_type {
        RuntimeType::Node => {
            command.env("NPM_CONFIG_CACHE", cache_dir);
        }
        RuntimeType::Bun => {
            command.env("BUN_INSTALL_CACHE_DIR", cache_dir);
        }
        RuntimeType::Uv => {
            // uv 环境变量控制，让 uv 自动管理 Python
            let mcpmate_dir = get_mcpmate_dir()?;
            let uv_cache_dir = cache_dir;
            let uv_python_cache_dir = mcpmate_dir.join("cache").join("uv");
            let uv_python_install_dir = mcpmate_dir.join("runtimes").join("uv");

            // 确保目录存在
            std::fs::create_dir_all(&uv_python_cache_dir)?;
            std::fs::create_dir_all(&uv_python_install_dir)?;

            // 设置 uv 环境变量
            command.env("UV_CACHE_DIR", uv_cache_dir);
            command.env("UV_PYTHON_CACHE_DIR", uv_python_cache_dir);
            command.env("UV_PYTHON_INSTALL_DIR", uv_python_install_dir);

            tracing::debug!(
                "Set uv environment variables: UV_CACHE_DIR={}, UV_PYTHON_CACHE_DIR={}, UV_PYTHON_INSTALL_DIR={}",
                uv_cache_dir.display(),
                uv_python_cache_dir.display(),
                uv_python_install_dir.display(),
            );
        }
    }

    // Log environment variables for debugging
    tracing::debug!(
        "Prepared environment for {:?} {}: PATH includes {}, cache at {}",
        runtime_type,
        version.unwrap_or("latest"),
        bin_dir.display(),
        cache_dir.display()
    );

    Ok(())
}

/// Get the current platform identifier
pub fn get_platform() -> String {
    if cfg!(windows) {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macos".to_string()
    } else {
        "linux".to_string()
    }
}

/// Get the current architecture identifier
pub fn get_architecture() -> String {
    if cfg!(target_arch = "x86_64") {
        "x86_64".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "aarch64".to_string()
    } else {
        "unknown".to_string()
    }
}
