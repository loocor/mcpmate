//! Unified environment variable management for MCPMate
//!
//! This module provides centralized environment variable management,
//! eliminating duplication across runtime and conf modules.

use anyhow::Result;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use tokio::process::Command;

use super::{paths::global_paths, types::RuntimeType};

// Re-export constants from the central constants module
pub use super::constants::env_vars as constants;
pub use super::constants::separators::get_path_separator;

const AMBIENT_PROXY_ENV_VARS: &[&str] = &[
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
    "no_proxy",
];

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
        other => Err(anyhow::anyhow!("Unsupported system architecture: {}", other)),
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
            set_command_env_if_absent(command, key, value);
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

fn command_has_explicit_env(
    command: &Command,
    key: &str,
) -> bool {
    let key = OsStr::new(key);
    command.as_std().get_envs().any(|(name, _)| name == key)
}

fn set_command_env_if_absent(
    command: &mut Command,
    key: &str,
    value: impl AsRef<OsStr>,
) {
    if command_has_explicit_env(command, key) {
        tracing::debug!("Preserving explicitly configured environment variable: {}", key);
        return;
    }

    command.env(key, value);
}

/// Remove inherited proxy settings from child process environments.
///
/// Server-level env values remain authoritative. This prevents desktop or
/// shell proxy settings from silently changing stdio server behavior.
pub fn sanitize_ambient_network_environment(command: &mut Command) {
    for key in AMBIENT_PROXY_ENV_VARS {
        if command_has_explicit_env(command, key) {
            tracing::debug!("Preserving explicitly configured proxy environment variable: {}", key);
            continue;
        }

        command.env_remove(key);
        tracing::debug!(
            "Removed inherited proxy environment variable from child process: {}",
            key
        );
    }
}

/// Apply MCPMate-owned cache directories to any child process.
///
/// Stdio servers can cross-call package managers internally, so cache
/// directories are process baselines rather than entry-runtime settings.
pub fn apply_default_runtime_cache_environment(command: &mut Command) -> Result<()> {
    let paths = global_paths();
    let cache_vars = [
        (
            constants::UV_CACHE_DIR,
            paths.runtime_cache_dir(RuntimeType::Uv.as_str()),
        ),
        (
            constants::BUN_INSTALL_CACHE_DIR,
            paths.runtime_cache_dir(RuntimeType::Bun.as_str()),
        ),
        (
            constants::NPM_CONFIG_CACHE,
            paths.runtime_cache_dir(RuntimeType::Node.as_str()),
        ),
    ];

    for (key, cache_dir) in cache_vars {
        std::fs::create_dir_all(&cache_dir)?;
        set_command_env_if_absent(command, key, cache_dir.as_os_str());
        tracing::debug!(
            "Prepared default runtime cache environment {}={}",
            key,
            cache_dir.display()
        );
    }

    Ok(())
}

/// Create runtime-specific environment for uv
pub fn create_uv_environment(bin_path: &Path) -> Result<EnvironmentManager> {
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
        "Created uv environment: PATH includes {}, cache at {}",
        bin_dir.display(),
        cache_dir.display()
    );

    Ok(env)
}

/// Create runtime-specific environment for Bun
pub fn create_bun_environment(bin_path: &Path) -> Result<EnvironmentManager> {
    let paths = global_paths();
    let mut env = EnvironmentManager::new();

    // Add runtime bin directory to PATH
    let bin_dir = bin_path.parent().unwrap_or(bin_path);
    env.prepend_path(bin_dir);

    // Set Bun specific environment variables
    let cache_dir = paths.runtime_cache_dir(RuntimeType::Bun.as_str());

    // Ensure cache directory exists
    std::fs::create_dir_all(&cache_dir)?;

    env.set_var(constants::BUN_INSTALL_CACHE_DIR, cache_dir.to_string_lossy());

    // Set runtime bin path for reference
    env.set_var(constants::MCP_RUNTIME_BIN, bin_path.to_string_lossy());

    // Set specific tool paths
    let bunx_path = bin_dir.join(if cfg!(windows) { "bunx.exe" } else { "bunx" });
    if bunx_path.exists() {
        env.set_var(constants::BUNX_BIN_PATH, bunx_path.to_string_lossy());
    }

    tracing::debug!(
        "Created Bun environment: PATH includes {}, cache at {}",
        bin_dir.display(),
        cache_dir.display()
    );

    Ok(env)
}

