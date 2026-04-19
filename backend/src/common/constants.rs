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

    /// Origin allowlist for API and /mcp (comma-separated; supports trailing '*')
    pub const MCPMATE_ALLOWED_ORIGINS: &str = "MCPMATE_ALLOWED_ORIGINS";

    /// Toggle file logging for backend ("1/true/on/yes" to enable, "0/false/off/no" to disable).
    /// TODO(temporary): remove this env after audit logging subsystem lands.
    pub const MCPMATE_LOG_TO_FILE: &str = "MCPMATE_LOG_TO_FILE";
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

/// Port configuration constants
pub mod ports {
    /// API server port (standard)
    pub const API_PORT: u16 = 8080;
    /// MCP proxy server port (standard)
    pub const MCP_PORT: u16 = 8000;
}

/// Common timeout constants
pub mod timeouts {
    /// Timeout for acquiring async locks in milliseconds (e.g., connection pool locks)
    pub const LOCK_MS: u64 = 500;

    /// Timeout for connection pool operations when disabling in seconds
    pub const POOL_DISABLE_SEC: u64 = 1;

    /// Database connection timeout in milliseconds
    pub const DB_CONNECTION_TIMEOUT_MS: u64 = 5000;

    /// Server startup timeout in seconds
    pub const SERVER_STARTUP_TIMEOUT_SEC: u64 = 30;

    /// Template reload coalescing TTL for /api/client/** middleware in seconds
    /// Requests within this window reuse the previous reload to reduce I/O,
    /// while keeping near-real-time freshness for desktop usage.
    pub const TEMPLATE_RELOAD_TTL_SEC: u64 = 2;

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
        pub const PROFILE_PROMPT: &str = "profile_prompt";
        pub const PROFILE_RESOURCE: &str = "profile_resource";
        pub const PROFILE_RESOURCE_TEMPLATE: &str = "profile_resource_template";
        pub const PROFILE_SERVER: &str = "profile_server";
        pub const PROFILE_TOOL: &str = "profile_tool";
        pub const PROFILE: &str = "profile";
        pub const CLIENT: &str = "client";
        pub const CLIENT_TEMPLATE_RUNTIME: &str = "client_template_runtime";
        pub const SYSTEM_SETTINGS: &str = "system_settings";
        pub const SERVER_ARGS: &str = "server_args";
        pub const SERVER_CONFIG: &str = "server_config";
        pub const SERVER_ENV: &str = "server_env";
        pub const SERVER_HEADERS: &str = "server_headers";
        pub const SERVER_META: &str = "server_meta";
        pub const SERVER_OAUTH_CONFIG: &str = "server_oauth_config";
        pub const SERVER_OAUTH_TOKENS: &str = "server_oauth_tokens";
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
        pub const PENDING_IMPORT: &str = "pending_import";
        pub const UNIFY_DIRECT_EXPOSURE_ELIGIBLE: &str = "unify_direct_exposure_eligible";
        pub const CAPABILITIES: &str = "capabilities";
        pub const REGISTRY_SERVER_ID: &str = "registry_server_id";
        pub const DESCRIPTION: &str = "description";
        pub const ARGS: &str = "args";
        pub const ENV: &str = "env";
        pub const VALUE: &str = "value";
        pub const PROFILE_ID: &str = "profile_id";
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

/// Profile keys used in client configs
pub mod profile_keys {
    /// Key for MCP tool key in config files
    pub const MCP_TOOL_KEY: &str = "MCPTool";
    /// Key for name in config files
    pub const NAME_KEY: &str = "name";
    /// Key for type in config files
    pub const TYPE_KEY: &str = "type";
    /// Key for transports in config files
    pub const TRANSPORTS_KEY: &str = "transports";
    /// Key for parameters in config files
    pub const PARAMETERS_KEY: &str = "parameters";
    /// Key for tool settings in config files
    pub const TOOL_SETTINGS_KEY: &str = "toolSettings";
    /// Key for tools in config files
    pub const TOOLS_KEY: &str = "tools";
    /// Key for MCPMate in config files
    pub const MCPMATE: &str = "MCPMate";
}

/// Path constants for unified path management
pub mod paths {
    /// Default MCPMate directory name
    pub const MCPMATE_DIR_NAME: &str = ".mcpmate";
    /// Database file name
    pub const DATABASE_FILE_NAME: &str = "mcpmate.db";
    pub const AUDIT_DATABASE_FILE_NAME: &str = "audit.db";
    /// Runtimes directory name
    pub const RUNTIMES_DIR_NAME: &str = "runtimes";
    /// Cache directory name
    pub const CACHE_DIR_NAME: &str = "cache";
    /// Downloads directory name
    pub const DOWNLOADS_DIR_NAME: &str = "downloads";
    /// Binary directory name
    pub const BIN_DIR_NAME: &str = "bin";
}

/// Default values used in configuration
pub mod defaults {
    /// Default server host
    pub const DEFAULT_HOST: &str = "127.0.0.1";
    /// Default cache TTL in seconds
    pub const DEFAULT_CACHE_TTL: u32 = 86400; // 24 hours
    /// Default requests limit
    pub const DEFAULT_REQUESTS_LIMIT: u32 = 100;
    /// Default runtime value
    pub const RUNTIME: &str = "node";

