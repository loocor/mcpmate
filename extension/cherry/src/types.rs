use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Request structure for MCP server configuration (supports stdio | sse | streamableHttp)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRequest {
    pub id: String,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "type")]
    pub server_type: String,
    pub name: String,

    // stdio fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,

    // http transport fields (Cherry uses baseUrl)
    #[serde(rename = "baseUrl", default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(
        rename = "longRunning",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub long_running: Option<bool>,
}

/// Response structure for MCP server information (mirrors ServerRequest)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerResponse {
    pub id: String,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "type")]
    pub server_type: String,
    pub name: String,

    // stdio fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,

    // http transport fields (Cherry uses baseUrl)
    #[serde(rename = "baseUrl", default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(
        rename = "longRunning",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub long_running: Option<bool>,
}

/// Request structure for updating MCP server list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigRequest {
    pub servers: Vec<ServerRequest>,
}

/// Response structure for MCP server list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfigResponse {
    pub servers: Vec<ServerResponse>,
}

/// Response structure for listing servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerListResponse {
    pub servers: Vec<ServerResponse>,
    pub total_count: usize,
}

// Internal structures for database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DatabaseEntry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub json_data: Option<serde_json::Value>,
}

// Internal structure that matches Cherry Studio's actual format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CherryMcpConfig {
    pub servers: Vec<ServerResponse>,
}

impl From<ServerRequest> for ServerResponse {
    fn from(req: ServerRequest) -> Self {
        ServerResponse {
            id: req.id,
            is_active: req.is_active,
            server_type: req.server_type,
            name: req.name,
            command: req.command,
            args: req.args,
            env: req.env,
            base_url: req.base_url,
            headers: req.headers,
            long_running: req.long_running,
        }
    }
}

impl From<ServerResponse> for ServerRequest {
    fn from(resp: ServerResponse) -> Self {
        ServerRequest {
            id: resp.id,
            is_active: resp.is_active,
            server_type: resp.server_type,
            name: resp.name,
            command: resp.command,
            args: resp.args,
            env: resp.env,
            base_url: resp.base_url,
            headers: resp.headers,
            long_running: resp.long_running,
        }
    }
}

impl From<McpConfigRequest> for McpConfigResponse {
    fn from(req: McpConfigRequest) -> Self {
        McpConfigResponse {
            servers: req.servers.into_iter().map(ServerResponse::from).collect(),
        }
    }
}
