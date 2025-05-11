// Tool prefix module
// Contains functions for handling tool name prefixes, parsing, and smart prefixing

use rmcp::model::Tool;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing;

use super::mapping::build_tool_mapping;
use super::types::ToolNameMapping;
use crate::http::pool::UpstreamConnectionPool;

/// Parse a tool name to extract server prefix if present
///
/// This function parses a tool name to extract the server prefix if present.
/// For example, "server_tool" would be parsed as (Some("server"), "tool").
/// It also handles cases like "server_server_tool" where the prefix is repeated.
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

        // Check if the remaining part starts with the same prefix
        // This handles cases like "playwright_playwright_navigate"
        let prefix_repeated = remaining.starts_with(&format!("{}_", server_prefix));

        if prefix_repeated {
            // Skip the repeated prefix
            if let Some(second_pos) = remaining.find('_') {
                let original_tool_name = &remaining[second_pos + 1..];
                tracing::debug!(
                    "Detected repeated prefix in tool name: '{}' -> server: '{}', tool: '{}'",
                    tool_name,
                    server_prefix,
                    original_tool_name
                );
                return (Some(server_prefix), original_tool_name);
            }
        }

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
pub fn detect_common_prefix(tools: &[Tool], server_name: &str) -> bool {
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

/// Get all tools with smart prefixing to avoid name conflicts
///
/// This function retrieves all tools from all connected servers and applies
/// smart prefixing to avoid name conflicts. It is used to build a comprehensive
/// list of available tools with unique names.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `Vec<Tool>` - A list of all tools with smart prefixing
pub async fn get_all_with_prefix(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
) -> Vec<Tool> {
    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // First, collect all tools by server
    let mut tools_by_server: HashMap<String, Vec<Tool>> = HashMap::new();
    let mut all_tool_names: HashMap<String, (String, String)> = HashMap::new(); // tool_name -> (server_name, instance_id)

    // Collect tools and check for name conflicts
    for (server_name, instances) in &pool.connections {
        for (instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Add all tools from this instance to the server's collection
            let server_tools = tools_by_server
                .entry(server_name.clone())
                .or_insert_with(Vec::new);

            for tool in &conn.tools {
                server_tools.push(tool.clone());

                // Convert Cow<str> to String for HashMap key
                let tool_name_string = tool.name.to_string();

                // Track tool names for conflict detection
                if let Some((existing_server, existing_instance)) =
                    all_tool_names.get(&tool_name_string)
                {
                    tracing::warn!(
                        "Tool name conflict: '{}' is provided by both '{}' (instance '{}') and '{}' (instance '{}')",
                        tool_name_string,
                        existing_server,
                        existing_instance,
                        server_name,
                        instance_id
                    );
                } else {
                    all_tool_names
                        .insert(tool_name_string, (server_name.clone(), instance_id.clone()));
                }
            }
        }
    }

    // Detect common prefixes for each server's tools
    let mut server_has_prefix: HashMap<String, bool> = HashMap::new();
    for (server_name, tools) in &tools_by_server {
        let has_common_prefix = detect_common_prefix(tools, server_name);
        server_has_prefix.insert(server_name.clone(), has_common_prefix);

        tracing::debug!(
            "Server '{}' tools {} a common prefix",
            server_name,
            if has_common_prefix {
                "have"
            } else {
                "do not have"
            }
        );
    }

    // Process tools with smart prefixing
    let mut result_tools = Vec::new();
    let mut processed_names = std::collections::HashSet::new();
    let mut name_conflicts = std::collections::HashSet::new();

    // First pass: identify conflicts
    for (tool_name, (_, _)) in &all_tool_names {
        let count = all_tool_names
            .iter()
            .filter(|(name, _)| *name == tool_name)
            .count();

        if count > 1 {
            name_conflicts.insert(tool_name.clone());
        }
    }

    // Second pass: process tools with smart prefixing
    for (server_name, tools) in &tools_by_server {
        let has_prefix = *server_has_prefix.get(server_name).unwrap_or(&false);
        let is_single_tool_server = tools.len() == 1;

        for tool in tools {
            let mut processed_tool = tool.clone();

            // Convert tool name to string for easier manipulation
            let tool_name_string = tool.name.to_string();

            // Check if the tool already has the server prefix
            let server_prefix = format!("{}_", server_name.to_lowercase());
            let already_has_server_prefix =
                tool_name_string.to_lowercase().starts_with(&server_prefix);

            // Check if the tool has any prefix that matches another server name
            let has_other_server_prefix = server_has_prefix
                .iter()
                .filter(|(other_server, _)| *other_server != server_name)
                .any(|(other_server, _)| {
                    let other_prefix = format!("{}_", other_server.to_lowercase());
                    tool_name_string.to_lowercase().starts_with(&other_prefix)
                });

            // Determine if we need to add a prefix
            let needs_prefix = if has_prefix || already_has_server_prefix {
                // Already has prefix, no need to add
                false
            } else if has_other_server_prefix {
                // Has prefix from another server, need to replace with correct prefix
                true
            } else if is_single_tool_server && !name_conflicts.contains(&tool_name_string) {
                // Single tool server with no conflicts, no need to add prefix
                false
            } else {
                // Either multi-tool server without prefix or has conflicts
                true
            };

            if needs_prefix {
                // If the tool already has a prefix from another server, we need to remove it first
                let original_name = if has_other_server_prefix {
                    // Find the underscore and get the part after it
                    if let Some(pos) = tool_name_string.find('_') {
                        &tool_name_string[pos + 1..]
                    } else {
                        &tool_name_string
                    }
                } else {
                    &tool_name_string
                };

                // Add prefix with proper conversion to Cow<str>
                processed_tool.name = format!("{}_{}", server_name, original_name).into();

                tracing::debug!(
                    "Added prefix to tool: '{}' -> '{}'",
                    tool_name_string,
                    processed_tool.name
                );

                // Update description to indicate source
                if !processed_tool
                    .description
                    .as_ref()
                    .map_or(false, |desc| desc.contains(&format!("[{}]", server_name)))
                {
                    let desc = processed_tool
                        .description
                        .as_ref()
                        .map_or("".to_string(), |d| d.to_string());
                    processed_tool.description = Some(format!("[{}] {}", server_name, desc).into());
                }
            }

            // Check for duplicate processed names
            if processed_names.contains(&processed_tool.name) {
                tracing::warn!(
                    "Duplicate processed tool name: '{}' after smart prefixing",
                    processed_tool.name
                );
                continue;
            }

            processed_names.insert(processed_tool.name.clone());
            result_tools.push(processed_tool);
        }
    }

    tracing::info!(
        "Processed {} tools with smart prefixing",
        result_tools.len()
    );

    result_tools
}

/// Build a mapping of client-facing tool names to upstream tool names
///
/// This function builds a mapping of client-facing tool names (which may include
/// a prefix) to the actual upstream tool names. It is used to handle tool name
/// prefixing and routing tool calls to the appropriate upstream server.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `HashMap<String, ToolNameMapping>` - A mapping of client-facing tool names to upstream tool names
pub async fn build_name_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
) -> HashMap<String, ToolNameMapping> {
    // Get all tools with smart prefixing
    let prefixed_tools = get_all_with_prefix(connection_pool).await;

    // Build the original tool mapping
    let original_mapping = build_tool_mapping(connection_pool).await;

    // Create the tool name mapping
    let mut name_mapping = HashMap::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Process each prefixed tool
    for prefixed_tool in &prefixed_tools {
        let client_tool_name = prefixed_tool.name.to_string();

        // Find the original tool and server information
        for (server_name, instances) in &pool.connections {
            for (instance_id, conn) in instances {
                if !conn.is_connected() {
                    continue;
                }

                // Look for the original tool in this instance
                for original_tool in &conn.tools {
                    // Check if this is the same tool (by comparing descriptions or other properties)
                    // This is a heuristic and might need improvement
                    if prefixed_tool.description.as_ref().map_or(false, |desc| {
                        original_tool
                            .description
                            .as_ref()
                            .map_or(false, |orig_desc| desc.contains(&orig_desc.to_string()))
                    }) || (prefixed_tool
                        .name
                        .to_string()
                        .contains(&original_tool.name.to_string())
                        && prefixed_tool.name.to_string() != original_tool.name.to_string())
                    {
                        // Found the original tool
                        name_mapping.insert(
                            client_tool_name.clone(),
                            ToolNameMapping {
                                client_tool_name: client_tool_name.clone(),
                                server_name: server_name.clone(),
                                instance_id: instance_id.clone(),
                                upstream_tool_name: original_tool.name.to_string(),
                            },
                        );
                        break;
                    }
                }
            }
        }
    }

    // Add any tools that weren't processed (e.g., single tool servers without conflicts)
    for prefixed_tool in &prefixed_tools {
        let client_tool_name = prefixed_tool.name.to_string();

        if !name_mapping.contains_key(&client_tool_name) {
            // This tool wasn't processed, which means it's the same as the original
            if let Some(mapping) = original_mapping.get(&client_tool_name) {
                name_mapping.insert(
                    client_tool_name.clone(),
                    ToolNameMapping {
                        client_tool_name: client_tool_name.clone(),
                        server_name: mapping.server_name.clone(),
                        instance_id: mapping.instance_id.clone(),
                        upstream_tool_name: mapping.upstream_tool_name.clone(),
                    },
                );
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
