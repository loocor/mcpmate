// ServerHandler implementation for the HTTP proxy server

use std::collections::HashMap;
use std::sync::Arc;

use rmcp::{
    Error as McpError, RoleServer, ServiceError,
    model::{
        CallToolRequestParam, CallToolResult, ListToolsResult, PaginatedRequestParam,
        ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
};
use tokio::sync::Mutex;

use super::get_tool_name_mapping;
use crate::{
    conf::operations,
    core::{ConnectionStatus, tool::call_upstream_tool},
    http::proxy::core::HttpProxyServer,
};

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
    // Get all tools from all connected servers
    let all_tools = crate::core::tool::get_all_tools(&server.connection_pool).await;

    // Filter disabled tools if database is available
    let mut tools = if let Some(db) = &server.database {
        let mut filtered_tools = Vec::new();
        let mut enabled_count = 0;
        let mut disabled_count = 0;
        let mut error_count = 0;
        let mut unknown_server_count = 0;

        tracing::info!(
            "Filtering tools based on enabled status, total tools before filtering: {}",
            all_tools.len()
        );

        for tool in all_tools {
            // Get server information from the tool name mapping

            // Get the server name from the tool name mapping
            let tool_name_mapping = get_tool_name_mapping(server).await;
            let server_name = if let Some(mapping) = tool_name_mapping.get(&tool.name.to_string()) {
                mapping.server_name.clone()
            } else {
                // If we can't determine the server, include the tool by default
                tracing::warn!(
                    "Could not determine server for tool '{}', including by default",
                    tool.name
                );
                unknown_server_count += 1;
                filtered_tools.push(tool);
                continue;
            };

            // Use the tool name for database check
            let tool_name_for_db_check = tool.name.to_string();

            // Log the tool name used for database check
            tracing::debug!(
                "Using tool name for database check: server='{}', tool_name='{}'",
                server_name,
                tool_name_for_db_check
            );

            tracing::debug!(
                "Checking if tool is enabled: server='{}', tool_name='{}'",
                server_name,
                tool_name_for_db_check
            );

            // Check if the tool is enabled
            match operations::tool::is_tool_enabled(&db.pool, &server_name, &tool_name_for_db_check)
                .await
            {
                Ok(enabled) =>
                    if enabled {
                        tracing::debug!(
                            "Including enabled tool '{}' from server '{}'",
                            tool_name_for_db_check,
                            server_name
                        );
                        enabled_count += 1;
                        filtered_tools.push(tool);
                    } else {
                        tracing::info!(
                            "Filtering out disabled tool '{}' from server '{}'",
                            tool_name_for_db_check,
                            server_name
                        );
                        disabled_count += 1;
                    },
                Err(e) => {
                    // Log the error but include the tool by default
                    tracing::warn!(
                        "Error checking if tool '{}' from server '{}' is enabled: {}. Including by default.",
                        tool_name_for_db_check,
                        server_name,
                        e
                    );
                    error_count += 1;
                    filtered_tools.push(tool);
                }
            }
        }

        tracing::info!(
            "Tool filtering summary: {} enabled, {} disabled, {} errors, {} unknown server",
            enabled_count,
            disabled_count,
            error_count,
            unknown_server_count
        );

        filtered_tools
    } else {
        // If no database, return all tools
        tracing::info!(
            "No database available, returning all {} tools without filtering",
            all_tools.len()
        );
        all_tools
    };

    // Update tool names with unique names from database before returning
    if let Some(db) = &server.database {
        update_tool_names_with_unique_names(&mut tools, &db.pool).await;
    }

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
            // Use the upstream tool name for database check
            let tool_name_for_db_check = mapping.upstream_tool_name.to_string();

            // Log the tool name used for database check
            tracing::info!(
                "Using tool name for database check in call_tool: server='{}', tool_name='{}'",
                mapping.server_name,
                tool_name_for_db_check
            );

            tracing::debug!(
                "Checking if tool is enabled for call: server='{}', tool_name='{}'",
                mapping.server_name,
                tool_name_for_db_check
            );

            // Check if the tool is enabled
            match operations::tool::is_tool_enabled(
                &db.pool,
                &mapping.server_name,
                &tool_name_for_db_check,
            )
            .await
            {
                Ok(enabled) =>
                    if !enabled {
                        return Err(McpError::invalid_params(
                            format!("Tool '{tool_name_str}' is disabled"),
                            None,
                        ));
                    },
                Err(e) => {
                    // Log the error but allow the tool call to proceed
                    tracing::warn!(
                        "Error checking if tool '{}' is enabled: {}. Allowing by default.",
                        tool_name_for_db_check,
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
            server.config_suit_merge_service.as_ref(),
        )
        .await
    } else {
        // Tool not found in mapping, try the old way as fallback
        tracing::warn!(
            "Tool '{}' not found in mapping, trying fallback method",
            tool_name_str
        );

        // Use the tool name directly

        // Get Config Suit merge service if available
        let config_suit_merge_service = server.config_suit_merge_service.as_ref();

        // Call the upstream tool
        match call_upstream_tool(
            &server.connection_pool,
            CallToolRequestParam {
                name: tool_name_str.clone().into(),
                arguments,
            },
            config_suit_merge_service,
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                tracing::error!("Error calling tool '{}': {}", tool_name_str, e);

                // Provide a generic error message
                Err(McpError::invalid_params(
                    format!("Tool '{tool_name_str}' not found or error occurred: {e}"),
                    None,
                ))
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
    _config_suit_merge_service: Option<&Arc<crate::core::suit::ConfigSuitMergeService>>,
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
                    format!("Server '{server_name}' instance '{instance_id}' is not connected"),
                    None,
                ));
            }

            // Check if the service is available
            if conn.service.is_none() {
                return Err(McpError::internal_error(
                    format!(
                        "Service for server '{server_name}' instance '{instance_id}' is not available"
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

            match conn
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

                    Err(McpError::internal_error(
                        format!("Error calling tool '{client_tool_name}': {error_message}"),
                        None,
                    ))
                }
            }
        }
        Err(e) => {
            tracing::error!("Error getting instance: {}", e);
            Err(McpError::internal_error(
                format!("Error getting instance: {e}"),
                None,
            ))
        }
    }
}