/// Create runtime-specific environment for Node.js
pub fn create_node_environment(bin_path: &Path) -> Result<EnvironmentManager> {
    let paths = global_paths();
    let mut env = EnvironmentManager::new();

    let bin_dir = bin_path.parent().unwrap_or(bin_path);
    env.prepend_path(bin_dir);

    let cache_dir = paths.runtime_cache_dir(RuntimeType::Node.as_str());
    std::fs::create_dir_all(&cache_dir)?;

    env.set_var(constants::NPM_CONFIG_CACHE, cache_dir.to_string_lossy());
    env.set_var(constants::MCP_RUNTIME_BIN, bin_path.to_string_lossy());

    tracing::debug!(
        "Created Node.js environment: PATH includes {}, cache at {}",
        bin_dir.display(),
        cache_dir.display()
    );

    Ok(env)
}

/// Create environment for a specific runtime type
pub fn create_runtime_environment(
    runtime_type: &str,
    bin_path: &Path,
) -> Result<EnvironmentManager> {
    use super::types::RuntimeType;
    use std::str::FromStr;

    if let Ok(rt) = RuntimeType::from_str(runtime_type) {
        match rt {
            RuntimeType::Uv => create_uv_environment(bin_path),
            RuntimeType::Bun => create_bun_environment(bin_path),
            RuntimeType::Node => create_node_environment(bin_path),
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
) -> Result<()> {
    apply_default_runtime_cache_environment(command)?;
    let env = create_runtime_environment(runtime_type, bin_path)?;
    env.apply_to_command(command);
    Ok(())
}

// -----------------------------------------------------------------------------
// Origin allowlist (shared by API and /mcp)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AllowedOrigins {
    entries: Vec<String>,
}

static GLOBAL_ALLOWED_ORIGINS: OnceCell<AllowedOrigins> = OnceCell::new();

impl AllowedOrigins {
    fn load_from_env() -> Self {
        let mut entries: Vec<String> = vec![
            // no-origin requests (curl, server-to-server)
            "null".into(),
            // loopback without port
            "http://localhost".into(),
            "https://localhost".into(),
            "http://127.0.0.1".into(),
            "https://127.0.0.1".into(),
            "http://[::1]".into(),
            "https://[::1]".into(),
            // loopback with any port (for Swagger UI / apidocs on 8080, etc.)
            "http://localhost:*".into(),
            "https://localhost:*".into(),
            "http://127.0.0.1:*".into(),
            "https://127.0.0.1:*".into(),
            "http://[::1]:*".into(),
            "https://[::1]:*".into(),
            // embedded desktop shell
            "tauri://localhost".into(),
            "http://tauri.localhost".into(),
        ];
        if let Ok(raw) = std::env::var(constants::MCPMATE_ALLOWED_ORIGINS) {
            for part in raw.split(',') {
                let s = part.trim().to_ascii_lowercase();
                if !s.is_empty() {
                    entries.push(s);
                }
            }
        }
        let mut seen = HashSet::new();
        entries.retain(|e| seen.insert(e.clone()));
        Self { entries }
    }

    pub fn global() -> &'static Self {
        GLOBAL_ALLOWED_ORIGINS.get_or_init(Self::load_from_env)
    }

    pub fn is_allowed(
        &self,
        origin: &str,
    ) -> bool {
        let o = origin.trim().to_ascii_lowercase();
        for e in &self.entries {
            if let Some(prefix) = e.strip_suffix('*') {
                if o.starts_with(prefix) {
                    return true;
                }
            } else if o == *e {
                return true;
            }
        }
        false
    }
}

/// Check if an origin is allowed using the global configuration.
pub fn is_allowed_origin(origin: &str) -> bool {
    AllowedOrigins::global().is_allowed(origin)
}

