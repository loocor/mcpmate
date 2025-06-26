// Capabilities Processing and Conversion
// Handles capability data processing, filtering, and format conversion

use super::types::{
    DiscoveryError, DiscoveryResult, PromptArgument, PromptInfo, ResourceInfo,
    ResourceTemplateInfo, ResponseFormat, ServerCapabilities, ToolInfo,
};
use crate::config::database::Database;

/// Capabilities processor for data transformation and filtering
pub struct CapabilitiesProcessor {
    /// Database connection for filtering enabled capabilities
    database: Option<Database>,
}

impl CapabilitiesProcessor {
    /// Create new capabilities processor
    pub fn new(database: Option<Database>) -> Self {
        Self { database }
    }

    /// Process and filter server capabilities based on configuration
    pub async fn process_capabilities(
        &self,
        capabilities: &ServerCapabilities,
        format: ResponseFormat,
    ) -> DiscoveryResult<ProcessedCapabilities> {
        let tools = self
            .process_tools(&capabilities.tools, &capabilities.server_id, format)
            .await?;
        let resources = self
            .process_resources(&capabilities.resources, &capabilities.server_id, format)
            .await?;
        let prompts = self
            .process_prompts(&capabilities.prompts, &capabilities.server_id, format)
            .await?;
        let resource_templates = self
            .process_resource_templates(&capabilities.resource_templates, format)
            .await?;

        Ok(ProcessedCapabilities {
            server_id: capabilities.server_id.clone(),
            server_name: capabilities.server_name.clone(),
            metadata: capabilities.metadata.clone(),
            tools,
            resources,
            prompts,
            resource_templates,
        })
    }

    /// Process tools with filtering and format conversion
    async fn process_tools(
        &self,
        tools: &[ToolInfo],
        server_id: &str,
        format: ResponseFormat,
    ) -> DiscoveryResult<Vec<ProcessedToolInfo>> {
        let mut processed_tools = Vec::new();

        for tool in tools {
            // Check if tool is enabled (if database is available)
            let enabled = if let Some(db) = &self.database {
                self.is_tool_enabled(db, server_id, &tool.name).await?
            } else {
                true // Default to enabled if no database
            };

            // Apply format-specific processing
            let processed_tool = match format {
                ResponseFormat::Compact => ProcessedToolInfo {
                    name: tool.name.clone(),
                    description: None, // Omit description in compact format
                    input_schema: serde_json::Value::Null, // Omit schema in compact format
                    annotations: None,
                    enabled,
                    unique_name: self.get_tool_unique_name(server_id, &tool.name).await?,
                },
                ResponseFormat::Json => ProcessedToolInfo {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    input_schema: tool.input_schema.clone(),
                    annotations: None, // Omit annotations in standard format
                    enabled,
                    unique_name: self.get_tool_unique_name(server_id, &tool.name).await?,
                },
                ResponseFormat::Detailed => ProcessedToolInfo {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    input_schema: tool.input_schema.clone(),
                    annotations: tool.annotations.clone(),
                    enabled,
                    unique_name: self.get_tool_unique_name(server_id, &tool.name).await?,
                },
            };

            processed_tools.push(processed_tool);
        }

        Ok(processed_tools)
    }

    /// Process resources with filtering and format conversion
    async fn process_resources(
        &self,
        resources: &[ResourceInfo],
        server_id: &str,
        format: ResponseFormat,
    ) -> DiscoveryResult<Vec<ProcessedResourceInfo>> {
        let mut processed_resources = Vec::new();

        for resource in resources {
            // Check if resource is enabled (if database is available)
            let enabled = if let Some(db) = &self.database {
                self.is_resource_enabled(db, server_id, &resource.uri)
                    .await?
            } else {
                true // Default to enabled if no database
            };

            // Apply format-specific processing
            let processed_resource = match format {
                ResponseFormat::Compact => ProcessedResourceInfo {
                    uri: resource.uri.clone(),
                    name: None,        // Omit name in compact format
                    description: None, // Omit description in compact format
                    mime_type: None,   // Omit mime_type in compact format
                    annotations: None,
                    enabled,
                },
                ResponseFormat::Json => ProcessedResourceInfo {
                    uri: resource.uri.clone(),
                    name: resource.name.clone(),
                    description: resource.description.clone(),
                    mime_type: resource.mime_type.clone(),
                    annotations: None, // Omit annotations in standard format
                    enabled,
                },
                ResponseFormat::Detailed => ProcessedResourceInfo {
                    uri: resource.uri.clone(),
                    name: resource.name.clone(),
                    description: resource.description.clone(),
                    mime_type: resource.mime_type.clone(),
                    annotations: resource.annotations.clone(),
                    enabled,
                },
            };

            processed_resources.push(processed_resource);
        }

        Ok(processed_resources)
    }

