// MCP Proxy server module
// Contains the ProxyServer struct and related functionality

use rmcp::{
    model::{CallToolRequestParam, CallToolResult, ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool, Error as McpError, ServerHandler,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::core::config::Config;
use crate::sse::pool::UpstreamConnectionPool;

/// MCP Proxy Server that aggregates tools from multiple MCP servers
#[derive(Debug, Clone)]
pub struct ProxyServer {
    /// Connection pool for upstream servers
    pub connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    /// Tool name mapping cache
    tool_name_mapping_cache: Arc<Mutex<Option<HashMap<String, super::tool::ToolNameMapping>>>>,
    /// Last time the tool name mapping was updated
    last_tool_mapping_update: Arc<Mutex<std::time::Instant>>,
    /// Last connection state hash, used to detect changes in the connection pool
    last_connection_state_hash: Arc<Mutex<u64>>,
}

#[tool(tool_box)]
impl ProxyServer {
    /// Get the cached tool name mapping, or build a new one if the cache is expired or empty
    ///
    /// This optimized version uses a more efficient caching strategy:
    /// 1. Uses a longer cache expiration time (2 minutes instead of 30 seconds)
    /// 2. Prioritizes connection state hash changes over time-based expiration
    /// 3. Provides detailed logging about cache update reasons
    /// 4. Reduces lock contention by acquiring locks only when necessary
    async fn get_tool_name_mapping(&self) -> HashMap<String, super::tool::ToolNameMapping> {
        // Cache expiration time (2 minutes)
        const CACHE_EXPIRATION: std::time::Duration = std::time::Duration::from_secs(120);

        // First, check if cache exists without calculating hash (fast path)
        let cache_exists = {
            let cache = self.tool_name_mapping_cache.lock().await;
            cache.is_some()
        };

        // If cache doesn't exist, we definitely need to update
        if !cache_exists {
            return self
                .rebuild_tool_mapping_cache("Cache is empty (first use)")
                .await;
        }

        // Calculate current connection state hash
        let current_hash = {
            let pool = self.connection_pool.lock().await;
            pool.calculate_connection_state_hash()
        };

        // Check if connection state has changed
        let hash_changed = {
            let last_hash = self.last_connection_state_hash.lock().await;
            *last_hash != current_hash
        };

        // If hash changed, update cache immediately
        if hash_changed {
            return self
                .rebuild_tool_mapping_cache("Connection state changed")
                .await;
        }

        // Check if cache has expired (only if hash hasn't changed)
        let cache_expired = {
            let last_update = self.last_tool_mapping_update.lock().await;
            last_update.elapsed() > CACHE_EXPIRATION
        };

        // If cache has expired, update it
        if cache_expired {
            return self.rebuild_tool_mapping_cache("Cache expired").await;
        }

        // Use the cached mapping (fast path)
        let cache = self.tool_name_mapping_cache.lock().await;
        cache.as_ref().unwrap().clone()
    }

    /// Helper method to rebuild the tool mapping cache
    async fn rebuild_tool_mapping_cache(
        &self,
        reason: &str,
    ) -> HashMap<String, super::tool::ToolNameMapping> {
        // Calculate current hash
        let current_hash = {
            let pool = self.connection_pool.lock().await;
            pool.calculate_connection_state_hash()
        };

        // Build a new tool name mapping
        let start_time = std::time::Instant::now();
        let new_mapping = super::tool::build_tool_name_mapping(&self.connection_pool).await;
        let build_time = start_time.elapsed();

        // Update the cache
        {
            let mut cache = self.tool_name_mapping_cache.lock().await;
            *cache = Some(new_mapping.clone());

            // Update the last update time
            let mut last_update = self.last_tool_mapping_update.lock().await;
            *last_update = std::time::Instant::now();

            // Update the last connection state hash
            let mut last_hash = self.last_connection_state_hash.lock().await;
            *last_hash = current_hash;
        }

        tracing::info!(
            "Updated tool name mapping cache with {} entries (reason: {}, build time: {:?})",
            new_mapping.len(),
            reason,
            build_time
        );

        new_mapping
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
            last_connection_state_hash: Arc::new(Mutex::new(0)),
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

                            // Handle different types of errors
                            use rmcp::ServiceError;
                            let error_message = match e {
                                ServiceError::McpError(mcp_err) => {
                                    // This is already a McpError, so we can just pass it through
                                    tracing::error!(
                                        "MCP error calling tool '{}' on server '{}' instance '{}': {}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
                                        mcp_err
                                    );
                                    return Err(mcp_err);
                                }
                                ServiceError::Transport(io_err) => {
                                    // Transport error (network, IO)
                                    tracing::error!(
                                        "Transport error calling tool '{}' on server '{}' instance '{}': {}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
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
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id
                                    );
                                    "Unexpected response type from upstream server".to_string()
                                }
                                ServiceError::Cancelled { reason } => {
                                    // Request was cancelled
                                    let reason_str = reason.as_deref().unwrap_or("<unknown>");
                                    tracing::error!(
                                        "Request cancelled for tool '{}' on server '{}' instance '{}': {}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
                                        reason_str
                                    );
                                    format!("Request cancelled: {}", reason_str)
                                }
                                ServiceError::Timeout { timeout } => {
                                    // Request timed out
                                    tracing::error!(
                                        "Request timeout for tool '{}' on server '{}' instance '{}' after {:?}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
                                        timeout
                                    );
                                    format!("Request timed out after {:?}", timeout)
                                }
                                // Handle any future error types that might be added
                                _ => {
                                    tracing::error!(
                                        "Unknown error calling tool '{}' on server '{}' instance '{}': {:?}",
                                        mapping.upstream_tool_name,
                                        mapping.server_name,
                                        mapping.instance_id,
                                        e
                                    );
                                    format!("Unknown error: {:?}", e)
                                }
                            };

                            Err(McpError::internal_error(
                                format!(
                                    "Error calling tool '{}': {}",
                                    tool_name_str, error_message
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