/// Axum middleware that enforces origin allowlist if `Origin` header is present.
pub async fn origin_guard_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::{
        http::{HeaderValue, Method, StatusCode, header},
        response::IntoResponse,
    };

    let method = req.method().clone();
    let origin_opt = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    if let Some(origin) = origin_opt.as_ref() {
        if !is_allowed_origin(origin) {
            tracing::warn!(origin = %origin, path = %req.uri(), "API request rejected: disallowed Origin");
            let body = axum::Json(serde_json::json!({
                "error": {"message": format!("Disallowed Origin: {}", origin), "status": 403}
            }));
            return (StatusCode::FORBIDDEN, body).into_response();
        }
    }

    // Handle CORS preflight requests explicitly
    if method == Method::OPTIONS {
        let mut response = StatusCode::NO_CONTENT.into_response();
        if let Some(origin) = origin_opt.as_ref() {
            if let Ok(value) = HeaderValue::from_str(origin) {
                response
                    .headers_mut()
                    .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
            }
        }
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET,POST,PUT,PATCH,DELETE,OPTIONS"),
        );
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("Authorization,Content-Type"),
        );
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
            HeaderValue::from_static("true"),
        );
        response
            .headers_mut()
            .insert(header::VARY, HeaderValue::from_static("Origin"));
        return response;
    }

    let mut response = next.run(req).await;

    if let Some(origin) = origin_opt.as_ref() {
        if let Ok(value) = HeaderValue::from_str(origin) {
            response
                .headers_mut()
                .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
            response.headers_mut().insert(
                header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            );
            response
                .headers_mut()
                .insert(header::VARY, HeaderValue::from_static("Origin"));
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::{AllowedOrigins, EnvironmentManager, constants, sanitize_ambient_network_environment};
    use std::ffi::OsStr;
    use tokio::process::Command;

    fn command_env_value(
        command: &Command,
        key: &str,
    ) -> Option<String> {
        let key = OsStr::new(key);
        command
            .as_std()
            .get_envs()
            .find_map(|(name, value)| (name == key).then_some(value).flatten())
            .map(|value| value.to_string_lossy().to_string())
    }

    fn command_env_setting(
        command: &Command,
        key: &str,
    ) -> Option<Option<String>> {
        let key = OsStr::new(key);
        command
            .as_std()
            .get_envs()
            .find_map(|(name, value)| (name == key).then(|| value.map(|value| value.to_string_lossy().to_string())))
    }

    #[test]
    fn default_allowed_origins_include_desktop_shell_origins() {
        let origins = AllowedOrigins::load_from_env();

        assert!(origins.is_allowed("tauri://localhost"));
        assert!(origins.is_allowed("http://tauri.localhost"));
    }

    #[test]
    fn default_allowed_origins_reject_external_origins() {
        let origins = AllowedOrigins::load_from_env();

        assert!(!origins.is_allowed("http://rejected-origin.invalid"));
    }

    #[test]
    fn environment_manager_preserves_explicit_command_env() {
        let mut command = Command::new("echo");
        command.env(constants::NPM_CONFIG_CACHE, "/custom/npm-cache");

        let mut env = EnvironmentManager::new();
        env.set_var(constants::NPM_CONFIG_CACHE, "/mcpmate/npm-cache");
        env.apply_to_command(&mut command);

        assert_eq!(
            command_env_value(&command, constants::NPM_CONFIG_CACHE),
            Some("/custom/npm-cache".to_string())
        );
    }

    #[test]
    fn environment_manager_applies_missing_command_env() {
        let mut command = Command::new("echo");

        let mut env = EnvironmentManager::new();
        env.set_var(constants::UV_CACHE_DIR, "/mcpmate/uv-cache");
        env.apply_to_command(&mut command);

        assert_eq!(
            command_env_value(&command, constants::UV_CACHE_DIR),
            Some("/mcpmate/uv-cache".to_string())
        );
    }

    #[test]
    fn ambient_network_sanitizer_removes_inherited_proxy_env() {
        let mut command = Command::new("echo");

        sanitize_ambient_network_environment(&mut command);

        assert_eq!(command_env_setting(&command, "ALL_PROXY"), Some(None));
        assert_eq!(command_env_setting(&command, "all_proxy"), Some(None));
    }

    #[test]
    fn ambient_network_sanitizer_preserves_explicit_proxy_env() {
        let mut command = Command::new("echo");
        command.env("ALL_PROXY", "socks5://127.0.0.1:1080");

        sanitize_ambient_network_environment(&mut command);

        assert_eq!(
            command_env_value(&command, "ALL_PROXY"),
            Some("socks5://127.0.0.1:1080".to_string())
        );
    }
}
