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

/// Transport type constants for unified transport handling
pub mod transport {
    pub const STDIO: &str = "stdio";
    pub const SSE: &str = "sse";
    pub const STREAMABLE_HTTP: &str = "streamable_http";
}

/// Database constants for unified database operations
pub mod database {
    /// Database table name constants
    pub mod tables {
        pub const CLIENT_CONFIG_RULES: &str = "client_config_rules";
        pub const CONFIG_SUIT_PROMPT: &str = "config_suit_prompt";
        pub const CONFIG_SUIT_RESOURCE: &str = "config_suit_resource";
        pub const CONFIG_SUIT_SERVER: &str = "config_suit_server";
        pub const CONFIG_SUIT_TOOL: &str = "config_suit_tool";
        pub const CONFIG_SUIT: &str = "config_suit";
        pub const SERVER_ARGS: &str = "server_args";
        pub const SERVER_CONFIG: &str = "server_config";
        pub const SERVER_ENV: &str = "server_env";
        pub const SERVER_META: &str = "server_meta";
        pub const SERVER_PROMPTS: &str = "server_prompts";
        pub const SERVER_RESOURCE_TEMPLATES: &str = "server_resource_templates";
        pub const SERVER_RESOURCES: &str = "server_resources";
        pub const SERVER_TOOLS: &str = "server_tools";
    }

    /// Database column name constants
    pub mod columns {
        pub const ID: &str = "id";
        pub const NAME: &str = "name";
        pub const SERVER_ID: &str = "server_id";
        pub const ENABLED: &str = "enabled";
        pub const CREATED_AT: &str = "created_at";
        pub const UPDATED_AT: &str = "updated_at";
        pub const SERVER_TYPE: &str = "server_type";
        pub const UNIQUE_NAME: &str = "unique_name";
        pub const TOOL_NAME: &str = "tool_name";
        pub const SERVER_NAME: &str = "server_name";
        pub const COMMAND: &str = "command";
        pub const URL: &str = "url";
        pub const TRANSPORT_TYPE: &str = "transport_type";
        pub const CAPABILITIES: &str = "capabilities";
        pub const DESCRIPTION: &str = "description";
        pub const ARGS: &str = "args";
        pub const ENV: &str = "env";
        pub const VALUE: &str = "value";
        pub const SUIT_ID: &str = "suit_id";
    }
}

/// Error message constants for unified error handling
pub mod errors {
    pub const RESOURCE_EXISTS: &str = "Resource already exists";
    pub const RESOURCE_NOT_FOUND: &str = "Resource not found";
    pub const FK_VIOLATION: &str = "Foreign key constraint violation";
    pub const CHECK_VIOLATION: &str = "Check constraint violation";
    pub const DB_TIMEOUT: &str = "Database connection timeout";

    // Error message templates
    pub const CREATE_TABLE_FAILED: &str = "Failed to create {} table: {}";
    pub const CREATE_INDEX_FAILED: &str = "Failed to create index on {}: {}";
    pub const TABLE_CREATED: &str = "{} table created or already exists";
}

/// Configuration key constants for unified config management
pub mod config_keys {
    pub const MCP_SERVERS: &str = "mcpServers";
    pub const MCP_SERVERS_SNAKE: &str = "mcp_servers";
    pub const CONTEXT_SERVERS: &str = "context_servers";
    pub const TOP_LEVEL_KEY: &str = "top_level_key";
    pub const IDENTIFIER: &str = "identifier";
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
