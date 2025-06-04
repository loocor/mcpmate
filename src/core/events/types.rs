//! Event types for the MCPMate event system

use crate::common::server::TransportType;

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

    /// Config suit enabled status changed
    ConfigSuitStatusChanged {
        /// Config suit ID
        suit_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Server enabled status changed in a config suit
    ServerEnabledInSuitChanged {
        /// Server ID
        server_id: String,
        /// Server name
        server_name: String,
        /// Config suit ID
        suit_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Tool enabled status changed in a config suit
    ToolEnabledInSuitChanged {
        /// Tool ID
        tool_id: String,
        /// Tool name
        tool_name: String,
        /// Config suit ID
        suit_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Resource enabled status changed in a config suit
    ResourceEnabledInSuitChanged {
        /// Resource ID
        resource_id: String,
        /// Resource URI
        resource_uri: String,
        /// Config suit ID
        suit_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Prompt enabled status changed in a config suit
    PromptEnabledInSuitChanged {
        /// Prompt ID
        prompt_id: String,
        /// Prompt name
        prompt_name: String,
        /// Config suit ID
        suit_id: String,
        /// New enabled status
        enabled: bool,
    },

    /// Database was initialized or changed
    DatabaseChanged,

    /// Configuration was reloaded
    ConfigReloaded,

    /// Server transport layer is ready
    ServerTransportReady {
        /// Transport type (SSE, StreamableHttp)
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
}
