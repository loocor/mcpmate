// Tool prefix module
// Contains functions for handling tool name prefixes, parsing, and smart prefixing

use std::{collections::HashMap, sync::Arc};

use rmcp::model::Tool;
use tokio::sync::Mutex;
use tracing;

use super::types::ToolNameMapping;
use crate::http::pool::UpstreamConnectionPool;

/// Parse a tool name to extract server prefix if present
///
/// This function parses a tool name to extract the server prefix if present.
/// For example, "server_tool" would be parsed as (Some("server"), "tool").
///
/// # Arguments
/// * `tool_name` - The tool name to parse
///
/// # Returns
/// * `(Option<&str>, &str)` - A tuple containing the optional server prefix and the original tool name
pub fn parse_tool_name(tool_name: &str) -> (Option<&str>, &str) {
    if let Some(pos) = tool_name.find('_') {
        let server_prefix = &tool_name[0..pos];
        let remaining = &tool_name[pos + 1..];

        tracing::debug!(
            "Parsed tool name: '{}' -> server: '{}', tool: '{}'",
            tool_name,
            server_prefix,
            remaining
        );

        (Some(server_prefix), remaining)
    } else {
        (None, tool_name)
    }
}

/// Detect if a server's tools already have a common prefix
///
/// This function checks if all tools from a server already have a common prefix.
/// It is used to determine whether to add a prefix to tools when aggregating them.
///
/// # Arguments
/// * `tools` - A slice of tools to check
/// * `server_name` - The name of the server
///
/// # Returns
/// * `bool` - True if all tools have a common prefix, false otherwise
pub fn detect_common_prefix(
    tools: &[Tool],
    server_name: &str,
) -> bool {
    if tools.is_empty() {
        return false;
    }

    // Check if all tools start with the server name followed by underscore
    let server_prefix = format!("{}_", server_name.to_lowercase());
    let all_have_server_prefix = tools.iter().all(|tool| {
        tool.name
            .to_string()
            .to_lowercase()
            .starts_with(&server_prefix)
    });

    // If all tools have the server prefix, return true
    if all_have_server_prefix {
        tracing::debug!(
            "All tools for server '{}' already have the server prefix",
            server_name
        );
        return true;
    }

    // Check for other common prefixes (e.g., playwright_, firecrawl_)
    if tools.len() >= 2 {
        // Get the first tool name as String
        let first_tool = tools[0].name.to_string();

        // Find the position of the first underscore
        if let Some(pos) = first_tool.find('_') {
            // Extract the prefix
            let potential_prefix = &first_tool[0..=pos];

            // Check if all tools have this prefix
            let all_have_same_prefix = tools
                .iter()
                .all(|tool| tool.name.to_string().starts_with(potential_prefix));

            if all_have_same_prefix {
                tracing::debug!(
                    "Detected common prefix '{}' for server '{}' tools",
                    potential_prefix,
                    server_name
                );

                // Check if the detected prefix is the same as the server name
                // This handles cases like "playwright_" prefix for "playwright" server
                if potential_prefix.to_lowercase() == server_prefix.to_lowercase() {
                    tracing::debug!("Common prefix matches server name for '{}'", server_name);
                }

                return true;
            }
        }
    }

    false
}

/// Get all tools from all connected servers
///
/// This function retrieves all tools from all connected servers without any
/// modification to their names. It is used to build a comprehensive list of available tools.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `Vec<Tool>` - A list of all tools from all connected servers
pub async fn get_all_tools_without_prefix(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<Tool> {
    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Collect all tools from all connected servers
    let mut result_tools = Vec::new();
    let mut processed_names = std::collections::HashSet::new();

    // Collect tools from all servers
    for (server_name, instances) in &pool.connections {
        for (_instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance to the result
            for tool in &conn.tools {
                let processed_tool = tool.clone();

                // Log the original tool name for debugging
                tracing::info!(
                    "Processing tool: '{}' from server '{}'",
                    processed_tool.name,
                    server_name
                );

                // Check for duplicate names
                if processed_names.contains(&processed_tool.name) {
                    tracing::warn!(
                        "Duplicate tool name: '{}' from server '{}'",
                        processed_tool.name,
                        server_name
                    );
                    continue;
                }

                // Add the tool to the result
                processed_names.insert(processed_tool.name.clone());
                result_tools.push(processed_tool);
            }
        }
    }

    tracing::info!(
        "Processed {} tools from all connected servers",
        result_tools.len()
    );

    result_tools
}

/// Build a mapping of tool names to server information
///
/// This function builds a mapping of tool names to server information.
/// It is used to route tool calls to the appropriate upstream server.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `HashMap<String, ToolNameMapping>` - A mapping of tool names to server information
pub async fn build_tool_server_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> HashMap<String, ToolNameMapping> {
    // Get all tools from all connected servers
    let tools = get_all_tools_without_prefix(connection_pool).await;

    // Create the tool name mapping
    let mut name_mapping = HashMap::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Process each tool
    for tool in &tools {
        let tool_name = tool.name.to_string();

        // Find the server information for this tool
        for (server_name, instances) in &pool.connections {
            for (instance_id, conn) in instances {
                if !conn.is_connected() {
                    continue;
                }

                // Look for the tool in this instance
                for server_tool in &conn.tools {
                    if server_tool.name == tool.name {
                        // Found the tool
                        name_mapping.insert(tool_name.clone(), ToolNameMapping {
                            client_tool_name: tool_name.clone(),
                            server_name: server_name.clone(),
                            instance_id: instance_id.clone(),
                            upstream_tool_name: server_tool.name.to_string(),
                        });

                        tracing::info!(
                            "Mapped tool: '{}' to server: '{}', instance: '{}'",
                            tool_name,
                            server_name,
                            instance_id
                        );

                        break;
                    }
                }
            }
        }
    }

    tracing::info!(
        "Built tool name mapping with {} entries",
        name_mapping.len()
    );

    // Log the mapping for debugging
    for (client_name, mapping) in &name_mapping {
        tracing::debug!(
            "Tool name mapping: '{}' -> server: '{}', upstream: '{}'",
            client_name,
            mapping.server_name,
            mapping.upstream_tool_name
        );
    }

    name_mapping
}
