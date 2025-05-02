// MCP Proxy server module
// Contains the ProxyServer struct and related functionality

use rmcp::{
    model::{CallToolRequestParam, CallToolResult, ServerCapabilities, ServerInfo, Tool},
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
    /// Last time the tool mapping was updated
    last_tool_mapping_update: Arc<Mutex<std::time::Instant>>,
    /// Cached tool mapping
    tool_mapping_cache: Arc<Mutex<Option<HashMap<String, super::tool::ToolMapping>>>>,
}

#[tool(tool_box)]
impl ProxyServer {
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
            last_tool_mapping_update: Arc::new(Mutex::new(std::time::Instant::now())),
            tool_mapping_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the cached tool mapping, or build a new one if the cache is expired or empty
    async fn get_tool_mapping(&self) -> HashMap<String, super::tool::ToolMapping> {
        // Cache expiration time (5 seconds)
        const CACHE_EXPIRATION: std::time::Duration = std::time::Duration::from_secs(5);

        // Check if we need to update the cache
        let update_cache = {
            let last_update = self.last_tool_mapping_update.lock().await;
            let cache = self.tool_mapping_cache.lock().await;

            // Update if cache is None or if it's been more than CACHE_EXPIRATION since last update
            cache.is_none() || last_update.elapsed() > CACHE_EXPIRATION
        };

        if update_cache {
            // Build a new tool mapping
            let new_mapping = super::tool::build_tool_mapping(&self.connection_pool).await;

            // Update the cache
            {
                let mut cache = self.tool_mapping_cache.lock().await;
                *cache = Some(new_mapping.clone());

                // Update the last update time
                let mut last_update = self.last_tool_mapping_update.lock().await;
                *last_update = std::time::Instant::now();
            }

            tracing::info!(
                "Updated tool mapping cache with {} tools",
                new_mapping.len()
            );

            new_mapping
        } else {
            // Use the cached mapping
            let cache = self.tool_mapping_cache.lock().await;
            cache.as_ref().unwrap().clone()
        }
    }

    /// Find a tool in the tool mapping
    async fn find_tool(&self, tool_name: &str) -> anyhow::Result<super::tool::ToolMapping> {
        let mapping = self.get_tool_mapping().await;

        mapping.get(tool_name).cloned().ok_or_else(|| {
            anyhow::anyhow!("Tool '{}' not found in any connected server", tool_name)
        })
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
        // Get the cached tool mapping
        let mapping = self.get_tool_mapping().await;

        // Extract all tools from the mapping
        let tools: Vec<Tool> = mapping.values().map(|tm| tm.tool.clone()).collect();

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

        // Find the tool in the mapping
        let tool_name_str = tool_name.to_string();
        let mapping_result = self.find_tool(&tool_name_str).await;

        match mapping_result {
            Ok(mapping) => {
                // Get the server and instance information
                let server_name = mapping.server_name;
                let instance_id = mapping.instance_id;

                // Lock the connection pool to access the service
                let mut pool = self.connection_pool.lock().await;

                // Get the instance
                let conn_result = pool.get_instance_mut(&server_name, &instance_id);

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

                        // Call the tool on the upstream server
                        let call_request = CallToolRequestParam {
                            name: tool_name,
                            arguments,
                        };

                        // Get the service and call the tool
                        let result =
                            match conn.service.as_mut().unwrap().call_tool(call_request).await {
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
                                        tool_name_str,
                                        server_name,
                                        instance_id,
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
            }
            Err(e) => {
                tracing::error!("Error finding tool '{}': {}", tool_name_str, e);
                Err(McpError::invalid_params(
                    format!("Tool '{}' not found", tool_name_str),
                    None,
                ))
            }
        }
    }
}
