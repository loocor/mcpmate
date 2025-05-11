// ServerHandler implementation for the HTTP proxy server

use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, ListToolsResult, PaginatedRequestParam,
        ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
    Error as McpError, RoleServer, ServiceError,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    conf::operations,
    core::tool::{call_upstream_tool, get_all_with_prefix, parse_tool_name},
    core::ConnectionStatus,
    http::proxy::core::HttpProxyServer,
};

use super::get_tool_name_mapping;

/// Get server information
pub fn get_info(_server: &HttpProxyServer) -> ServerInfo {
    ServerInfo {
        instructions: Some(
            "MCPMate Proxy Server that aggregates tools from multiple MCP servers".into(),
        ),
        capabilities: ServerCapabilities::builder()
            .enable_tools()
            .enable_tool_list_changed()
            .build(),
        ..Default::default()
    }
}

/// List all available tools
pub async fn list_tools(
    server: &HttpProxyServer,
    _request: Option<PaginatedRequestParam>,
    _context: RequestContext<RoleServer>,
) -> Result<ListToolsResult, McpError> {
    // Get tools with smart prefixing
    let all_tools = get_all_with_prefix(&server.connection_pool).await;

    // Filter disabled tools if database is available
    let tools = if let Some(db) = &server.database {
        let mut filtered_tools = Vec::new();

        for tool in all_tools {
            // Parse the tool name to extract server prefix if present
            let (server_prefix, original_tool_name) = parse_tool_name(&tool.name);

            // Get the server name (either from prefix or from the tool name mapping)
            let server_name = if let Some(prefix) = server_prefix {
                prefix.to_string()
            } else {
                // If no prefix, try to get the server name from the tool name mapping
                let tool_name_mapping = get_tool_name_mapping(server).await;
                if let Some(mapping) = tool_name_mapping.get(&tool.name.to_string()) {
                    mapping.server_name.clone()
                } else {
                    // If we can't determine the server, include the tool by default
                    filtered_tools.push(tool);
                    continue;
                }
            };

            // Check if the tool is enabled
            match operations::is_tool_enabled(&db.pool, &server_name, &original_tool_name).await {
                Ok(enabled) => {
                    if enabled {
                        filtered_tools.push(tool);
                    } else {
                        tracing::debug!(
                            "Filtering out disabled tool '{}' from server '{}'",
                            original_tool_name,
                            server_name
                        );
                    }
                }
                Err(e) => {
                    // Log the error but include the tool by default
                    tracing::warn!(
                        "Error checking if tool '{}' is enabled: {}. Including by default.",
                        original_tool_name,
                        e
                    );
                    filtered_tools.push(tool);
                }
            }
        }

        filtered_tools
    } else {
        // If no database, return all tools
        all_tools
    };

    tracing::info!("Returning {} aggregated tools to client", tools.len());

    Ok(ListToolsResult {
        next_cursor: None,
        tools,
    })
}

