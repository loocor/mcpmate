//! Tool call module
//!
//! Contains functions for calling tools on upstream servers

use std::sync::Arc;

use anyhow::{Context, Result};
use rmcp::model::{CallToolRequestParam, CallToolResult};
use tokio::sync::Mutex;
use tracing;

use crate::core::{foundation::types::ConnectionStatus, pool::UpstreamConnectionPool};

/// Call a tool on the appropriate upstream server
///
/// This function calls a tool on the appropriate upstream server based on the tool name.
/// It handles tool name resolution and routing to the correct server and instance using
/// configuration suits to determine tool availability and permissions.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
/// * `request` - The tool call request
/// * `suit_service` - Suit service for tool resolution and enablement check
///
/// # Returns
/// * `Result<CallToolResult>` - The result of the tool call, or an error if the call failed
pub async fn call_upstream_tool(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    request: CallToolRequestParam,
    suit_service: &Arc<crate::core::suit::SuitService>,
) -> Result<CallToolResult> {
    // Extract the unique name from the request
    let unique_name = request.name.to_string();

    // Resolve tool name using SuitService
    let (server_name, original_tool_name) =
        resolve_tool_with_suit_service(connection_pool, &unique_name, suit_service)
            .await
            .context(format!(
                "Failed to resolve tool '{}' to any available server",
                unique_name
            ))?;

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

    // Check if the tool is enabled for this server using SuitService
    let tool_enabled = check_tool_enablement(suit_service, &server_name, &original_tool_name)
        .await
        .context(format!(
            "Failed to check enablement for tool '{}' on server '{}'",
            original_tool_name, server_name
        ))?;

    if !tool_enabled {
        return Err(anyhow::anyhow!(
            "Tool '{}' is disabled for server '{}' according to configuration suits",
            original_tool_name,
            server_name
        ));
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
    conn.status = ConnectionStatus::Ready;

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

/// Resolve tool name using SuitService
///
/// This function resolves a tool name using the SuitService, which respects
/// configuration suit settings. Only tools that are enabled in active suits
/// and available on connected servers will be resolved.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
/// * `tool_name` - The tool name to resolve
/// * `suit_service` - SuitService for configuration-aware resolution
///
/// # Returns
/// * `Result<(String, String)>` - Tuple of (server_name, original_tool_name)
async fn resolve_tool_with_suit_service(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    tool_name: &str,
    suit_service: &Arc<crate::core::suit::SuitService>,
) -> Result<(String, String)> {
    tracing::debug!(
        "Resolving tool '{}' using SuitService configuration",
        tool_name
    );

    // Get merged tool configurations from all active suits
    let merge_result = suit_service.merge_all_configs().await.context(format!(
        "Failed to get SuitService configuration for tool '{}'",
        tool_name
    ))?;

    // Look for the tool in the merged configuration
    let tool_config = merge_result
        .tools
        .iter()
        .find(|t| t.tool_name == tool_name)
        .context(format!(
            "Tool '{}' not found in any active configuration suit",
            tool_name
        ))?;

    // Check if the tool is enabled
    if !tool_config.enabled {
        return Err(anyhow::anyhow!(
            "Tool '{}' is disabled in configuration suits",
            tool_name
        ));
    }

    // Check if any servers are configured for this tool
    if tool_config.server_ids.is_empty() {
        return Err(anyhow::anyhow!(
            "Tool '{}' has no servers configured in configuration suits",
            tool_name
        ));
    }

    // Find the first available server that has this tool enabled and connected
    for server_id in &tool_config.server_ids {
        if let Ok(mapping) =
            super::mapping::find_tool_in_server(connection_pool, server_id, tool_name).await
        {
            tracing::info!(
                "Resolved tool '{}' to server '{}' (enabled in {} suits)",
                tool_name,
                server_id,
                tool_config.source_suits.len()
            );
            return Ok((mapping.server_name, mapping.upstream_tool_name));
        }
    }

    // Tool is enabled in suits but no connected server provides it
    Err(anyhow::anyhow!(
        "Tool '{}' is enabled in configuration suits but no connected server provides it (configured servers: {:?})",
        tool_name,
        tool_config.server_ids
    ))
}

/// Check if a tool is enabled for a specific server using SuitService
///
/// This function checks whether a tool is enabled for a specific server
/// according to the configuration suits. Returns an error if the check fails.
///
/// # Arguments
/// * `suit_service` - SuitService for enablement checking
/// * `server_name` - The server name to check
/// * `tool_name` - The tool name to check
///
/// # Returns
/// * `Result<bool>` - True if the tool is enabled, false otherwise
async fn check_tool_enablement(
    suit_service: &Arc<crate::core::suit::SuitService>,
    server_name: &str,
    tool_name: &str,
) -> Result<bool> {
    tracing::debug!(
        "Checking tool enablement for '{}' on server '{}' using SuitService",
        tool_name,
        server_name
    );

    // Check if the tool is enabled for this specific server
    let enabled = suit_service
        .is_tool_enabled_for_server(server_name, tool_name)
        .await
        .context(format!(
            "Failed to check tool enablement for '{}' on server '{}'",
            tool_name, server_name
        ))?;

    if enabled {
        tracing::debug!(
            "Tool '{}' is enabled for server '{}' according to SuitService",
            tool_name,
            server_name
        );
    } else {
        tracing::warn!(
            "Tool '{}' is disabled for server '{}' according to SuitService",
            tool_name,
            server_name
        );
    }

    Ok(enabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::database::Database;
    use crate::core::suit::SuitService;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_resolve_tool_with_suit_service_not_found() {
        // Test that the function returns error when tool is not found in SuitService

        // Create a mock database and SuitService
        let db = Arc::new(Database::new().await.unwrap());
        let suit_service = Arc::new(SuitService::new(db));

        // Create a mock config
        let config = Arc::new(crate::core::models::Config {
            mcp_servers: std::collections::HashMap::new(),
            pagination: None,
        });

        // Create a mock connection pool (empty)
        let pool = Arc::new(Mutex::new(UpstreamConnectionPool::new(config, None)));

        // Test with SuitService but nonexistent tool
        let result = resolve_tool_with_suit_service(&pool, "nonexistent_tool", &suit_service).await;

        // Should fail because tool is not found in configuration suits
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not found in any active configuration suit")
        );
    }

    #[tokio::test]
    async fn test_check_tool_enablement_with_service() {
        // Test tool enablement checking with a SuitService

        // Create a mock database and SuitService
        let db = Arc::new(Database::new().await.unwrap());
        let suit_service = Arc::new(SuitService::new(db));

        // Test enablement check (will likely return true due to empty database)
        let result = check_tool_enablement(&suit_service, "test_server", "test_tool").await;

        // Should succeed (may return true or false depending on configuration)
        assert!(result.is_ok());
    }
}
