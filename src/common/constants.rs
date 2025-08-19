//! Common constants for MCPMate
//!
//! This module contains constants used throughout MCPMate,
//! including runtime commands, environment variables, and path separators.

/// Runtime command constants
pub mod commands {
    /// UV package executor command
    pub const UVX: &str = "uvx";

    /// Bun package executor command
    pub const BUNX: &str = "bunx";

    /// Docker command
    pub const DOCKER: &str = "docker";

    /// NPX command, only used for npx compatibility
    pub const NPX: &str = "npx";
}

/// Environment variable name constants
pub mod env_vars {
    /// Generic runtime binary path
    pub const MCP_RUNTIME_BIN: &str = "MCP_RUNTIME_BIN";

    /// UV specific binary path
    pub const UVX_BIN_PATH: &str = "UVX_BIN_PATH";

    /// Bun specific binary path
    pub const BUNX_BIN_PATH: &str = "BUNX_BIN_PATH";

    /// UV cache directory
    pub const UV_CACHE_DIR: &str = "UV_CACHE_DIR";

    /// Bun cache directory
    pub const BUN_INSTALL_CACHE_DIR: &str = "BUN_INSTALL_CACHE_DIR";

    /// System PATH environment variable
    pub const PATH: &str = "PATH";
}

/// Path separator constants
pub mod separators {
    /// Windows path separator
    pub const WINDOWS: &str = ";";

    /// Unix-like path separator
    pub const UNIX: &str = ":";

    /// Get platform-specific path separator
    pub fn get_path_separator() -> &'static str {
        if cfg!(windows) { WINDOWS } else { UNIX }
    }
}

/// Common timeout constants
pub mod timeouts {
    /// Timeout for acquiring async locks in milliseconds (e.g., connection pool locks)
    pub const LOCK_MS: u64 = 500;

    /// Timeout for connection pool operations when disabling in seconds
    pub const POOL_DISABLE_SEC: u64 = 1;
}

/// Common strategy labels for inspect responses
pub mod strategies {
    /// Data served from cache (fresh or offline)
    pub const CACHE: &str = "cache";

    /// Data aggregated at runtime from live instances
    pub const RUNTIME: &str = "runtime";

    /// No data available from any strategy
    pub const NONE: &str = "none";
}
