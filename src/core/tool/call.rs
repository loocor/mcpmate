// Tool call module
// Contains functions for calling tools on upstream servers

use std::sync::Arc;

use anyhow::{Context, Result};
use rmcp::model::{CallToolRequestParam, CallToolResult};
use tokio::sync::Mutex;
use tracing;

use super::mapping::build_tool_mapping;
use crate::core::http::pool::UpstreamConnectionPool;

/// Call a tool on the appropriate upstream server
///
/// This function calls a tool on the appropriate upstream server based on the tool name.
/// It handles tool name prefixing and routing to the correct server and instance.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
/// * `request` - The tool call request
/// * `config_suit_merge_service` - Optional Config Suit merge service for tool enablement check
///
/// # Returns
/// * `Result<CallToolResult>` - The result of the tool call, or an error if the call failed
pub async fn call_upstream_tool(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    request: CallToolRequestParam,
    config_suit_merge_service: Option<&Arc<crate::core::suit::ConfigSuitMergeService>>,
) -> Result<CallToolResult> {
    // Extract the unique name from the request
    let unique_name = request.name.to_string();

    // Variables to store resolved server name and original tool name
    let (server_name, original_tool_name);

    // Try to resolve the unique name using the database if config_suit_merge_service is available
    if let Some(merge_service) = config_suit_merge_service {
        match crate::core::tool::resolve_unique_name(&merge_service.db.pool, &unique_name).await {
            Ok((server, tool)) => {
                server_name = server;
                original_tool_name = tool;

                tracing::debug!(
                    "Resolved unique name '{}' to server '{}' and tool '{}'",
                    unique_name,
                    server_name,
                    original_tool_name
                );
            }
            Err(e) => {
                // If resolution fails, fall back to the tool mapping
                tracing::warn!(
                    "Failed to resolve unique name '{}': {}, falling back to tool mapping",
                    unique_name,
                    e
                );

                // Build the tool mapping to find the server for this tool
                let tool_mapping = build_tool_mapping(connection_pool).await;

                // Find the mapping for this tool
                let mapping = tool_mapping.get(&unique_name).cloned().context(format!(
                    "Tool '{unique_name}' not found in any connected server"
                ))?;

                server_name = mapping.server_name;
                original_tool_name = unique_name.clone();
            }
        }
    } else {
        // If no config_suit_merge_service is available, use the tool mapping
        // Build the tool mapping to find the server for this tool
        let tool_mapping = build_tool_mapping(connection_pool).await;

        // Find the mapping for this tool
        let mapping = tool_mapping.get(&unique_name).cloned().context(format!(
            "Tool '{unique_name}' not found in any connected server"
        ))?;

        server_name = mapping.server_name;
        original_tool_name = unique_name.clone();
    }

    // Get the instance
    let connection_pool_guard = connection_pool.lock().await;
    let instances = connection_pool_guard
        .connections
        .get(&server_name)
        .context(format!(
            "Server '{server_name}' not found in connection pool"
        ))?;

    // Find a connected instance
    let mut instance_id = String::new();
    for (id, conn) in instances {
        if conn.is_connected() {
            instance_id = id.clone();
            break;
        }
    }

    if instance_id.is_empty() {
        return Err(anyhow::anyhow!(
            "No connected instance found for server '{}'",
            server_name
        ));
    }

    // Release the connection pool guard
    drop(connection_pool_guard);

    // Check if the tool is enabled in the config suit
    if let Some(merge_service) = config_suit_merge_service {
        // Get server ID from database
        if let Ok(Some(server)) =
            crate::config::server::get_server(&merge_service.db.pool, &server_name).await
        {
            if let Some(server_id) = &server.id {
                // Check if the tool is enabled
                let is_enabled = merge_service
                    .is_tool_enabled(server_id, &original_tool_name)
                    .await
                    .unwrap_or(true); // Default to enabled if check fails

                if !is_enabled {
                    return Err(anyhow::anyhow!(
                        "Tool '{}' is disabled in the active configuration suits",
                        unique_name
                    ));
                }
            }
        }
    }

    // Use the original tool name for upstream server
    let upstream_tool_name = &original_tool_name;

    tracing::info!(
        "Routing tool call '{}' to server '{}' instance '{}' (upstream tool name: '{}')",
        unique_name,
        server_name,
        instance_id,
        upstream_tool_name
    );

    // Lock the connection pool to access the service
    let mut pool = connection_pool.lock().await;

    // Get the instance
    let conn = pool.get_instance_mut(&server_name, &instance_id)?;

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
    conn.status = crate::core::types::ConnectionStatus::Ready;

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
                    format!("MCP protocol error: {mcp_err}")
                }
                ServiceError::TransportSend(io_err) => {
                    // Transport send error (network, IO)
                    tracing::error!(
                        "Transport send error calling tool '{}' on server '{}' instance '{}': {}",
                        upstream_tool_name,
                        server_name,
                        instance_id,
                        io_err
                    );

                    // Update connection status to error
                    conn.update_failed(format!("Transport send error: {io_err}"));

                    format!("Network or IO error: {io_err}")
                }
                ServiceError::TransportClosed => {
                    // Transport closed error
                    tracing::error!(
                        "Transport closed while calling tool '{}' on server '{}' instance '{}'",
                        upstream_tool_name,
                        server_name,
                        instance_id
                    );

                    // Update connection status to error
                    conn.update_failed("Transport connection closed".to_string());

                    "Connection closed by upstream server".to_string()
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
                    format!("Request cancelled: {reason_str}")
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
                    format!("Request timed out after {timeout:?}")
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
                    format!("Unknown error: {e:?}")
                }
            };

            // Create a detailed error message
            Err(anyhow::anyhow!(
                "Error calling tool '{}' (upstream: '{}') on server '{}' instance '{}': {}",
                unique_name,
                upstream_tool_name,
                server_name,
                instance_id,
                error_message
            ))
        }
    }
}
