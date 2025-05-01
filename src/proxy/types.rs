// MCP Proxy types
// Contains shared type definitions for the MCP proxy server

/// Connection status for an upstream server
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// Server is connected and operational
    Connected,
    /// Server is disconnected
    Disconnected,
    /// Server is in the process of connecting
    Connecting,
    /// Server connection failed with an error
    Failed(String),
}
