//! Runtime executable path utilities
//!
//! This module provides runtime-specific executable path resolution and installation checks.
//! Basic path management is handled by common/paths.rs.

use crate::common::paths::global_paths;
use crate::runtime::types::RuntimeType;
use std::path::PathBuf;

/// Get the executable path of the specific runtime version
pub fn get_runtime_executable_path(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<PathBuf, anyhow::Error> {
    let version = version.unwrap_or_else(|| runtime_type.default_version());
    let paths = global_paths();
    let version_dir = paths.runtime_version_dir(runtime_type.as_str(), version);
    let bin_dir = version_dir.join("bin");

    // For Node.js and Uv, prioritize the execution wrappers over base commands
    let exe_name = match runtime_type {
        RuntimeType::Node => {
            // Check for npx first (preferred for MCP servers)
            if cfg!(windows) {
                // On Windows, check for both .exe and .cmd versions
                let npx_exe = bin_dir.join("npx.exe");
                if npx_exe.exists() {
                    return Ok(npx_exe);
                }
                let npx_cmd = bin_dir.join("npx.cmd");
                if npx_cmd.exists() {
                    return Ok(npx_cmd);
                }
            } else {
                let npx_path = bin_dir.join("npx");
                if npx_path.exists() {
                    return Ok(npx_path);
                }
            }
            // Fall back to node if npx doesn't exist
            runtime_type.executable_name()
        }
        RuntimeType::Uv => {
            // Check for uvx first (preferred for MCP servers)
            let uvx_name = if cfg!(windows) { "uvx.exe" } else { "uvx" };
            let uvx_path = bin_dir.join(uvx_name);
            if uvx_path.exists() {
                return Ok(uvx_path);
            }
            // Fall back to uv if uvx doesn't exist
            runtime_type.executable_name()
        }
        RuntimeType::Bun => {
            // Check for bunx first (preferred for MCP servers)
            let bunx_name = if cfg!(windows) { "bunx.exe" } else { "bunx" };
            let bunx_path = bin_dir.join(bunx_name);
            if bunx_path.exists() {
                return Ok(bunx_path);
            }
            // Fall back to bun if bunx doesn't exist
            runtime_type.executable_name()
        }
    };

    // check the bin directory first
    let bin_exe_path = bin_dir.join(&exe_name);
    if bin_exe_path.exists() {
        return Ok(bin_exe_path);
    }

    // then check the specific sub directory
    match runtime_type {
        RuntimeType::Node => {
            // check the decompressed Node.js directory structure (e.g. node-v14.17.0-darwin-x64)
            if let Ok(entries) = std::fs::read_dir(&version_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir()
                        && path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with("node-")
                    {
                        // For Node.js, prioritize npx over node in subdirectories too
                        let npx_name = if cfg!(windows) { "npx.exe" } else { "npx" };
                        let npx_path = path.join("bin").join(npx_name);
                        if npx_path.exists() {
                            return Ok(npx_path);
                        }

                        // check the bin sub directory first (macOS/Linux)
                        let node_version_bin_path = path.join("bin").join(&exe_name);
                        if node_version_bin_path.exists() {
                            return Ok(node_version_bin_path);
                        }

                        // then check the root directory (Windows)
                        let node_version_root_path = path.join(&exe_name);
                        if node_version_root_path.exists() {
                            return Ok(node_version_root_path);
                        }

                        // For Windows, also check for npx in the root directory
                        if cfg!(windows) {
                            let npx_root_path = path.join("npx.exe");
                            if npx_root_path.exists() {
                                return Ok(npx_root_path);
                            }
                        }
                    }
                }
            }
        }
        RuntimeType::Bun => {
            // check the root directory
            let root_exe_path = version_dir.join(&exe_name);
            if root_exe_path.exists() {
                return Ok(root_exe_path);
            }

            // check the specific platform directory
            let possible_dirs = [
                version_dir.join("bun-darwin-x64"),
                version_dir.join("bun-darwin-aarch64"),
                version_dir.join("bun-linux-x64"),
                version_dir.join("bun-linux-aarch64"),
                version_dir.join("bun-win-x64"),
            ];

            for dir in possible_dirs.iter() {
                if dir.exists() {
                    let dir_exe_path = dir.join(&exe_name);
                    if dir_exe_path.exists() {
                        return Ok(dir_exe_path);
                    }
                }
            }
        }
        RuntimeType::Uv => {
            // check the root directory
            let root_exe_path = version_dir.join(&exe_name);
            if root_exe_path.exists() {
                return Ok(root_exe_path);
            }

            // Check for platform-specific subdirectory (e.g., uv-aarch64-apple-darwin)
            // Look through any directory that starts with "uv-"
            if let Ok(entries) = std::fs::read_dir(&version_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir()
                        && path
                            .file_name()
                            .map(|name| name.to_string_lossy().starts_with("uv-"))
                            .unwrap_or(false)
                    {
                        // Check for uv executable
                        let uv_path = path.join(&exe_name);
                        if uv_path.exists() {
                            return Ok(uv_path);
                        }

                        // Also check for uvx executable
                        let uvx_name = if cfg!(windows) { "uvx.exe" } else { "uvx" };
                        let uvx_path = path.join(uvx_name);
                        if uvx_path.exists() {
                            return Ok(uvx_path);
                        }
                    }
                }
            }
        }
    }

    // return the executable path in the bin directory (even if it doesn't exist)
    Ok(bin_exe_path)
}

