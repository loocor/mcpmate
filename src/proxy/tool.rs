// MCP Proxy tool module
// Contains functions for handling tool calls and routing them to upstream servers

use anyhow::{Context, Result};
use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing;

use super::pool::UpstreamConnectionPool;

/// Tool mapping information
#[derive(Debug, Clone)]
pub struct ToolMapping {
    /// Name of the server that provides this tool
    pub server_name: String,
    /// ID of the instance that provides this tool
    pub instance_id: String,
    /// Original tool definition
    pub tool: Tool,
}

/// Build a mapping of tool names to server/instance information
pub async fn build_tool_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
) -> HashMap<String, ToolMapping> {
    let mut tool_mapping = HashMap::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Iterate through all servers and instances
    for (server_name, instances) in &pool.connections {
        for (instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance to the mapping
            for tool in &conn.tools {
                // Skip tools that are already in the mapping (first one wins)
                if tool_mapping.contains_key(&tool.name.to_string()) {
                    tracing::warn!(
                        "Tool '{}' is provided by multiple servers, using the first one",
                        tool.name
                    );
                    continue;
                }

                // Add the tool to the mapping
                tool_mapping.insert(
                    tool.name.to_string(),
                    ToolMapping {
                        server_name: server_name.clone(),
                        instance_id: instance_id.clone(),
                        tool: tool.clone(),
                    },
                );
            }
        }
    }

    tracing::info!("Built tool mapping with {} tools", tool_mapping.len());
    tool_mapping
}

/// Get all tools from all connected servers
pub async fn get_all_tools(connection_pool: &Arc<Mutex<UpstreamConnectionPool>>) -> Vec<Tool> {
    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Collect all tools from all connected instances
    let mut all_tools = Vec::new();

    // Iterate through all servers and instances
    for (_server_name, instances) in &pool.connections {
        for (_instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance
            all_tools.extend(conn.tools.clone());
        }
    }

    // Log the number of tools found
    tracing::info!("Found {} tools from all connected servers", all_tools.len());

    all_tools
}

/// Call a tool on the appropriate upstream server
pub async fn call_upstream_tool(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    tool_name: &str,
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<CallToolResult> {
    // Build the tool mapping to find the server for this tool
    let tool_mapping = build_tool_mapping(connection_pool).await;

    // Find the mapping for this tool
    let mapping = tool_mapping.get(tool_name).context(format!(
        "Tool '{}' not found in any connected server",
        tool_name
    ))?;

    // Get the server and instance
    let server_name = &mapping.server_name;
    let instance_id = &mapping.instance_id;

    tracing::info!(
        "Routing tool call '{}' to server '{}' instance '{}'",
        tool_name,
        server_name,
        instance_id
    );

    // Lock the connection pool to access the service
    let mut pool = connection_pool.lock().await;

    // Get the instance
    let conn = pool.get_instance_mut(server_name, instance_id)?;

    // Check if the connection is ready
    if !conn.is_connected() {
        return Err(anyhow::anyhow!(
            "Server '{}' instance '{}' is not connected (status: {})",
            server_name,
            instance_id,
            conn.status
        ));
    }

    // Check if the service is available
    if conn.service.is_none() {
        return Err(anyhow::anyhow!(
            "Service for server '{}' instance '{}' is not available",
            server_name,
            instance_id
        ));
    }

    // Mark the connection as busy
    conn.update_busy();

    // Prepare the request with a String conversion of tool_name
    let request = CallToolRequestParam {
        name: tool_name.to_string().into(),
        arguments,
    };

    // Get the service and call the tool
    // We already checked that service is Some above
    let result = conn.service.as_mut().unwrap().call_tool(request).await;

    // Mark the connection as ready again
    conn.status = super::types::ConnectionStatus::Ready;

    // Handle the result
    match result {
        Ok(result) => Ok(result),
        Err(e) => {
            // Log the error and return it
            tracing::error!(
                "Error calling tool '{}' on server '{}' instance '{}': {}",
                tool_name,
                server_name,
                instance_id,
                e
            );
            Err(anyhow::anyhow!("Error calling tool '{}': {}", tool_name, e))
        }
    }
}
