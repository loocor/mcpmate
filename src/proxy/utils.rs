use std::env;
use tokio::process::Command;

/// Prepare command environment variables for different command types
pub fn prepare_command_env(command: &mut Command, command_str: &str) {
    // 1. bin path
    let bin_var = match command_str {
        "npx" => "NPX_BIN_PATH",
        "uvx" => "UVX_BIN_PATH",
        "docker" => "DOCKER_BIN_PATH",
        _ => "MCP_RUNTIME_BIN",
    };
    let bin_path = env::var(bin_var)
        .or_else(|_| env::var("MCP_RUNTIME_BIN"))
        .ok();
    if let Some(bin_path) = bin_path {
        let old_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_path, old_path);
        command.env("PATH", new_path);
    }

    // 2. cache env
    let cache_var = match command_str {
        "npx" => "NPM_CONFIG_CACHE",
        "uvx" => "UV_CACHE_DIR",
        _ => "",
    };
    if !cache_var.is_empty() {
        if let Ok(cache_val) = env::var(cache_var) {
            command.env(cache_var, cache_val);
        }
    }

    // 3. Docker specific settings
    if command_str == "docker" {
        // Set DOCKER_HOST if available
        if let Ok(docker_host) = env::var("DOCKER_HOST") {
            command.env("DOCKER_HOST", docker_host);
        }
    }
}

/// Determine appropriate connection timeout based on command type
pub fn get_connection_timeout(command: &str) -> std::time::Duration {
    match command {
        "docker" => std::time::Duration::from_secs(120), // Docker operations can take longer
        "npx" => std::time::Duration::from_secs(60),     // npm operations can take time
        _ => std::time::Duration::from_secs(30),         // Default timeout
    }
}

/// Determine appropriate tools listing timeout based on command type
pub fn get_tools_timeout(command: &str) -> std::time::Duration {
    match command {
        "docker" => std::time::Duration::from_secs(60), // Docker operations can take longer
        "npx" => std::time::Duration::from_secs(30),    // npm operations can take time
        _ => std::time::Duration::from_secs(20),        // Default timeout
    }
}
