// MCP Discovery Client
// Provides independent MCP server connection capabilities for discovery system

use std::time::Duration;
use tokio_util::sync::CancellationToken;

use super::types::{
    CapabilitiesMetadata, DiscoveryError, DiscoveryResult, PromptArgument, PromptInfo,
    ResourceInfo, ServerCapabilities, ToolInfo,
};
use crate::common::server::{ServerType, TransportType};
use crate::config::database::Database;
use crate::core::models::MCPServerConfig;
use crate::core::transport::unified;

/// MCP Discovery Client for independent server connections
#[derive(Clone)]
pub struct McpDiscoveryClient {
    /// Connection timeout
    connection_timeout: Duration,
    /// Request timeout
    request_timeout: Duration,
}

impl McpDiscoveryClient {
    /// Create a new discovery client
    pub fn new() -> Self {
        Self {
            connection_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(10),
        }
    }

    /// Create a new discovery client with custom timeouts
    pub fn with_timeouts(
        connection_timeout: Duration,
        request_timeout: Duration,
    ) -> Self {
        Self {
            connection_timeout,
            request_timeout,
        }
    }

    /// Get server capabilities by connecting temporarily
    pub async fn get_server_capabilities(
        &self,
        server_id: &str,
        database: &Database,
    ) -> DiscoveryResult<ServerCapabilities> {
        // Get server configuration from database
        let server = self.get_server_from_database(server_id, database).await?;

        // Convert to MCPServerConfig format
        let server_config = self.convert_to_mcp_config(&server, database).await?;

        // Create temporary connection and collect capabilities
        let capabilities = self
            .connect_and_collect_capabilities(server_id, &server_config, database)
            .await?;

        Ok(capabilities)
    }

    /// Connect to server and collect capabilities using unified transport
    async fn connect_and_collect_capabilities(
        &self,
        server_id: &str,
        server_config: &MCPServerConfig,
        database: &Database,
    ) -> DiscoveryResult<ServerCapabilities> {
        // Create cancellation token for timeout control
        let ct = CancellationToken::new();

        // Set up timeout
        let timeout_future = tokio::time::sleep(self.connection_timeout);
        tokio::pin!(timeout_future);

        // Use the transport type from config, with fallback
        let transport_type = server_config.transport_type.unwrap_or(TransportType::Stdio);
        let server_type = self.transport_to_server_type(&transport_type)?;

        // Connect using unified transport interface
        let connection_result = tokio::select! {
            result = unified::connect_server(
                server_id,
                server_config,
                server_type,
                transport_type,
                Some(ct.clone()),
                Some(&database.pool),
                None, // No runtime cache needed for discovery
            ) => result,
            _ = &mut timeout_future => {
                ct.cancel();
                return Err(DiscoveryError::Timeout("Connection timeout".to_string()));
            }
        };

        let (service, tools, server_capabilities, _process_id) =
            connection_result.map_err(|e| DiscoveryError::ConnectionFailed(e.to_string()))?;

        // Get additional capabilities from the service
        let resources = self.get_resources_from_service(&service).await?;
        let prompts = self.get_prompts_from_service(&service).await?;
        // Note: Resource templates implementation is simplified for now
        // TODO: Implement proper resource template conversion once RMCP API is stable

        // Convert to our format
        let capabilities = self
            .convert_capabilities(
                server_id,
                tools,
                resources,
                prompts,
                server_capabilities,
                database,
            )
            .await?;

        // Cancel the service to clean up
        if let Err(e) = service.cancel().await {
            tracing::warn!(
                "Failed to cancel discovery service for {}: {}",
                server_id,
                e
            );
        }

        Ok(capabilities)
    }

    /// Get server from database
    async fn get_server_from_database(
        &self,
        server_id: &str,
        database: &Database,
    ) -> DiscoveryResult<crate::config::models::server::Server> {
        // Get server from database by ID (not by name)
        let server = crate::config::server::get_server_by_id(&database.pool, server_id)
            .await
            .map_err(|e| DiscoveryError::DatabaseError(e.to_string()))?
            .ok_or_else(|| DiscoveryError::ServerNotFound(server_id.to_string()))?;

        Ok(server)
    }

    /// Convert database server model to MCPServerConfig
    async fn convert_to_mcp_config(
        &self,
        server: &crate::config::models::server::Server,
        database: &Database,
    ) -> DiscoveryResult<MCPServerConfig> {
        // Get server arguments and environment variables
        let server_id = server.id.clone().unwrap_or_default();

        let args = crate::config::server::get_server_args(&database.pool, &server_id)
            .await
            .map_err(|e| DiscoveryError::DatabaseError(e.to_string()))?;

        let env = crate::config::server::get_server_env(&database.pool, &server_id)
            .await
            .map_err(|e| DiscoveryError::DatabaseError(e.to_string()))?;

        // Convert args to Vec<String>
        let args_strings: Vec<String> = args.into_iter().map(|arg| arg.arg_value).collect();

        // Convert env to HashMap<String, String>
        let env_map: std::collections::HashMap<String, String> = env.into_iter().collect();

        Ok(MCPServerConfig {
            kind: server.server_type,
            command: server.command.clone(),
            args: Some(args_strings),
            env: Some(env_map),
            url: server.url.clone(),
            transport_type: server.transport_type,
        })
    }

