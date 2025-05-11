// Tool mapping module
// Contains functions for building and managing tool mappings

use anyhow::{Context, Result};
use rmcp::model::Tool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing;

use super::types::ToolMapping;
use crate::http::pool::UpstreamConnectionPool;

/// Build a mapping of tool names to server/instance information
///
/// This function builds a mapping of tool names to the server and instance
/// that provides them. It is used to route tool calls to the appropriate upstream server.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `HashMap<String, ToolMapping>` - A mapping of tool names to server/instance information
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
                        upstream_tool_name: tool.name.to_string(),
                    },
                );
            }
        }
    }

    tracing::info!("Built tool mapping with {} tools", tool_mapping.len());
    tool_mapping
}

/// Get all tools from all connected servers
///
/// This function retrieves all tools from all connected servers.
/// It is used to build a comprehensive list of available tools.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `Vec<Tool>` - A list of all tools from all connected servers
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

/// Find a tool in a specific server by its original name
///
/// This function finds a tool in a specific server by its original name.
/// It is used to route tool calls to the appropriate upstream server.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
/// * `server_name` - The name of the server to search
/// * `tool_name` - The name of the tool to find
///
/// # Returns
/// * `Result<ToolMapping>` - The tool mapping if found, or an error if not found
pub async fn find_tool_in_server(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    server_name: &str,
    tool_name: &str,
) -> Result<ToolMapping> {
    let pool = connection_pool.lock().await;

    // Check if the server exists
    let instances = pool.connections.get(server_name).context(format!(
        "Server '{}' not found in connection pool",
        server_name
    ))?;

    // Look for the tool in all instances of this server
    for (instance_id, conn) in instances {
        if !conn.is_connected() {
            continue;
        }

        // Look for the tool in this instance
        for tool in &conn.tools {
            if tool.name.to_string() == tool_name {
                return Ok(ToolMapping {
                    server_name: server_name.to_string(),
                    instance_id: instance_id.clone(),
                    tool: tool.clone(),
                    upstream_tool_name: tool.name.to_string(),
                });
            }
        }
    }

    Err(anyhow::anyhow!(
        "Tool '{}' not found in server '{}'",
        tool_name,
        server_name
    ))
}
