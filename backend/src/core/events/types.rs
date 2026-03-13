//! Event types for the core event system

use crate::core::transport::TransportType;
use std::collections::HashMap;

/// Cache update types
#[derive(Debug, Clone)]
pub enum CacheUpdateType {
    /// Fresh capabilities fetched from server
    Fresh,
    /// Background refresh completed
    BackgroundRefresh,
    /// Cache restored from file
    FileCache,
    /// Manual refresh requested
    Manual,
}

/// Events that can be published in the system
#[derive(Debug, Clone)]
pub enum Event {
    /// Server global availability status changed
    ServerGlobalStatusChanged {
        /// Server ID
        server_id: String,
        /// Server name
        server_name: String,
        /// New global availability status
        enabled: bool,
    },

    /// Config profile enabled status changed
    ProfileStatusChanged {
        /// Config profile ID
        profile_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Server enabled status changed in a profile
    ServerEnabledInProfileChanged {
        /// Server ID
        server_id: String,
        /// Server name
        server_name: String,
        /// Config profile ID
        profile_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Tool enabled status changed in a profile
    ToolEnabledInProfileChanged {
        /// Tool ID
        tool_id: String,
        /// Tool name
        tool_name: String,
        /// Config profile ID
        profile_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Resource enabled status changed in a profile
    ResourceEnabledInProfileChanged {
        /// Resource ID
        resource_id: String,
        /// Resource URI
        resource_uri: String,
        /// Config profile ID
        profile_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Resource template enabled status changed in a profile
    ResourceTemplateEnabledInProfileChanged {
        /// Template ID
        template_id: String,
        /// Template URI pattern
        uri_template: String,
        /// Config profile ID
        profile_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Prompt enabled status changed in a profile
    PromptEnabledInProfileChanged {
        /// Prompt ID
        prompt_id: String,
        /// Prompt name
        prompt_name: String,
        /// Config profile ID
        profile_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Database was initialized or changed
    DatabaseChanged,

    /// Configuration was reloaded
    ConfigReloaded,

    /// Cache was updated for a server
    CacheUpdated {
        /// Server ID
        server_id: String,
        /// Server name
        server_name: String,
        /// Cache update type
        update_type: CacheUpdateType,
    },

    /// Cache was invalidated for a server
    CacheInvalidated {
        /// Server ID
        server_id: String,
        /// Server name
        server_name: String,
    },

    /// Cache was cleared
    CacheCleared,

    /// Server transport layer is ready
    ServerTransportReady {
        /// Transport type (using core::transport::TransportType)
        transport_type: TransportType,
        /// Ready status
        ready: bool,
    },

    /// Runtime environment check started
    RuntimeCheckStarted {
        /// Runtime type (node, uv, bun)
        runtime_type: String,
        /// Target version (optional)
        version: Option<String>,
    },

    /// Runtime environment check succeeded
    RuntimeCheckSuccess {
        /// Runtime type (node, uv, bun)
        runtime_type: String,
        /// Found version
        version: String,
        /// Binary path
        bin_path: String,
    },

    /// Runtime environment check failed
    RuntimeCheckFailed {
        /// Runtime type (node, uv, bun)
        runtime_type: String,
        /// Error message
        error: String,
    },

    /// Runtime download started
    RuntimeDownloadStarted {
        /// Runtime type (node, uv, bun)
        runtime_type: String,
        /// Version to download
        version: String,
    },

    /// Runtime download completed
    RuntimeDownloadCompleted {
        /// Runtime type (node, uv, bun)
        runtime_type: String,
        /// Downloaded version
        version: String,
        /// Installation path
        install_path: String,
    },

    /// Runtime environment is ready for use
    RuntimeReady {
        /// Runtime type (node, uv, bun)
        runtime_type: String,
        /// Available version
        version: String,
        /// Binary path
        bin_path: String,
    },

    /// Runtime setup failed
    RuntimeSetupFailed {
        /// Runtime type (node, uv, bun)
        runtime_type: String,
        /// Error message
        error: String,
    },

    /// Configuration application started
    ConfigApplicationStarted {
        /// Profile ID that triggered the application
        profile_id: String,
        /// List of servers that need to be started
        servers_to_start: Vec<String>,
        /// List of servers that need to be stopped
        servers_to_stop: Vec<String>,
    },

    /// Server connection startup initiated
    ServerConnectionStartup {
        /// Server name
        server_name: String,
        /// Startup stage (connecting, authenticating, ready)
        stage: String,
        /// Progress percentage (0-100)
        progress: u8,
    },

    /// Server connection startup completed
    ServerConnectionStartupCompleted {
        /// Server ID
        server_id: String,
        /// Server name
        server_name: String,
        /// Whether startup was successful
        success: bool,
        /// Error message if startup failed
        error: Option<String>,
    },

    /// Server connection shutdown initiated
    ServerConnectionShutdown {
        /// Server name
        server_name: String,
    },

    /// Server connection shutdown completed
    ServerConnectionShutdownCompleted {
        /// Server name
        server_name: String,
        /// Whether shutdown was successful
        success: bool,
    },

    /// Configuration application completed
    ConfigApplicationCompleted {
        /// Profile ID that triggered the application
        profile_id: String,
        /// Total number of servers processed
        total_servers: usize,
        /// Successfully started servers
        started_servers: Vec<String>,
        /// Successfully stopped servers
        stopped_servers: Vec<String>,
        /// Failed operations with error messages
        failed_operations: HashMap<String, String>,
        /// Total duration in milliseconds
        duration_ms: u64,
    },

    /// Configuration application progress update
    ConfigApplicationProgress {
        /// Profile ID
        profile_id: String,
        /// Current stage description
        stage: String,
        /// Progress percentage (0-100)
        progress: u8,
        /// Estimated remaining time in seconds
        estimated_remaining_seconds: Option<u32>,
    },
}
