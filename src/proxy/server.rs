// MCP Proxy server module
// Contains the ProxyServer struct and related functionality

use rmcp::{
    model::{CallToolRequestParam, CallToolResult, ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool, Error as McpError, ServerHandler,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use super::pool::UpstreamConnectionPool;
use crate::config::Config;

/// MCP Proxy Server that aggregates tools from multiple MCP servers
#[derive(Debug, Clone)]
pub struct ProxyServer {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    /// Tool name mapping cache
    tool_name_mapping_cache: Arc<Mutex<Option<HashMap<String, super::tool::ToolNameMapping>>>>,
    /// Last time the tool name mapping was updated
    last_tool_mapping_update: Arc<Mutex<std::time::Instant>>,
}

#[tool(tool_box)]
impl ProxyServer {
    /// Get the cached tool name mapping, or build a new one if the cache is expired or empty
    async fn get_tool_name_mapping(&self) -> HashMap<String, super::tool::ToolNameMapping> {
        // Cache expiration time (5 seconds)
        const CACHE_EXPIRATION: std::time::Duration = std::time::Duration::from_secs(5);

        // Check if we need to update the cache
        let update_cache = {
            let last_update = self.last_tool_mapping_update.lock().await;
            let cache = self.tool_name_mapping_cache.lock().await;

            // Update if cache is None or if it's been more than CACHE_EXPIRATION since last update
            cache.is_none() || last_update.elapsed() > CACHE_EXPIRATION
        };

        if update_cache {
            // Build a new tool name mapping
            let new_mapping = super::tool::build_tool_name_mapping(&self.connection_pool).await;

            // Update the cache
            {
                let mut cache = self.tool_name_mapping_cache.lock().await;
                *cache = Some(new_mapping.clone());

                // Update the last update time
                let mut last_update = self.last_tool_mapping_update.lock().await;
                *last_update = std::time::Instant::now();
            }

            tracing::info!(
                "Updated tool name mapping cache with {} entries",
                new_mapping.len()
            );

            new_mapping
        } else {
            // Use the cached mapping
            let cache = self.tool_name_mapping_cache.lock().await;
            cache.as_ref().unwrap().clone()
        }
    }

    pub fn new(config: Arc<Config>, rule_config: Arc<HashMap<String, bool>>) -> Self {
        // Create connection pool
        let mut pool = UpstreamConnectionPool::new(config, rule_config);

        // Initialize the pool
        pool.initialize();

        let connection_pool = Arc::new(Mutex::new(pool));

        // Start health check task
        UpstreamConnectionPool::start_health_check(connection_pool.clone());

        Self {
            connection_pool,
            tool_name_mapping_cache: Arc::new(Mutex::new(None)),
            last_tool_mapping_update: Arc::new(Mutex::new(std::time::Instant::now())),
        }
    }
}

impl ServerHandler for ProxyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCP Proxy Server that aggregates tools from multiple MCP servers".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _: Option<rmcp::model::PaginatedRequestParamInner>,
        _: RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        // Get tools with smart prefixing
        let tools = super::tool::get_all_tools_with_smart_prefix(&self.connection_pool).await;

        tracing::info!("Returning {} aggregated tools to client", tools.len());

        Ok(rmcp::model::ListToolsResult {
            next_cursor: None,
            tools,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Extract the tool name and arguments
        let tool_name = request.name.clone();
        let arguments = request.arguments.clone();
        let tool_name_str = tool_name.to_string();

        // Get the tool name mapping
        let tool_name_mapping = self.get_tool_name_mapping().await;

        // Look up the tool in the mapping
        if let Some(mapping) = tool_name_mapping.get(&tool_name_str) {
            // We found the tool in our mapping
            tracing::info!(
                "Found tool '{}' in mapping -> server: '{}', upstream: '{}'",
                tool_name_str,
                mapping.server_name,
                mapping.upstream_tool_name
            );

            // Lock the connection pool to access the service
            let mut pool = self.connection_pool.lock().await;

            // Get the instance
            let conn_result = pool.get_instance_mut(&mapping.server_name, &mapping.instance_id);

            match conn_result {
                Ok(conn) => {
                    // Check if the connection is ready
                    if !conn.is_connected() {
                        return Err(McpError::internal_error(
                            format!(
                                "Server '{}' instance '{}' is not connected",
                                mapping.server_name, mapping.instance_id
                            ),
                            None,
                        ));
                    }

                    // Check if the service is available
                    if conn.service.is_none() {
                        return Err(McpError::internal_error(
                            format!(
                                "Service for server '{}' instance '{}' is not available",
                                mapping.server_name, mapping.instance_id
                            ),
                            None,
                        ));
                    }

                    // Mark the connection as busy
                    conn.update_busy();

                    // Prepare the request with the upstream tool name
                    let upstream_request = CallToolRequestParam {
                        name: mapping.upstream_tool_name.clone().into(),
                        arguments: arguments.clone(),
                    };

                    tracing::info!(
                        "Calling upstream tool '{}' on server '{}' instance '{}'",
                        mapping.upstream_tool_name,
                        mapping.server_name,
                        mapping.instance_id
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
                            conn.status = super::types::ConnectionStatus::Ready;
                            Ok(result)
                        }
                        Err(e) => {
                            // Mark the connection as ready again
                            conn.status = super::types::ConnectionStatus::Ready;

                            // Log the error and return it
                            tracing::error!(
                                "Error calling tool '{}' on server '{}' instance '{}': {}",
                                mapping.upstream_tool_name,
                                mapping.server_name,
                                mapping.instance_id,
                                e
                            );
                            Err(McpError::internal_error(
                                format!("Error calling tool '{}': {}", tool_name_str, e),
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
        } else {
            // Tool not found in mapping, try the old way as fallback
            tracing::warn!(
                "Tool '{}' not found in mapping, trying fallback method",
                tool_name_str
            );

            // Try to parse the tool name to extract server prefix if present
            let (server_prefix, original_tool_name) = super::tool::parse_tool_name(&tool_name_str);

            // Call the upstream tool
            match super::tool::call_upstream_tool(&self.connection_pool, &tool_name_str, arguments)
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
}