/// Check if the runtime is installed
pub fn is_runtime_installed(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<bool, anyhow::Error> {
    let version = version.unwrap_or_else(|| runtime_type.default_version());
    let paths = global_paths();
    let version_dir = paths.runtime_version_dir(runtime_type.as_str(), version);

    // check if the executable file exists
    let bin_dir = version_dir.join("bin");
    let exe_name = runtime_type.executable_name();
    let bin_exe_path = bin_dir.join(&exe_name);

    if bin_exe_path.exists() {
        return Ok(true);
    }

    // check the specific sub directory
    match runtime_type {
        RuntimeType::Node => {
            // check the standard Node.js directory structure
            let node_bin_path = version_dir.join("node").join("bin").join(&exe_name);
            if node_bin_path.exists() {
                return Ok(true);
            }

            // check the decompressed Node.js directory structure (e.g. node-v14.17.0-darwin-x64)
            if let Ok(entries) = std::fs::read_dir(&version_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir()
                        && path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with("node-")
                    {
                        let node_version_bin_path = path.join("bin").join(&exe_name);
                        if node_version_bin_path.exists() {
                            return Ok(true);
                        }

                        // then check the root directory (Windows)
                        let node_version_root_path = path.join(&exe_name);
                        if node_version_root_path.exists() {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        RuntimeType::Bun => {
            // check the root directory
            let root_exe_path = version_dir.join(&exe_name);
            if root_exe_path.exists() {
                return Ok(true);
            }

            // check the specific platform directory
            let possible_dirs = [
                version_dir.join("bun-darwin-x64"),
                version_dir.join("bun-darwin-aarch64"),
                version_dir.join("bun-linux-x64"),
                version_dir.join("bun-linux-aarch64"),
                version_dir.join("bun-win-x64"),
            ];

            for dir in possible_dirs.iter() {
                if dir.exists() {
                    let dir_exe_path = dir.join(&exe_name);
                    if dir_exe_path.exists() {
                        return Ok(true);
                    }
                }
            }
        }
        RuntimeType::Uv => {
            // check the root directory
            let root_exe_path = version_dir.join(&exe_name);
            if root_exe_path.exists() {
                return Ok(true);
            }

            // Check for platform-specific subdirectory (e.g., uv-aarch64-apple-darwin)
            // Look through any directory that starts with "uv-"
            if let Ok(entries) = std::fs::read_dir(&version_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir()
                        && path
                            .file_name()
                            .map(|name| name.to_string_lossy().starts_with("uv-"))
                            .unwrap_or(false)
                    {
                        // Check for uv executable
                        let uv_path = path.join(&exe_name);
                        if uv_path.exists() {
                            return Ok(true);
                        }

                        // Also check for uvx executable
                        let uvx_name = if cfg!(windows) { "uvx.exe" } else { "uvx" };
                        let uvx_path = path.join(uvx_name);
                        if uvx_path.exists() {
                            return Ok(true);
                        }
                    }
                }
            }
        }
    }

    Ok(false)
}
