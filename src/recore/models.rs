//! Core models for recore MCPMate
//! Contains data models for recore functionality - completely independent from core

use crate::common::server::{ServerType, TransportType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pagination configuration for proxy responses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaginationConfig {
    /// Maximum number of items per page
    pub max_page_size: usize,
    /// Default page size if not specified
    pub default_page_size: usize,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Map of MCP server name to configuration
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, MCPServerConfig>,
    /// Pagination configuration for proxy responses
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationConfig>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Type of the server (stdio, sse, streamable_http)
    #[serde(rename = "type")]
    pub kind: ServerType,
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

        // Otherwise, infer from the 'kind' field
        match self.kind {
            ServerType::Stdio => TransportType::Stdio,
            ServerType::Sse => TransportType::Sse,
            ServerType::StreamableHttp => TransportType::StreamableHttp,
        }
    }
}
