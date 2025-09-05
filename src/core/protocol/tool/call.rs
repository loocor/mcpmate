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
    let pool_result = tokio::time::timeout(
        std::time::Duration::from_millis(500), // 500ms timeout for connection pool access
        connection_pool.lock(),
    )
    .await;

    let mut connection_pool_guard = match pool_result {
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

    // Mark the connection as busy
    conn.update_busy();

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

    // Get the service and call the tool with timeout
    let tool_call_result = tokio::time::timeout(
        std::time::Duration::from_secs(30), // 30 second timeout for tool calls
        conn.service.as_mut().unwrap().call_tool(upstream_request),
    )
    .await;

    // Mark the connection as ready again (regardless of result)
    conn.status = crate::core::foundation::types::ConnectionStatus::Ready;

    // Release the connection pool guard early
    drop(connection_pool_guard);

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
