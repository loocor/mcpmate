//! Tool capability helpers
//!
//! Provides mapping builders and helper utilities for working with upstream tools.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{Context, Result};
use futures::StreamExt;
use rmcp::model::{PaginatedRequestParams, Tool};
use tokio::sync::Mutex;
use tracing;

use crate::core::capability::naming::{generate_tool_name_with_policy, infer_tool_naming_policy};
use crate::core::pool::UpstreamConnectionPool;

/// Tool mapping information returned by builders.
#[derive(Debug, Clone)]
pub struct ToolMapping {
    pub server_name: String,
    pub server_id: Option<String>,
    pub instance_id: String,
    pub tool: Tool,
    pub upstream_tool_name: String,
}

/// Legacy struct retained for compatibility; not currently used.
#[derive(Debug, Clone)]
pub struct ToolNameMapping {
    pub client_tool_name: String,
    pub server_name: String,
    pub server_id: Option<String>,
    pub instance_id: String,
    pub upstream_tool_name: String,
}

pub async fn build_tool_mapping(connection_pool: &Arc<Mutex<UpstreamConnectionPool>>) -> HashMap<String, ToolMapping> {
    build_tool_mapping_filtered(connection_pool, None).await
}

pub async fn build_tool_mapping_filtered(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    enabled_server_ids: Option<&HashSet<String>>,
) -> HashMap<String, ToolMapping> {
    let snapshot = {
        let pool = connection_pool.lock().await;
        pool.get_snapshot()
    };

    let mut tasks = Vec::new();
    for (server_id, instances) in snapshot {
        if let Some(enabled_ids) = enabled_server_ids {
            if !enabled_ids.contains(&server_id) {
                tracing::debug!("Skipping disabled server: {}", server_id);
                continue;
            }
        }

        for (instance_id, status, _res, _prm, peer_opt) in instances {
            if !matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) {
                continue;
            }
            let Some(peer) = peer_opt else { continue };

            tasks.push(collect_tools_from_instance_peer(
                server_id.clone(),
                instance_id.clone(),
                peer,
            ));
        }
    }

    let results = futures::stream::iter(tasks)
        .buffer_unordered(crate::core::capability::facade::concurrency_limit())
        .collect::<Vec<_>>()
        .await;

    let mut tool_mapping: HashMap<String, ToolMapping> = HashMap::new();
    for tools in results {
        for tool in tools {
            let unique_name = tool.tool.name.to_string();
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
                continue;
            }
            tool_mapping.insert(unique_name, tool);
        }
    }

    tracing::debug!("Built tool mapping with {} tools", tool_mapping.len());
    tool_mapping
}

pub async fn get_all_tools(connection_pool: &Arc<Mutex<UpstreamConnectionPool>>) -> Vec<Tool> {
    let pool = connection_pool.lock().await;
    let mut all_tools = Vec::new();

    for (server_id, instances) in &pool.connections {
        let server_name = match crate::core::capability::resolver::to_name(server_id).await {
            Ok(Some(name)) => name,
            _ => {
                tracing::warn!("Server ID '{}' not found in resolver, skipping", server_id);
                continue;
            }
        };

        // Aggregate visible tool names across instances to infer a policy per server
        let mut aggregated_names: Vec<&str> = Vec::new();
        for conn in instances.values() {
            if !conn.is_connected() {
                continue;
            }
            for tool in &conn.tools {
                aggregated_names.push(tool.name.as_ref());
            }
        }
        let policy = infer_tool_naming_policy(&server_name, aggregated_names);

        for conn in instances.values() {
            if !conn.is_connected() {
                continue;
            }
            for tool in &conn.tools {
                let unique_name = generate_tool_name_with_policy(&server_name, &tool.name, &policy);
                let mut unique_tool = tool.clone();
                unique_tool.name = unique_name.into();
                all_tools.push(unique_tool);
            }
        }
    }

    tracing::info!(
        "Found {} tools from all connected servers (with standardized names)",
        all_tools.len()
    );

    all_tools
}

pub async fn find_tool_in_server(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    server_name: &str,
    tool_name: &str,
) -> Result<ToolMapping> {
    let pool = connection_pool.lock().await;
    let server_id = crate::core::capability::resolver::to_id(server_name)
        .await
        .ok()
        .flatten()
        .context(format!("Server '{}' not found in resolver", server_name))?;

    let instances = pool.connections.get(&server_id).context(format!(
        "Server '{}' (ID: {}) not found in connection pool",
        server_name, server_id
    ))?;

    for (instance_id, conn) in instances {
        if !conn.is_connected() {
            continue;
        }
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

async fn collect_tools_from_instance_peer(
    server_id: String,
    instance_id: String,
    peer: rmcp::service::Peer<rmcp::service::RoleClient>,
) -> Vec<ToolMapping> {
    let mut results = Vec::new();
    let server_name = match crate::core::capability::resolver::to_name(&server_id).await {
        Ok(Some(name)) => name,
        _ => {
            tracing::warn!("Server ID '{}' not found in resolver, skipping", server_id);
            return results;
        }
    };

    // Collect all tools across pagination first
    let mut raw_tools: Vec<Tool> = Vec::new();
    let mut cursor = None;
    loop {
        match peer
            .list_tools(Some(PaginatedRequestParams::default().with_cursor(cursor)))
            .await
        {
            Ok(result) => {
                for tool in result.tools {
                    raw_tools.push(tool);
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

    // Infer policy for this server from the collected list
    let name_view: Vec<&str> = raw_tools.iter().map(|t| t.name.as_ref()).collect();
    let policy = infer_tool_naming_policy(&server_name, name_view);

    for tool in raw_tools.into_iter() {
        let unique_name = generate_tool_name_with_policy(&server_name, &tool.name, &policy);
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

    results
}