/// Update tool names in the tool list with unique names from the database
///
/// This function queries the database for unique names and replaces the original names in the tool list.
///
/// # Arguments
/// * `tools` - The tool list to update
/// * `pool` - The database connection pool
async fn update_tool_names_with_unique_names(
    tools: &mut Vec<rmcp::model::Tool>,
    pool: &sqlx::Pool<sqlx::Sqlite>,
) {
    // If the tool list is empty, return immediately
    if tools.is_empty() {
        return;
    }

    // Get all tool configurations
    let tool_configs = match sqlx::query_as::<_, (String, String, String)>(
        r#"
        SELECT server_name, tool_name, unique_name
        FROM config_suit_tool
        WHERE unique_name IS NOT NULL
        "#
    )
    .fetch_all(pool)
    .await {
        Ok(configs) => configs,
        Err(e) => {
            tracing::error!("Failed to query tool configurations: {}", e);
            return;
        }
    };

    // Create mapping: (server_name, tool_name) -> unique_name
    let mut name_map = HashMap::new();
    for (server_name, tool_name, unique_name) in tool_configs {
        name_map.insert((server_name, tool_name), unique_name);
    }

    // Get tool name mapping
    let tool_name_mapping = match get_tool_name_mapping_for_tools().await {
        Ok(mapping) => mapping,
        Err(e) => {
            tracing::error!("Failed to get tool name mapping: {}", e);
            return;
        }
    };

    // Update tool names
    for tool in tools.iter_mut() {
        let tool_name = tool.name.to_string();

        // Get server name and original tool name from mapping
        if let Some(mapping) = tool_name_mapping.get(&tool_name) {
            let server_name = &mapping.server_name;
            let original_tool_name = &mapping.upstream_tool_name;

            // Look up unique name
            if let Some(unique_name) = name_map.get(&(server_name.clone(), original_tool_name.clone())) {
                // Log name change
                tracing::debug!(
                    "Updating tool name: '{}' -> '{}'",
                    tool.name,
                    unique_name
                );

                // Update tool name
                tool.name = unique_name.clone().into();
            }
        }
    }
}

/// Get tool name mapping for tools
///
/// This function creates a tool name mapping for the tool list.
///
/// # Returns
/// * `Result<HashMap<String, ToolMapping>>` - The tool name mapping
async fn get_tool_name_mapping_for_tools() -> Result<HashMap<String, crate::core::tool::ToolMapping>, anyhow::Error> {
    // Use global proxy server instance to get tool mapping
    if let Some(server) = crate::http::proxy::get_proxy_server() {
        let mapping = get_tool_name_mapping(&server).await;
        Ok(mapping)
    } else {
        Err(anyhow::anyhow!("Failed to get proxy server instance"))
    }
}