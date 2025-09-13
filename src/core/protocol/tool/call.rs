//! Tool call module
//!
//! Contains functions for calling tools on upstream servers

use super::database::DatabaseToolService;
use crate::core::pool::UpstreamConnectionPool;
use anyhow::{Context, Result};
use rmcp::model::{CallToolRequestParam, CallToolResult};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing;

/// Call a tool on the appropriate upstream server
///
/// This function calls a tool on the appropriate upstream server based on the tool name.
/// It uses the database service for authoritative tool resolution and routing.
///
/// # Arguments
/// * `db_service` - Database tool service for authoritative tool resolution
/// * `request` - The tool call request
///
/// # Returns
/// * `Result<CallToolResult>` - The result of the tool call, or an error if the call failed
pub async fn call_upstream_tool(
    db_service: &DatabaseToolService,
    request: CallToolRequestParam,
) -> Result<CallToolResult> {
    // Extract the unique name from the request
    let unique_name = request.name.to_string();

    // Resolve tool name using database service (authoritative method)
    let (server_id, original_tool_name) = db_service.resolve_tool(&unique_name).await.context(format!(
        "Failed to resolve tool '{}' using database service",
        unique_name
    ))?;

    // Call the tool using the resolved information
    call_tool_on_server(
        &db_service.connection_pool,
        &server_id,
        &original_tool_name,
        &unique_name,
        request,
    )
    .await
}

/// Call a tool on a specific server (helper function)
///
/// This function handles the actual tool call on a specific server instance.
/// It's used by the database service method for tool calling.
///
/// Features fault isolation and timeout protection to prevent server failures from affecting the system.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
/// * `server_id` - The ID of the server to call
/// * `original_tool_name` - The original tool name on the server
/// * `unique_name` - The unique tool name for logging
/// * `request` - The tool call request
///
/// # Returns
/// * `Result<CallToolResult>` - The result of the tool call
async fn call_tool_on_server(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    server_id: &str,
    original_tool_name: &str,
    unique_name: &str,
    request: CallToolRequestParam,
) -> Result<CallToolResult> {
    // Use timeout to prevent indefinite blocking on connection pool lock
    let mut connection_pool_guard = match tokio::time::timeout(
        std::time::Duration::from_millis(500), // 500ms timeout for connection pool access
        connection_pool.lock(),
    )
    .await
    {
        Ok(guard) => guard,
        Err(_) => {
            tracing::error!(
                "Timeout waiting for connection pool lock in call_tool_on_server for tool '{}'",
                unique_name
            );
            return Err(anyhow::anyhow!("Connection pool access timeout"));
        }
    };

    // Find the server in the connection pool
    let instances = connection_pool_guard
        .connections
        .get_mut(server_id)
        .context(format!("Server with ID '{}' not found in connection pool", server_id))?;

    // Find a connected instance for this server with fault isolation
    let (instance_id, conn) = instances
        .iter_mut()
        .find(|(_, conn)| {
            // Check if connected and has service
            if !conn.is_connected() || conn.service.is_none() {
                return false;
            }

            // Skip servers with permanent errors
            if let crate::core::foundation::types::ConnectionStatus::Error(ref error_details) = conn.status {
                if error_details.error_type == crate::core::foundation::types::ErrorType::Permanent {
                    tracing::debug!(
                        "Skipping server '{}' for tool call due to permanent error: {}",
                        server_id,
                        error_details.message
                    );
                    return false;
                }
            }

            true
        })
        .context(format!(
            "No healthy connected instances found for server '{}'",
            server_id
        ))?;

    let instance_id = instance_id.clone();

    // Check if the connection is busy
    if conn.status == crate::core::foundation::types::ConnectionStatus::Busy {
        return Err(anyhow::anyhow!(
            "Server '{}' instance '{}' is currently busy",
            server_id,
            instance_id
        ));
    }

    // Mark the connection as busy and get service reference
    conn.update_busy();

    // Get a clone of the service Arc for use outside the lock
    let service_arc = conn.service.as_ref().unwrap().clone();

    // Prepare the request with the upstream tool name
    let upstream_request = CallToolRequestParam {
        name: original_tool_name.to_string().into(),
        arguments: request.arguments,
    };

    tracing::info!(
        "Routing tool call '{}' to server '{}' instance '{}' (upstream tool name: '{}')",
        unique_name,
        server_id,
        instance_id,
        original_tool_name
    );

    // Release the connection pool guard BEFORE making the tool call
    drop(connection_pool_guard);

    // Call the tool with timeout (outside the connection pool lock)
    let tool_call_result = tokio::time::timeout(
        std::time::Duration::from_secs(30), // 30 second timeout for tool calls
        service_arc.call_tool(upstream_request),
    )
    .await;

    // Update connection status after tool call completion
    // Use a separate function to handle status update with timeout protection
    update_connection_status_after_tool_call(connection_pool, server_id, &instance_id, &tool_call_result, unique_name)
        .await;

    match tool_call_result {
        Ok(Ok(tool_result)) => {
            tracing::info!(
                "Tool call '{}' completed successfully on server '{}' instance '{}'",
                unique_name,
                server_id,
                instance_id
            );
            Ok(tool_result)
        }
        Ok(Err(e)) => {
            tracing::error!(
                "Tool call '{}' failed on server '{}' instance '{}': {}",
                unique_name,
                server_id,
                instance_id,
                e
            );
            Err(anyhow::anyhow!("Tool call failed on server '{}': {}", server_id, e))
        }
        Err(_) => {
            tracing::error!(
                "Tool call '{}' timed out on server '{}' instance '{}' after 30 seconds",
                unique_name,
                server_id,
                instance_id
            );
            Err(anyhow::anyhow!(
                "Tool call timed out on server '{}' after 30 seconds",
                server_id
            ))
        }
    }
}

