//! Core models for core MCPMate
//! Contains data models for core functionality - completely independent from core

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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Map of MCP server ID to configuration
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: HashMap<String, MCPServerConfig>,
    /// Pagination configuration for proxy responses
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PaginationConfig>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerConfig {
    /// Type of the server (stdio, streamable_http)
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
    /// Default HTTP headers for SSE/Streamable HTTP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

impl MCPServerConfig {
    /// Get the transport type for this server
    pub fn get_transport_type(&self) -> TransportType {
        // Infer strictly from the 'kind' field
        match self.kind {
            ServerType::Stdio => TransportType::Stdio,
            ServerType::StreamableHttp => TransportType::StreamableHttp,
        }
    }
}
