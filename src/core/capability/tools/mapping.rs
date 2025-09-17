//! Tool mapping module
//!
//! Contains functions for building and managing tool mappings

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use futures::StreamExt;
use rmcp::model::{PaginatedRequestParam, Tool};

use tokio::sync::Mutex;
use tracing;

use super::types::ToolMapping;
use crate::core::capability::naming::{generate_unique_name, NamingKind};
use crate::core::capability::internal::concurrency_limit;
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
    build_tool_mapping_filtered(connection_pool, None).await
}

pub async fn build_tool_mapping_filtered(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    enabled_server_ids: Option<&std::collections::HashSet<String>>,
) -> HashMap<String, ToolMapping> {
    // Step 1: Take light snapshot (only lock acquisition point)
    let snapshot = {
        let pool = connection_pool.lock().await;
        pool.get_snapshot()
    };

    // Step 2: Build tasks for parallel execution (no locks held)
    let mut tasks = Vec::new();

    for (server_id, instances) in snapshot {
        // Pre-filter by enabled server IDs if provided
        if let Some(enabled_ids) = enabled_server_ids {
            if !enabled_ids.contains(&server_id) {
                tracing::debug!("Skipping disabled server: {}", server_id);
                continue;
            }
        }

        for (instance_id, status, _res, _prm, peer_opt) in instances {
            // Pre-filter: only Ready instances with an available peer
            if !matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) {
                continue;
            }
            let Some(peer) = peer_opt else {
                continue;
            };

            // Create async task for this server/instance combination
            let server_id_clone = server_id.clone();
            let instance_id_clone = instance_id.clone();
            tasks.push(collect_tools_from_instance_peer(
                server_id_clone,
                instance_id_clone,
                peer,
            ));
        }
    }

    // Step 3: Execute all tasks concurrently with bounded concurrency
    let results = futures::stream::iter(tasks)
        .buffer_unordered(concurrency_limit())
        .collect::<Vec<_>>()
        .await;

    // Step 4: Merge results and handle conflicts
    let mut tool_mapping: HashMap<String, ToolMapping> = HashMap::new();
    for tools in results {
        for tool in tools {
            let unique_name = tool.tool.name.to_string();

            // Check for name conflicts and log warnings
            if let Some(existing) = tool_mapping.get(&unique_name) {
                tracing::warn!(
                    "Tool '{}' (unique name: '{}') is provided by both '{}' (instance: {}) and '{}' (instance: {}). Using the first one.",
                    tool.upstream_tool_name,
                    unique_name,
                    existing.server_name,
                    existing.instance_id,
                    tool.server_name,
                    tool.instance_id
                );
                continue; // Keep the first one, skip the conflicting one
            }

            tool_mapping.insert(unique_name, tool);
        }
    }

    tracing::debug!("Built tool mapping with {} tools", tool_mapping.len());
    tool_mapping
}

/// Collect tools from a single instance peer (lightweight path)
async fn collect_tools_from_instance_peer(
    server_id: String,
    instance_id: String,
    peer: rmcp::service::Peer<rmcp::service::RoleClient>,
) -> Vec<ToolMapping> {
    let mut results = Vec::new();

    // Resolve server name using the global mapping manager
    let server_name = match crate::core::capability::global_server_mapping_manager()
        .get_name_by_id(&server_id)
        .await
    {
        Some(name) => name,
        None => {
            tracing::warn!("Server ID '{}' not found in mapping manager, skipping", server_id);
            return results;
        }
    };

    // List tools via peer with pagination (if supported)
    let mut cursor = None;
    loop {
        match peer.list_tools(Some(PaginatedRequestParam { cursor })).await {
            Ok(result) => {
                for tool in result.tools {
                    let unique_name = generate_unique_name(NamingKind::Tool, &server_name, &tool.name);
                    let mut unique_tool = tool.clone();
                    unique_tool.name = unique_name.clone().into();

                    results.push(ToolMapping {
                        server_name: server_name.clone(),
                        server_id: Some(server_id.clone()),
                        instance_id: instance_id.clone(),
                        tool: unique_tool,
                        upstream_tool_name: tool.name.to_string(),
                    });
                }
                cursor = result.next_cursor;
                if cursor.is_none() {
                    break;
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to collect tools from instance {} (server: {}): {}",
                    instance_id,
                    server_name,
                    e
                );
                break;
            }
        }
    }

    results
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
        // Get server_name using the global mapping manager
        let server_name = match crate::core::capability::global_server_mapping_manager()
            .get_name_by_id(server_id)
            .await
        {
            Some(name) => name,
            None => {
                tracing::warn!("Server ID '{}' not found in mapping manager, skipping", server_id);
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
                    let unique_name = generate_unique_name(NamingKind::Tool, &server_name, &tool.name);

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

    // Convert server_name to server_id using the global mapping manager
    let server_id = match crate::core::capability::global_server_mapping_manager()
        .get_id_by_name(server_name)
        .await
    {
        Some(id) => id,
        None => {
            return Err(anyhow::anyhow!("Server '{}' not found in mapping manager", server_name));
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
                    server_id: Some(server_id.clone()),
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
