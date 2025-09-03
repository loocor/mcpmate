//! Tool mapping module
//!
//! Contains functions for building and managing tool mappings

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use rmcp::model::Tool;

use tokio::sync::Mutex;
use tracing;

use super::naming::generate_unique_name;
use super::types::ToolMapping;
use crate::core::pool::UpstreamConnectionPool;

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
pub async fn build_tool_mapping(connection_pool: &Arc<Mutex<UpstreamConnectionPool>>) -> HashMap<String, ToolMapping> {
    let mut tool_mapping = HashMap::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Iterate through all servers and instances
    for (server_id, instances) in &pool.connections {
        // Get server_name for MCP protocol layer operations using resolver
        let server_name = match crate::core::protocol::resolver::to_name(server_id).await {
            Ok(Some(name)) => name,
            Ok(None) => {
                tracing::warn!("Server ID '{}' not found, skipping", server_id);
                continue;
            }
            Err(e) => {
                tracing::error!("Failed to resolve server ID '{}': {}, skipping", server_id, e);
                continue;
            }
        };

        for (instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance to the mapping
            for tool in &conn.tools {
                // Generate a unique name for this tool (with server prefix)
                let unique_name = generate_unique_name(&server_name, &tool.name);

                // Skip tools that are already in the mapping (first one wins)
                if tool_mapping.contains_key(&unique_name) {
                    tracing::warn!(
                        "Tool '{}' (unique name: '{}') is provided by multiple servers, using the first one",
                        tool.name,
                        unique_name
                    );
                    continue;
                }

                // Create a modified tool with the unique name
                let mut unique_tool = tool.clone();
                unique_tool.name = unique_name.clone().into();

                // Add the tool to the mapping
                tool_mapping.insert(
                    unique_name.clone(),
                    ToolMapping {
                        server_name: server_name.clone(),
                        instance_id: instance_id.clone(),
                        tool: unique_tool,
                        upstream_tool_name: tool.name.to_string(),
                    },
                );

                tracing::debug!(
                    "Added tool '{}' -> '{}' from server '{}'",
                    tool.name,
                    unique_name,
                    server_name
                );
            }
        }
    }

    tracing::debug!("Built tool mapping with {} tools", tool_mapping.len());
    tool_mapping
}

/// Get all tools from all connected servers with standardized names
///
/// This function retrieves all tools from all connected servers and applies
/// name standardization to prevent conflicts between tools from different servers.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `Vec<Tool>` - A list of all tools with standardized names
pub async fn get_all_tools(connection_pool: &Arc<Mutex<UpstreamConnectionPool>>) -> Vec<Tool> {
    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Collect all tools from all connected instances
    let mut all_tools = Vec::new();

    // Iterate through all servers and instances
    for (server_id, instances) in &pool.connections {
        // Get server_name for MCP protocol layer operations using resolver
        let server_name = match crate::core::protocol::resolver::to_name(server_id).await {
            Ok(Some(name)) => name,
            Ok(None) => {
                tracing::warn!("Server ID '{}' not found, skipping", server_id);
                continue;
            }
            Err(e) => {
                tracing::error!("Failed to resolve server ID '{}': {}, skipping", server_id, e);
                continue;
            }
        };

        for conn in instances.values() {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance with standardized names
            for tool in &conn.tools {
                // Generate a unique name for this tool
                let unique_name = generate_unique_name(&server_name, &tool.name);

                // Create a modified tool with the unique name
                let mut unique_tool = tool.clone();
                unique_tool.name = unique_name.into();

                all_tools.push(unique_tool);
            }
        }
    }

    // Log the number of tools found
    tracing::info!(
        "Found {} tools from all connected servers (with standardized names)",
        all_tools.len()
    );

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

    // Convert server_name to server_id for connection pool lookup using resolver
    let server_id = match crate::core::protocol::resolver::to_id(server_name).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            return Err(anyhow::anyhow!("Server '{}' not found in database", server_name));
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to resolve server '{}': {}", server_name, e));
        }
    };

    // Check if the server exists in connection pool
    let instances = pool.connections.get(&server_id).context(format!(
        "Server '{}' (ID: {}) not found in connection pool",
        server_name, server_id
    ))?;

    // Look for the tool in all instances of this server
    for (instance_id, conn) in instances {
        if !conn.is_connected() {
            continue;
        }

        // Look for the tool in this instance
        for tool in &conn.tools {
            if tool.name == tool_name {
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