    /// Default behavior for writing logs to file (temporary default: disabled to avoid log bloat)
    pub const LOG_TO_FILE_DEFAULT: bool = false;
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

/// MCPMate branding constants for unified brand management
pub mod branding {
    /// MCPMate product name
    pub const PRODUCT_NAME: &str = "mcpmate";

    /// MCPMate display title
    pub const DISPLAY_TITLE: &str = "MCPMate";

    /// MCPMate official website URL
    pub const WEBSITE_URL: &str = "https://mcp.umate.ai";

    /// MCPMate logo URL (SVG format)
    pub const LOGO_URL: &str = "https://mcp.umate.ai/logo.svg";

    /// MCPMate logo MIME type
    pub const LOGO_MIME_TYPE: &str = "image/svg+xml";

    /// MCPMate description for MCP server info
    pub const DESCRIPTION: &str = "MCPMate - Aggregates tools, resources, and prompts from multiple upstream MCP servers. Connect to access all configured MCP services through a single endpoint.";

    /// Create MCPMate icon for RMCP Implementation
    pub fn create_logo_icon() -> rmcp::model::Icon {
        // Some upstream MCP servers still expect the icon sizes field to follow the
        // pre-2025-06 list-based schema. Until the SDK migrates fully, omit it to
        // stay compatible while keeping the icon metadata available.
        rmcp::model::Icon::new(LOGO_URL).with_mime_type(LOGO_MIME_TYPE)
    }

    /// Create MCPMate Implementation for RMCP server info
    pub fn create_implementation() -> rmcp::model::Implementation {
        rmcp::model::Implementation::new(PRODUCT_NAME, env!("CARGO_PKG_VERSION"))
            .with_title(DISPLAY_TITLE)
            .with_icons(vec![create_logo_icon()])
            .with_website_url(WEBSITE_URL)
    }

    /// Bridge-specific constants
    pub mod bridge {
        /// Bridge client name prefix
        pub const CLIENT_NAME_PREFIX: &str = "mcpmate-bridge";

        /// Bridge server name
        pub const SERVER_NAME: &str = "mcpmate-bridge";

        /// Bridge display title
        pub const DISPLAY_TITLE: &str = "MCPMate Bridge";

        /// Bridge description
        pub const DESCRIPTION: &str = "This is a bridge server that forwards requests to an SSE server.";

        /// Create bridge Implementation with specified name
        fn create_implementation(name: String) -> rmcp::model::Implementation {
            rmcp::model::Implementation::new(name, env!("CARGO_PKG_VERSION"))
                .with_title(DISPLAY_TITLE)
                .with_icons(vec![super::create_logo_icon()])
                .with_website_url(super::WEBSITE_URL)
        }

        /// Create bridge client Implementation with dynamic appid
        pub fn create_client_implementation(appid: &str) -> rmcp::model::Implementation {
            let name = if appid.is_empty() {
                CLIENT_NAME_PREFIX.to_string()
            } else {
                format!("{}::{}", CLIENT_NAME_PREFIX, appid)
            };
            create_implementation(name)
        }

        /// Create bridge server Implementation
        pub fn create_server_implementation() -> rmcp::model::Implementation {
            create_implementation(SERVER_NAME.to_string())
        }
    }
}