    /// Convert transport type to server type
    fn transport_to_server_type(
        &self,
        transport_type: &TransportType,
    ) -> DiscoveryResult<ServerType> {
        match transport_type {
            TransportType::Stdio => Ok(ServerType::Stdio),
            TransportType::Sse => Ok(ServerType::Sse),
            TransportType::StreamableHttp => Ok(ServerType::StreamableHttp),
        }
    }

    /// Get resources from the service
    async fn get_resources_from_service(
        &self,
        service: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
    ) -> DiscoveryResult<Vec<rmcp::model::Resource>> {
        match tokio::time::timeout(self.request_timeout, service.list_all_resources()).await {
            Ok(Ok(resources)) => Ok(resources),
            Ok(Err(e)) => {
                tracing::warn!("Failed to get resources: {}", e);
                Ok(Vec::new()) // Return empty list on error
            }
            Err(_) => {
                tracing::warn!("Timeout getting resources");
                Ok(Vec::new()) // Return empty list on timeout
            }
        }
    }

    /// Get prompts from the service
    async fn get_prompts_from_service(
        &self,
        service: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
    ) -> DiscoveryResult<Vec<rmcp::model::Prompt>> {
        match tokio::time::timeout(self.request_timeout, service.list_all_prompts()).await {
            Ok(Ok(prompts)) => Ok(prompts),
            Ok(Err(e)) => {
                tracing::warn!("Failed to get prompts: {}", e);
                Ok(Vec::new()) // Return empty list on error
            }
            Err(_) => {
                tracing::warn!("Timeout getting prompts");
                Ok(Vec::new()) // Return empty list on timeout
            }
        }
    }

    /// Convert rmcp types to our discovery types
    async fn convert_capabilities(
        &self,
        server_id: &str,
        tools: Vec<rmcp::model::Tool>,
        resources: Vec<rmcp::model::Resource>,
        prompts: Vec<rmcp::model::Prompt>,
        _server_capabilities: Option<rmcp::model::ServerCapabilities>,
        database: &Database,
    ) -> DiscoveryResult<ServerCapabilities> {
        // Convert tools
        let tool_infos: Vec<ToolInfo> = tools
            .into_iter()
            .map(|tool| ToolInfo {
                name: tool.name.to_string(),
                description: tool.description.map(|d| d.to_string()),
                input_schema: serde_json::to_value(&tool.input_schema)
                    .map_err(|e| {
                        tracing::warn!(
                            "Failed to serialize tool input schema for '{}': {}",
                            tool.name,
                            e
                        );
                        e
                    })
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
                annotations: tool.annotations.map(|a| {
                    serde_json::to_value(&a)
                        .map_err(|e| {
                            tracing::warn!(
                                "Failed to serialize tool annotations for '{}': {}",
                                tool.name,
                                e
                            );
                            e
                        })
                        .unwrap_or(serde_json::Value::Null)
                }),
            })
            .collect();

        // Convert resources
        let resource_infos: Vec<ResourceInfo> = resources
            .into_iter()
            .map(|resource| ResourceInfo {
                uri: resource.uri.clone(),
                name: Some(resource.name.clone()),
                description: resource.description.clone(),
                mime_type: resource.mime_type.clone(),
                annotations: resource.annotations.clone().map(|a| {
                    serde_json::to_value(&a)
                        .map_err(|e| {
                            tracing::warn!(
                                "Failed to serialize resource annotations for '{}': {}",
                                resource.uri,
                                e
                            );
                            e
                        })
                        .unwrap_or(serde_json::Value::Null)
                }),
            })
            .collect();

        // Convert prompts
        let prompt_infos: Vec<PromptInfo> = prompts
            .into_iter()
            .map(|prompt| {
                let arguments = prompt
                    .arguments
                    .unwrap_or_default()
                    .into_iter()
                    .map(|arg| PromptArgument {
                        name: arg.name,
                        description: arg.description,
                        required: arg.required.unwrap_or(false),
                    })
                    .collect();

                PromptInfo {
                    name: prompt.name,
                    description: prompt.description,
                    arguments,
                    annotations: None, // rmcp::model::Prompt doesn't have annotations field
                }
            })
            .collect();

        let metadata = CapabilitiesMetadata {
            last_updated: std::time::SystemTime::now(),
            version: "1.0".to_string(),
            ttl: Duration::from_secs(300),
            protocol_version: Some("2024-11-05".to_string()), // Default MCP protocol version
        };

        // Get actual server name from database
        let server_name = match self.get_server_from_database(server_id, database).await {
            Ok(server) => server.name,
            Err(_) => {
                tracing::warn!(
                    "Failed to get server name for {}, using server_id as fallback",
                    server_id
                );
                server_id.to_string()
            }
        };

        Ok(ServerCapabilities {
            server_id: server_id.to_string(),
            server_name,
            metadata,
            tools: tool_infos,
            resources: resource_infos,
            prompts: prompt_infos,
            resource_templates: Vec::new(),
        })
    }
}

impl Default for McpDiscoveryClient {
    fn default() -> Self {
        Self::new()
    }
}