/// Update connection status after tool call completion with timeout protection
///
/// This function safely updates the connection status after a tool call completes,
/// with timeout protection to avoid blocking the connection pool.
async fn update_connection_status_after_tool_call(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    server_id: &str,
    instance_id: &str,
    tool_call_result: &Result<Result<CallToolResult, rmcp::service::ServiceError>, tokio::time::error::Elapsed>,
    unique_name: &str,
) {
    // Use timeout to prevent blocking on connection pool lock
    match tokio::time::timeout(
        std::time::Duration::from_millis(500), // 500ms timeout for status update
        connection_pool.lock(),
    )
    .await
    {
        Ok(mut pool_guard) => {
            // Find the connection and update its status
            if let Ok(conn) = pool_guard.get_instance_mut(server_id, instance_id) {
                // Always mark as ready regardless of tool call result
                // The connection is still valid even if the tool call failed or timed out
                conn.status = crate::core::foundation::types::ConnectionStatus::Ready;

                let status_msg = match tool_call_result {
                    Ok(Ok(_)) => "after successful tool call",
                    Ok(Err(_)) => "after tool error (connection still valid)",
                    Err(_) => "after tool timeout (connection still valid)",
                };

                tracing::debug!(
                    "Updated connection status to Ready {} for tool '{}' on server '{}' instance '{}'",
                    status_msg,
                    unique_name,
                    server_id,
                    instance_id
                );
            } else {
                tracing::warn!(
                    "Failed to find connection for status update after tool call '{}' on server '{}' instance '{}'",
                    unique_name,
                    server_id,
                    instance_id
                );
            }
        }
        Err(_) => {
            tracing::warn!(
                "Timeout acquiring connection pool lock for status update after tool call '{}' on server '{}' instance '{}'",
                unique_name,
                server_id,
                instance_id
            );
            // Connection will remain in Busy state, but this is better than blocking the entire pool
            // The health check system will eventually reset connections in persistent Busy state
        }
    }
}
