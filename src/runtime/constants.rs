use crate::runtime::types::RuntimeType;
use std::path::PathBuf;

// base directory structure
pub const MCPMATE_DIR_NAME: &str = ".mcpmate";
pub const RUNTIMES_DIR_NAME: &str = "runtimes";
pub const CACHE_DIR_NAME: &str = "cache";
pub const TMP_DIR_NAME: &str = "tmp";
pub const DOWNLOADS_DIR_NAME: &str = "downloads";
pub const BIN_DIR_NAME: &str = "bin";

// runtime specific directory specific directory
pub const NODE_DIR_NAME: &str = "node";
pub const BUN_DIR_NAME: &str = "bun";
pub const UV_DIR_NAME: &str = "uv";

// default version
pub const DEFAULT_NODE_VERSION: &str = "latest";
pub const DEFAULT_BUN_VERSION: &str = "latest";
pub const DEFAULT_UV_VERSION: &str = "latest";

/// get the default version of the runtime
pub fn get_default_version(runtime_type: RuntimeType) -> &'static str {
    match runtime_type {
        RuntimeType::Node => DEFAULT_NODE_VERSION,
        RuntimeType::Bun => DEFAULT_BUN_VERSION,
        RuntimeType::Uv => DEFAULT_UV_VERSION,
    }
}

/// get the directory name of the runtime
pub fn get_runtime_dir_name(runtime_type: RuntimeType) -> &'static str {
    match runtime_type {
        RuntimeType::Node => "node",
        RuntimeType::Bun => "bun",
        RuntimeType::Uv => "uv",
    }
}

/// get the executable name (platform dependent)
pub fn get_executable_name(runtime_type: RuntimeType) -> &'static str {
    match runtime_type {
        RuntimeType::Node => {
            if cfg!(windows) {
                "node.exe"
            } else {
                "node"
            }
        }
        RuntimeType::Bun => {
            if cfg!(windows) {
                "bun.exe"
            } else {
                "bun"
            }
        }
        RuntimeType::Uv => {
            if cfg!(windows) {
                "uv.exe"
            } else {
                "uv"
            }
        }
    }
}

/// get the MCPMate user directory
pub fn get_mcpmate_dir() -> Result<PathBuf, anyhow::Error> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot get user home directory"))?;
    Ok(home_dir.join(MCPMATE_DIR_NAME))
}

/// get the runtime root directory
pub fn get_runtimes_base_dir() -> Result<PathBuf, anyhow::Error> {
    Ok(get_mcpmate_dir()?.join(RUNTIMES_DIR_NAME))
}

/// get the directory of the specific runtime
pub fn get_runtime_type_dir(runtime_type: RuntimeType) -> Result<PathBuf, anyhow::Error> {
    Ok(get_runtimes_base_dir()?.join(get_runtime_dir_name(runtime_type)))
}

/// get the directory of the specific runtime version
pub fn get_runtime_version_dir(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<PathBuf, anyhow::Error> {
    let version = version.unwrap_or_else(|| get_default_version(runtime_type));
    Ok(get_runtime_type_dir(runtime_type)?.join(version))
}

/// get the bin directory of the specific runtime version
pub fn get_runtime_bin_dir(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<PathBuf, anyhow::Error> {
    Ok(get_runtime_version_dir(runtime_type, version)?.join(BIN_DIR_NAME))
}

/// get the executable path of the specific runtime version
pub fn get_runtime_executable_path(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<PathBuf, anyhow::Error> {
    let version_dir = get_runtime_version_dir(runtime_type, version)?;
    let bin_dir = version_dir.join(BIN_DIR_NAME);
    let exe_name = get_executable_name(runtime_type);

    // check the bin directory first
    let bin_exe_path = bin_dir.join(exe_name);
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
                        // check the bin sub directory first (macOS/Linux)
                        let node_version_bin_path = path.join("bin").join(exe_name);
                        if node_version_bin_path.exists() {
                            return Ok(node_version_bin_path);
                        }

                        // then check the root directory (Windows)
                        let node_version_root_path = path.join(exe_name);
                        if node_version_root_path.exists() {
                            return Ok(node_version_root_path);
                        }
                    }
                }
            }
        }
        RuntimeType::Bun => {
            // check the root directory
            let root_exe_path = version_dir.join(exe_name);
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
                    let dir_exe_path = dir.join(exe_name);
                    if dir_exe_path.exists() {
                        return Ok(dir_exe_path);
                    }
                }
            }
        }
        RuntimeType::Uv => {
            // check the root directory
            let root_exe_path = version_dir.join(exe_name);
            if root_exe_path.exists() {
                return Ok(root_exe_path);
            }
        }
    }

    // return the executable path in the bin directory (even if it doesn't exist)
    Ok(bin_exe_path)
}

/// get the cache directory
pub fn get_cache_dir(runtime_type: RuntimeType) -> Result<PathBuf, anyhow::Error> {
    Ok(get_mcpmate_dir()?
        .join(CACHE_DIR_NAME)
        .join(get_runtime_dir_name(runtime_type)))
}

/// get the temporary download directory
pub fn get_temp_download_dir() -> Result<PathBuf, anyhow::Error> {
    Ok(get_mcpmate_dir()?
        .join(TMP_DIR_NAME)
        .join(DOWNLOADS_DIR_NAME))
}

/// check if the runtime is installed
pub fn is_runtime_installed(
    runtime_type: RuntimeType,
    version: Option<&str>,
) -> Result<bool, anyhow::Error> {
    let version_dir = get_runtime_version_dir(runtime_type, version)?;

    // check if the executable file exists
    let bin_dir = version_dir.join(BIN_DIR_NAME);
    let exe_name = get_executable_name(runtime_type);
    let bin_exe_path = bin_dir.join(exe_name);

    if bin_exe_path.exists() {
        return Ok(true);
    }

    // check the specific sub directory
    match runtime_type {
        RuntimeType::Node => {
            // check the standard Node.js directory structure
            let node_bin_path = version_dir.join(NODE_DIR_NAME).join("bin").join(exe_name);
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
                        let node_version_bin_path = path.join("bin").join(exe_name);
                        if node_version_bin_path.exists() {
                            return Ok(true);
                        }

                        // then check the root directory (Windows)
                        let node_version_root_path = path.join(exe_name);
                        if node_version_root_path.exists() {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        RuntimeType::Bun => {
            // check the root directory
            let root_exe_path = version_dir.join(exe_name);
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
                    let dir_exe_path = dir.join(exe_name);
                    if dir_exe_path.exists() {
                        return Ok(true);
                    }
                }
            }
        }
        RuntimeType::Uv => {
            // check the root directory
            let root_exe_path = version_dir.join(exe_name);
            if root_exe_path.exists() {
                return Ok(true);
            }
        }
    }

    Ok(false)
}
