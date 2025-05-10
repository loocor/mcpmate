// Core models for MCPMate
// Contains data models for core functionality

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::transport::TransportType;

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Map of MCP server name to configuration
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, MCPServerConfig>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Type of the server (stdio, sse, streamable_http)
    #[serde(rename = "type")]
    pub kind: String,
    /// Command to execute (for stdio servers)
    pub command: Option<String>,
    /// Arguments to pass to the command (for stdio servers)
    pub args: Option<Vec<String>>,
    /// URL (for sse and streamable_http servers)
    pub url: Option<String>,
    /// Environment variables to set (for stdio servers)
    pub env: Option<HashMap<String, String>>,
    /// Transport type
    #[serde(rename = "transportType")]
    #[serde(default)]
    pub transport_type: Option<TransportType>,
}

impl MCPServerConfig {
    /// Get the transport type for this server
    pub fn get_transport_type(&self) -> TransportType {
        // If transport_type is explicitly set, use it
        if let Some(transport_type) = self.transport_type {
            return transport_type;
        }

        // Otherwise, infer from the 'kind' field for backward compatibility
        match self.kind.as_str() {
            "stdio" => TransportType::Stdio,
            "sse" => TransportType::Sse,
            "streamable_http" | "streamablehttp" => TransportType::StreamableHttp,
            _ => {
                // Default to SSE for unknown types
                tracing::warn!("Unknown server type: {}, defaulting to SSE", self.kind);
                TransportType::Sse
            }
        }
    }
}
