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