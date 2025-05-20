//! Event types for the MCPMate event system

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

    /// Database was initialized or changed
    DatabaseChanged,

    /// Configuration was reloaded
    ConfigReloaded,
}