/// Call a tool on the appropriate upstream server
pub async fn call_tool(
    server: &HttpProxyServer,
    request: CallToolRequestParam,
    _context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    // Extract the tool name and arguments
    let tool_name = request.name.clone();
    let arguments = request.arguments.clone();
    let tool_name_str = tool_name.to_string();

    // Get the tool name mapping
    let tool_name_mapping = get_tool_name_mapping(server).await;

    // Look up the tool in the mapping
    if let Some(mapping) = tool_name_mapping.get(&tool_name_str) {
        // We found the tool in our mapping
        tracing::info!(
            "Found tool '{}' in mapping -> server: '{}', upstream: '{}'",
            tool_name_str,
            mapping.server_name,
            mapping.upstream_tool_name
        );

        // Check if the tool is enabled if database is available
        if let Some(db) = &server.database {
            // Parse the tool name to extract original name
            let (_, original_tool_name) = parse_tool_name(&mapping.upstream_tool_name);

            // Check if the tool is enabled
            match operations::is_tool_enabled(&db.pool, &mapping.server_name, &original_tool_name)
                .await
            {
                Ok(enabled) => {
                    if !enabled {
                        return Err(McpError::invalid_params(
                            format!("Tool '{}' is disabled", tool_name_str),
                            None,
                        ));
                    }
                }
                Err(e) => {
                    // Log the error but allow the tool call to proceed
                    tracing::warn!(
                        "Error checking if tool '{}' is enabled: {}. Allowing by default.",
                        original_tool_name,
                        e
                    );
                }
            }
        }

        // Call the tool on the mapped server and instance
        call_tool_on_instance(
            &server.connection_pool,
            &mapping.server_name,
            &mapping.instance_id,
            &tool_name_str,
            &mapping.upstream_tool_name,
            arguments.into(),
        )
        .await
    } else {
        // Tool not found in mapping, try the old way as fallback
        tracing::warn!(
            "Tool '{}' not found in mapping, trying fallback method",
            tool_name_str
        );

        // Try to parse the tool name to extract server prefix if present
        let (server_prefix, original_tool_name) = parse_tool_name(&tool_name_str);

        // Call the upstream tool
        match call_upstream_tool(
            &server.connection_pool,
            CallToolRequestParam {
                name: tool_name_str.clone().into(),
                arguments,
            },
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::error!("Error calling tool '{}': {}", tool_name_str, e);

                // Provide a more helpful error message if we have a server prefix
                if let Some(server_prefix) = server_prefix {
                    Err(McpError::invalid_params(
                        format!(
                            "Error calling tool '{}' on server '{}': {}",
                            original_tool_name, server_prefix, e
                        ),
                        None,
                    ))
                } else {
                    Err(McpError::invalid_params(
                        format!(
                            "Tool '{}' not found or error occurred: {}",
                            tool_name_str, e
                        ),
                        None,
                    ))
                }
            }
        }
    }
}

/// Helper function to call a tool on a specific server instance
pub async fn call_tool_on_instance(
    connection_pool: &Arc<Mutex<crate::core::UpstreamConnectionPool>>,
    server_name: &str,
    instance_id: &str,
    client_tool_name: &str,
    upstream_tool_name: &str,
    arguments: serde_json::Value,
) -> Result<CallToolResult, McpError> {
    // Lock the connection pool to access the service
    let mut pool = connection_pool.lock().await;

    // Get the instance
    let conn_result = pool.get_instance_mut(server_name, instance_id);

    match conn_result {
        Ok(conn) => {
            // Check if the connection is ready
            if !conn.is_connected() {
                return Err(McpError::internal_error(
                    format!(
                        "Server '{}' instance '{}' is not connected",
                        server_name, instance_id
                    ),
                    None,
                ));
            }

            // Check if the service is available
            if conn.service.is_none() {
                return Err(McpError::internal_error(
                    format!(
                        "Service for server '{}' instance '{}' is not available",
                        server_name, instance_id
                    ),
                    None,
                ));
            }

            // Mark the connection as busy
            conn.update_busy();

            // Prepare the request with the upstream tool name
            let upstream_request = CallToolRequestParam {
                name: upstream_tool_name.to_string().into(),
                arguments: arguments.clone().as_object().cloned(),
            };

            tracing::info!(
                "Calling upstream tool '{}' on server '{}' instance '{}'",
                upstream_tool_name,
                server_name,
                instance_id
            );

            // Call the tool on the upstream server
            let result = match conn
                .service
                .as_mut()
                .unwrap()
                .call_tool(upstream_request)
                .await
            {
                Ok(result) => {
                    // Mark the connection as ready again
                    conn.status = ConnectionStatus::Ready;
                    Ok(result)
                }
                Err(e) => {
                    // Mark the connection as ready again
                    conn.status = ConnectionStatus::Ready;

                    // Handle different types of errors
                    let error_message = match e {
                        ServiceError::McpError(mcp_err) => {
                            // This is already a McpError, so we can just pass it through
                            tracing::error!(
                                "MCP error calling tool '{}' on server '{}' instance '{}': {}",
                                upstream_tool_name,
                                server_name,
                                instance_id,
                                mcp_err
                            );
                            return Err(mcp_err);
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

                    Err(McpError::internal_error(
                        format!(
                            "Error calling tool '{}': {}",
                            client_tool_name, error_message
                        ),
                        None,
                    ))
                }
            };

            result
        }
        Err(e) => {
            tracing::error!("Error getting instance: {}", e);
            Err(McpError::internal_error(
                format!("Error getting instance: {}", e),
                None,
            ))
        }
    }
}
