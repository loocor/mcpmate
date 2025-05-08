// MCP Proxy tool module
// Contains functions for handling tool calls and routing them to upstream servers

use anyhow::{Context, Result};
use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing;

use crate::http::pool::UpstreamConnectionPool;

/// Tool mapping information
#[derive(Debug, Clone)]
pub struct ToolMapping {
    /// Name of the server that provides this tool
    pub server_name: String,
    /// ID of the instance that provides this tool
    pub instance_id: String,
    /// Original tool definition
    pub tool: Tool,
    /// Original upstream tool name (without any modifications)
    pub upstream_tool_name: String,
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

/// Tool name mapping information
#[derive(Debug, Clone)]
pub struct ToolNameMapping {
    /// Client-facing tool name (with prefix if needed)
    pub client_tool_name: String,
    /// Server name
    pub server_name: String,
    /// Instance ID
    pub instance_id: String,
    /// Original upstream tool name (without any modifications)
    pub upstream_tool_name: String,
}

/// Get all tools with smart prefixing to avoid name conflicts
pub async fn get_all_tools_with_smart_prefix(
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
pub async fn build_tool_name_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
) -> HashMap<String, ToolNameMapping> {
    // Get all tools with smart prefixing
    let prefixed_tools = get_all_tools_with_smart_prefix(connection_pool).await;

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

/// Detect if a server's tools already have a common prefix
fn detect_common_prefix(tools: &[Tool], server_name: &str) -> bool {
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

/// Call a tool on the appropriate upstream server
pub async fn call_upstream_tool(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    request: CallToolRequestParam,
) -> Result<CallToolResult> {
    // Extract the tool name from the request
    let tool_name = request.name.to_string();

    // Try to parse the tool name to extract server prefix if present
    let (server_prefix, original_tool_name) = parse_tool_name(&tool_name);

    tracing::debug!(
        "Parsed tool name '{}' -> prefix: {:?}, original: '{}'",
        tool_name,
        server_prefix,
        original_tool_name
    );

    // Build the tool mapping to find the server for this tool
    let tool_mapping = build_tool_mapping(connection_pool).await;

    // Find the mapping for this tool
    let mapping = if let Some(server_prefix) = server_prefix {
        // If we have a server prefix, try to find the tool with the original name in that server
        match find_tool_in_server(connection_pool, server_prefix, original_tool_name).await {
            Ok(mapping) => mapping,
            Err(e) => {
                tracing::debug!(
                    "Could not find tool '{}' in server '{}', trying with full name: {}",
                    original_tool_name,
                    server_prefix,
                    e
                );

                // If we couldn't find the tool with the original name, try with the full name
                // This handles cases where the prefix detection might be incorrect
                tool_mapping.get(&tool_name).cloned().context(format!(
                    "Tool '{}' not found in any connected server",
                    tool_name
                ))?
            }
        }
    } else {
        // Otherwise, try to find the tool directly
        tool_mapping.get(&tool_name).cloned().context(format!(
            "Tool '{}' not found in any connected server",
            tool_name
        ))?
    };

    // Get the server and instance
    let server_name = &mapping.server_name;
    let instance_id = &mapping.instance_id;

    // Determine the actual tool name to call on the upstream server
    let upstream_tool_name = if let Some(prefix) = server_prefix {
        // If the server name matches the prefix, use the original tool name
        if prefix.to_lowercase() == server_name.to_lowercase() {
            original_tool_name
        } else {
            // If the prefix doesn't match the server, use the original tool name
            // but log a warning
            tracing::warn!(
                "Tool prefix '{}' doesn't match server name '{}', using original name '{}'",
                prefix,
                server_name,
                original_tool_name
            );
            original_tool_name
        }
    } else {
        // Otherwise, use the tool name as is
        &tool_name
    };

    tracing::info!(
        "Routing tool call '{}' to server '{}' instance '{}' (upstream tool name: '{}')",
        tool_name,
        server_name,
        instance_id,
        upstream_tool_name
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

    // Prepare the request with the upstream tool name
    let upstream_request = CallToolRequestParam {
        name: upstream_tool_name.to_string().into(),
        arguments: request.arguments,
    };

    // Get the service and call the tool
    // We already checked that service is Some above
    let result = conn
        .service
        .as_mut()
        .unwrap()
        .call_tool(upstream_request)
        .await;

    // Mark the connection as ready again
    conn.status = super::types::ConnectionStatus::Ready;

    // Handle the result with detailed error handling
    match result {
        Ok(result) => Ok(result),
        Err(e) => {
            // Handle different types of errors
            use rmcp::ServiceError;
            let error_message = match &e {
                ServiceError::McpError(mcp_err) => {
                    // This is an MCP protocol error
                    tracing::error!(
                        "MCP protocol error calling tool '{}' on server '{}' instance '{}': {}",
                        upstream_tool_name,
                        server_name,
                        instance_id,
                        mcp_err
                    );
                    format!("MCP protocol error: {}", mcp_err)
                }
                ServiceError::Transport(io_err) => {
                    // Transport error (network, IO)
                    tracing::error!(
                        "Transport error calling tool '{}' on server '{}' instance '{}': {}",
                        upstream_tool_name,
                        server_name,
                        instance_id,
                        io_err
                    );

                    // Update connection status to error
                    conn.update_failed(format!("Transport error: {}", io_err));

                    format!("Network or IO error: {}", io_err)
                }
                ServiceError::UnexpectedResponse => {
                    // Unexpected response type
                    tracing::error!(
                        "Unexpected response type from tool '{}' on server '{}' instance '{}'",
                        upstream_tool_name,
                        server_name,
                        instance_id
                    );
                    "Unexpected response type from upstream server".to_string()
                }
                ServiceError::Cancelled { reason } => {
                    // Request was cancelled
                    let reason_str = reason.as_deref().unwrap_or("<unknown>");
                    tracing::error!(
                        "Request cancelled for tool '{}' on server '{}' instance '{}': {}",
                        upstream_tool_name,
                        server_name,
                        instance_id,
                        reason_str
                    );
                    format!("Request cancelled: {}", reason_str)
                }
                ServiceError::Timeout { timeout } => {
                    // Request timed out
                    tracing::error!(
                        "Request timeout for tool '{}' on server '{}' instance '{}' after {:?}",
                        upstream_tool_name,
                        server_name,
                        instance_id,
                        timeout
                    );
                    format!("Request timed out after {:?}", timeout)
                }
                // Handle any future error types that might be added
                _ => {
                    tracing::error!(
                        "Unknown error calling tool '{}' on server '{}' instance '{}': {:?}",
                        upstream_tool_name,
                        server_name,
                        instance_id,
                        e
                    );
                    format!("Unknown error: {:?}", e)
                }
            };

            // Create a detailed error message
            Err(anyhow::anyhow!(
                "Error calling tool '{}' (upstream: '{}') on server '{}' instance '{}': {}",
                tool_name,
                upstream_tool_name,
                server_name,
                instance_id,
                error_message
            ))
        }
    }
}

/// Parse a tool name to extract server prefix if present
/// Returns (Option<server_prefix>, original_tool_name)
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

/// Find a tool in a specific server by its original name
async fn find_tool_in_server(
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