    /// Process prompts with filtering and format conversion
    async fn process_prompts(
        &self,
        prompts: &[PromptInfo],
        server_id: &str,
        format: ResponseFormat,
    ) -> DiscoveryResult<Vec<ProcessedPromptInfo>> {
        let mut processed_prompts = Vec::new();

        for prompt in prompts {
            // Check if prompt is enabled (if database is available)
            let enabled = if let Some(db) = &self.database {
                self.is_prompt_enabled(db, server_id, &prompt.name).await?
            } else {
                true // Default to enabled if no database
            };

            // Apply format-specific processing
            let processed_prompt = match format {
                ResponseFormat::Compact => ProcessedPromptInfo {
                    name: prompt.name.clone(),
                    description: None,     // Omit description in compact format
                    arguments: Vec::new(), // Omit arguments in compact format
                    annotations: None,
                    enabled,
                },
                ResponseFormat::Json => ProcessedPromptInfo {
                    name: prompt.name.clone(),
                    description: prompt.description.clone(),
                    arguments: prompt.arguments.clone(),
                    annotations: None, // Omit annotations in standard format
                    enabled,
                },
                ResponseFormat::Detailed => ProcessedPromptInfo {
                    name: prompt.name.clone(),
                    description: prompt.description.clone(),
                    arguments: prompt.arguments.clone(),
                    annotations: prompt.annotations.clone(),
                    enabled,
                },
            };

            processed_prompts.push(processed_prompt);
        }

        Ok(processed_prompts)
    }

    /// Process resource templates with format conversion
    async fn process_resource_templates(
        &self,
        resource_templates: &[ResourceTemplateInfo],
        format: ResponseFormat,
    ) -> DiscoveryResult<Vec<ProcessedResourceTemplateInfo>> {
        let mut processed_templates = Vec::new();

        for template in resource_templates {
            // Apply format-specific processing
            let processed_template = match format {
                ResponseFormat::Compact => ProcessedResourceTemplateInfo {
                    uri_template: template.uri_template.clone(),
                    name: None,        // Omit name in compact format
                    description: None, // Omit description in compact format
                    mime_type: None,   // Omit mime_type in compact format
                    annotations: None,
                },
                ResponseFormat::Json => ProcessedResourceTemplateInfo {
                    uri_template: template.uri_template.clone(),
                    name: template.name.clone(),
                    description: template.description.clone(),
                    mime_type: template.mime_type.clone(),
                    annotations: None, // Omit annotations in standard format
                },
                ResponseFormat::Detailed => ProcessedResourceTemplateInfo {
                    uri_template: template.uri_template.clone(),
                    name: template.name.clone(),
                    description: template.description.clone(),
                    mime_type: template.mime_type.clone(),
                    annotations: template.annotations.clone(),
                },
            };

            processed_templates.push(processed_template);
        }

        Ok(processed_templates)
    }

    /// Check if tool is enabled in configuration
    async fn is_tool_enabled(
        &self,
        database: &Database,
        server_id: &str,
        tool_name: &str,
    ) -> DiscoveryResult<bool> {
        // Get server name from server_id
        let server = crate::config::server::get_server(&database.pool, server_id)
            .await
            .map_err(|e| DiscoveryError::DatabaseError(e.to_string()))?;

        if let Some(server) = server {
            // Use existing tool status checking logic
            let enabled = crate::config::operations::tool::is_tool_enabled(
                &database.pool,
                &server.name,
                tool_name,
            )
            .await
            .unwrap_or(true); // Default to enabled if check fails

            Ok(enabled)
        } else {
            // Server not found, default to enabled
            Ok(true)
        }
    }

    /// Check if resource is enabled in configuration
    async fn is_resource_enabled(
        &self,
        database: &Database,
        server_id: &str,
        resource_uri: &str,
    ) -> DiscoveryResult<bool> {
        match crate::core::protocol::resource::status::is_resource_enabled(
            &database.pool,
            server_id,
            resource_uri,
        )
        .await
        {
            Ok(enabled) => Ok(enabled),
            Err(_) => Ok(true), // Default to enabled if status check fails
        }
    }

    /// Check if prompt is enabled in configuration
    async fn is_prompt_enabled(
        &self,
        database: &Database,
        server_id: &str,
        prompt_name: &str,
    ) -> DiscoveryResult<bool> {
        // Get server name from server_id
        let server = crate::config::server::get_server(&database.pool, server_id)
            .await
            .map_err(|e| DiscoveryError::DatabaseError(e.to_string()))?;

        if let Some(server) = server {
            // Use existing prompt status checking logic
            let enabled = crate::core::protocol::prompt::status::is_prompt_enabled(
                &database.pool,
                &server.name,
                prompt_name,
            )
            .await
            .unwrap_or(true); // Default to enabled if check fails

            Ok(enabled)
        } else {
            // Server not found, default to enabled
            Ok(true)
        }
    }

