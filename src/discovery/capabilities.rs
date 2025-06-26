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

        // Generate summary information
        let summary = self.generate_summary(&tools, &resources, &prompts, &resource_templates);

        Ok(ProcessedCapabilities {
            server_id: capabilities.server_id.clone(),
            server_name: capabilities.server_name.clone(),
            tools,
            resources,
            prompts,
            resource_templates,
            summary,
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
            // Apply format-specific processing
            let processed_tool = match format {
                ResponseFormat::Compact => ProcessedToolInfo {
                    name: tool.name.clone(),
                    description: None, // Omit description in compact format
                    input_schema: serde_json::Value::Null, // Omit schema in compact format
                    annotations: None,
                    unique_name: self.get_tool_unique_name(server_id, &tool.name).await?,
                },
                ResponseFormat::Json => ProcessedToolInfo {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    input_schema: tool.input_schema.clone(),
                    annotations: None, // Omit annotations in standard format
                    unique_name: self.get_tool_unique_name(server_id, &tool.name).await?,
                },
                ResponseFormat::Detailed => ProcessedToolInfo {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    input_schema: tool.input_schema.clone(),
                    annotations: tool.annotations.clone(),
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
        _server_id: &str,
        format: ResponseFormat,
    ) -> DiscoveryResult<Vec<ProcessedResourceInfo>> {
        let mut processed_resources = Vec::new();

        for resource in resources {
            // Apply format-specific processing
            let processed_resource = match format {
                ResponseFormat::Compact => ProcessedResourceInfo {
                    uri: resource.uri.clone(),
                    name: None,        // Omit name in compact format
                    description: None, // Omit description in compact format
                    mime_type: None,   // Omit mime_type in compact format
                    annotations: None,
                },
                ResponseFormat::Json => ProcessedResourceInfo {
                    uri: resource.uri.clone(),
                    name: resource.name.clone(),
                    description: resource.description.clone(),
                    mime_type: resource.mime_type.clone(),
                    annotations: None, // Omit annotations in standard format
                },
                ResponseFormat::Detailed => ProcessedResourceInfo {
                    uri: resource.uri.clone(),
                    name: resource.name.clone(),
                    description: resource.description.clone(),
                    mime_type: resource.mime_type.clone(),
                    annotations: resource.annotations.clone(),
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
        _server_id: &str,
        format: ResponseFormat,
    ) -> DiscoveryResult<Vec<ProcessedPromptInfo>> {
        let mut processed_prompts = Vec::new();

        for prompt in prompts {
            // Apply format-specific processing
            let processed_prompt = match format {
                ResponseFormat::Compact => ProcessedPromptInfo {
                    name: prompt.name.clone(),
                    description: None,     // Omit description in compact format
                    arguments: Vec::new(), // Omit arguments in compact format
                    annotations: None,
                },
                ResponseFormat::Json => ProcessedPromptInfo {
                    name: prompt.name.clone(),
                    description: prompt.description.clone(),
                    arguments: prompt.arguments.clone(),
                    annotations: None, // Omit annotations in standard format
                },
                ResponseFormat::Detailed => ProcessedPromptInfo {
                    name: prompt.name.clone(),
                    description: prompt.description.clone(),
                    arguments: prompt.arguments.clone(),
                    annotations: prompt.annotations.clone(),
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

    /// Generate summary information from processed capabilities
    fn generate_summary(
        &self,
        tools: &[ProcessedToolInfo],
        resources: &[ProcessedResourceInfo],
        prompts: &[ProcessedPromptInfo],
        resource_templates: &[ProcessedResourceTemplateInfo],
    ) -> CapabilitiesSummary {
        // Count total items (no enabled/disabled distinction in Discovery API)
        let total_tools = tools.len();
        let total_resources = resources.len();
        let total_prompts = prompts.len();

        // Analyze MIME types
        let mut mime_types = std::collections::HashMap::new();
        for resource in resources {
            if let Some(mime_type) = &resource.mime_type {
                *mime_types.entry(mime_type.clone()).or_insert(0) += 1;
            }
        }

        // Check for complex prompts (with arguments)
        let has_complex_prompts = prompts.iter().any(|p| !p.arguments.is_empty());

        // Check for dynamic resources (has templates)
        let has_dynamic_resources = !resource_templates.is_empty();

        CapabilitiesSummary {
            total_tools,
            total_resources,
            total_prompts,
            total_resource_templates: resource_templates.len(),
            mime_types,
            has_complex_prompts,
            has_dynamic_resources,
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
    /// Processed tools
    pub tools: Vec<ProcessedToolInfo>,
    /// Processed resources
    pub resources: Vec<ProcessedResourceInfo>,
    /// Processed prompts
    pub prompts: Vec<ProcessedPromptInfo>,
    /// Processed resource templates
    pub resource_templates: Vec<ProcessedResourceTemplateInfo>,
    /// Summary information
    pub summary: CapabilitiesSummary,
}

/// Capabilities summary information
#[derive(Debug, Clone, serde::Serialize)]
pub struct CapabilitiesSummary {
    /// Total number of tools
    pub total_tools: usize,
    /// Total number of resources
    pub total_resources: usize,
    /// Total number of prompts
    pub total_prompts: usize,
    /// Total number of resource templates
    pub total_resource_templates: usize,
    /// MIME type distribution for resources
    pub mime_types: std::collections::HashMap<String, usize>,
    /// Whether server has complex prompts (with arguments)
    pub has_complex_prompts: bool,
    /// Whether server supports dynamic resources
    pub has_dynamic_resources: bool,
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
