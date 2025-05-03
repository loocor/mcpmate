// Core upstream interfaces for MCPMan
// These interfaces define how to interact with upstream MCP servers

use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use super::types::{ConnectionStatus, ResourceUsage, ToolCallResult};

/// Interface for connecting to upstream MCP servers
#[async_trait]
pub trait UpstreamConnector {
    /// Connect to an upstream server
    async fn connect(&self, server_name: &str) -> Result<Box<dyn UpstreamConnection>>;
    
    /// Disconnect from an upstream server
    async fn disconnect(&self, server_name: &str) -> Result<()>;
    
    /// Get all available tools from all connected servers
    async fn get_all_tools(&self) -> Result<HashMap<String, Vec<Tool>>>;
    
    /// Call a tool on an upstream server
    async fn call_tool(
        &self,
        tool_name: &str,
        args: Option<Value>,
    ) -> Result<ToolCallResult>;
}

/// Interface for a connection to an upstream MCP server
#[async_trait]
pub trait UpstreamConnection: Send + Sync {
    /// Get the server name
    fn server_name(&self) -> &str;
    
    /// Get the instance ID
    fn instance_id(&self) -> Uuid;
    
    /// Get the connection status
    fn status(&self) -> ConnectionStatus;
    
    /// Get the resource usage
    fn resource_usage(&self) -> Option<ResourceUsage>;
    
    /// Get all available tools
    async fn get_tools(&self) -> Result<Vec<Tool>>;
    
    /// Call a tool
    async fn call_tool(&self, tool_name: &str, args: Option<Value>) -> Result<Value>;
    
    /// Disconnect from the server
    async fn disconnect(&mut self) -> Result<()>;
    
    /// Reconnect to the server
    async fn reconnect(&mut self) -> Result<()>;
}