    /// Get unique tool name from configuration
    async fn get_tool_unique_name(
        &self,
        server_id: &str,
        tool_name: &str,
    ) -> DiscoveryResult<Option<String>> {
        if let Some(database) = &self.database {
            // Get server name from server_id
            let server = crate::config::server::get_server(&database.pool, server_id)
                .await
                .map_err(|e| DiscoveryError::DatabaseError(e.to_string()))?;

            if let Some(server) = server {
                // Query server_tools table for unique name
                let unique_name = sqlx::query_scalar::<_, String>(
                    r#"
                    SELECT unique_name
                    FROM server_tools
                    WHERE server_name = ? AND tool_name = ?
                    "#,
                )
                .bind(&server.name)
                .bind(tool_name)
                .fetch_optional(&database.pool)
                .await
                .map_err(|e| DiscoveryError::DatabaseError(e.to_string()))?;

                Ok(unique_name)
            } else {
                Ok(None)
            }
        } else {
            // No database available, return None
            Ok(None)
        }
    }

    /// Convert to enabled-only list for compatibility
    pub fn to_enabled_tools(processed_tools: &[ProcessedToolInfo]) -> Vec<&ProcessedToolInfo> {
        processed_tools.iter().filter(|t| t.enabled).collect()
    }

    /// Convert to enabled-only list for resources
    pub fn to_enabled_resources(
        processed_resources: &[ProcessedResourceInfo]
    ) -> Vec<&ProcessedResourceInfo> {
        processed_resources.iter().filter(|r| r.enabled).collect()
    }

    /// Convert to enabled-only list for prompts
    pub fn to_enabled_prompts(
        processed_prompts: &[ProcessedPromptInfo]
    ) -> Vec<&ProcessedPromptInfo> {
        processed_prompts.iter().filter(|p| p.enabled).collect()
    }

    /// Filter capabilities by enabled status
    pub fn filter_enabled_only(capabilities: &ProcessedCapabilities) -> ProcessedCapabilities {
        ProcessedCapabilities {
            server_id: capabilities.server_id.clone(),
            server_name: capabilities.server_name.clone(),
            metadata: capabilities.metadata.clone(),
            tools: capabilities
                .tools
                .iter()
                .filter(|t| t.enabled)
                .cloned()
                .collect(),
            resources: capabilities
                .resources
                .iter()
                .filter(|r| r.enabled)
                .cloned()
                .collect(),
            prompts: capabilities
                .prompts
                .iter()
                .filter(|p| p.enabled)
                .cloned()
                .collect(),
            resource_templates: capabilities.resource_templates.clone(), // Templates don't have enabled status
        }
    }
}

/// Processed capabilities with additional metadata
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessedCapabilities {
    /// Server identifier
    pub server_id: String,
    /// Server name
    pub server_name: String,
    /// Capabilities metadata
    pub metadata: super::types::CapabilitiesMetadata,
    /// Processed tools
    pub tools: Vec<ProcessedToolInfo>,
    /// Processed resources
    pub resources: Vec<ProcessedResourceInfo>,
    /// Processed prompts
    pub prompts: Vec<ProcessedPromptInfo>,
    /// Processed resource templates
    pub resource_templates: Vec<ProcessedResourceTemplateInfo>,
}

/// Processed tool information with additional metadata
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessedToolInfo {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// Input schema
    pub input_schema: serde_json::Value,
    /// Tool annotations
    pub annotations: Option<serde_json::Value>,
    /// Whether tool is enabled
    pub enabled: bool,
    /// Unique name in configuration
    pub unique_name: Option<String>,
}

/// Processed resource information with additional metadata
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessedResourceInfo {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: Option<String>,
    /// Resource description
    pub description: Option<String>,
    /// MIME type
    pub mime_type: Option<String>,
    /// Resource annotations
    pub annotations: Option<serde_json::Value>,
    /// Whether resource is enabled
    pub enabled: bool,
}

/// Processed prompt information with additional metadata
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessedPromptInfo {
    /// Prompt name
    pub name: String,
    /// Prompt description
    pub description: Option<String>,
    /// Prompt arguments
    pub arguments: Vec<PromptArgument>,
    /// Prompt annotations
    pub annotations: Option<serde_json::Value>,
    /// Whether prompt is enabled
    pub enabled: bool,
}

/// Processed resource template information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessedResourceTemplateInfo {
    /// Template URI pattern
    pub uri_template: String,
    /// Template name
    pub name: Option<String>,
    /// Template description
    pub description: Option<String>,
    /// MIME type
    pub mime_type: Option<String>,
    /// Template annotations
    pub annotations: Option<serde_json::Value>,
}
