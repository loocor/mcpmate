// Discovery aggregation handlers
// Provides aggregated views across all servers for tools, resources, and prompts

use axum::{Json, extract::State};
use std::sync::Arc;

use crate::{
    api::{handlers::ApiError, routes::AppState},
    config::server,
    discovery::types::DiscoveryParams,
};

/// List all tools from all servers (aggregated view)
pub async fn all_tools(
    State(state): State<Arc<AppState>>
) -> Result<Json<Vec<rmcp::model::Tool>>, ApiError> {
    // Get discovery service
    let discovery_service = match &state.discovery_service {
        Some(service) => service,
        None => {
            return Err(ApiError::InternalError(
                "Discovery service not available".to_string(),
            ));
        }
    };

    // Get database from state
    let database = match &state.database {
        Some(db) => db,
        None => {
            return Err(ApiError::InternalError(
                "Database not available".to_string(),
            ));
        }
    };

    // Get all servers from database
    let all_servers = server::get_all_servers(&database.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers: {}", e)))?;

    let mut mcp_tools = Vec::new();
    let params = DiscoveryParams::default();

    // Collect tools from all servers
    for server in all_servers {
        if let Some(server_id) = &server.id {
            match discovery_service
                .get_server_tools(server_id, params.clone())
                .await
            {
                Ok(tools_response) => {
                    for tool in tools_response {
                        if tool.enabled {
                            // Convert discovery ProcessedToolInfo to rmcp::model::Tool
                            let input_schema = match tool.input_schema {
                                serde_json::Value::Object(obj) => std::sync::Arc::new(obj),
                                _ => std::sync::Arc::new(serde_json::Map::new()),
                            };

                            let annotations = tool.annotations.and_then(|a| {
                                serde_json::from_value::<rmcp::model::ToolAnnotations>(a).ok()
                            });

                            let mcp_tool = rmcp::model::Tool {
                                name: tool.unique_name.unwrap_or(tool.name).into(),
                                description: tool.description.map(|d| d.into()),
                                input_schema,
                                annotations,
                            };
                            mcp_tools.push(mcp_tool);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get tools for server {}: {}", server_id, e);
                    // Continue with other servers instead of failing completely
                }
            }
        }
    }

    tracing::info!(
        "Returning {} tools from discovery aggregation endpoint",
        mcp_tools.len()
    );

    Ok(Json(mcp_tools))
}

/// List all resources from all servers (aggregated view)
pub async fn all_resources(
    State(state): State<Arc<AppState>>
) -> Result<Json<Vec<rmcp::model::Resource>>, ApiError> {
    // Get discovery service
    let discovery_service = match &state.discovery_service {
        Some(service) => service,
        None => {
            return Err(ApiError::InternalError(
                "Discovery service not available".to_string(),
            ));
        }
    };

    // Get database from state
    let database = match &state.database {
        Some(db) => db,
        None => {
            return Err(ApiError::InternalError(
                "Database not available".to_string(),
            ));
        }
    };

    // Get all servers from database
    let all_servers = server::get_all_servers(&database.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers: {}", e)))?;

    let mut mcp_resources = Vec::new();
    let params = DiscoveryParams::default();

    // Collect resources from all servers
    for server in all_servers {
        if let Some(server_id) = &server.id {
            match discovery_service
                .get_server_resources(server_id, params.clone())
                .await
            {
                Ok(resources_response) => {
                    for resource in resources_response {
                        if resource.enabled {
                            // Convert discovery ProcessedResourceInfo to rmcp::model::Resource
                            let raw_resource = rmcp::model::RawResource {
                                uri: resource.uri,
                                name: resource.name.unwrap_or_else(|| "Unknown".to_string()),
                                description: resource.description,
                                mime_type: resource.mime_type,
                                size: None,
                            };

                            let annotations = resource.annotations.and_then(|a| {
                                serde_json::from_value::<rmcp::model::Annotations>(a).ok()
                            });

                            let mcp_resource = rmcp::model::Resource {
                                raw: raw_resource,
                                annotations,
                            };
                            mcp_resources.push(mcp_resource);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get resources for server {}: {}", server_id, e);
                    // Continue with other servers instead of failing completely
                }
            }
        }
    }

    tracing::info!(
        "Returning {} resources from discovery aggregation endpoint",
        mcp_resources.len()
    );

    Ok(Json(mcp_resources))
}

/// List all prompts from all servers (aggregated view)
pub async fn all_prompts(
    State(state): State<Arc<AppState>>
) -> Result<Json<Vec<rmcp::model::Prompt>>, ApiError> {
    // Get discovery service
    let discovery_service = match &state.discovery_service {
        Some(service) => service,
        None => {
            return Err(ApiError::InternalError(
                "Discovery service not available".to_string(),
            ));
        }
    };

    // Get database from state
    let database = match &state.database {
        Some(db) => db,
        None => {
            return Err(ApiError::InternalError(
                "Database not available".to_string(),
            ));
        }
    };

    // Get all servers from database
    let all_servers = server::get_all_servers(&database.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers: {}", e)))?;

    let mut mcp_prompts = Vec::new();
    let params = DiscoveryParams::default();

    // Collect prompts from all servers
    for server in all_servers {
        if let Some(server_id) = &server.id {
            match discovery_service
                .get_server_prompts(server_id, params.clone())
                .await
            {
                Ok(prompts_response) => {
                    for prompt in prompts_response {
                        if prompt.enabled {
                            // Convert discovery ProcessedPromptInfo to rmcp::model::Prompt
                            let arguments = prompt
                                .arguments
                                .into_iter()
                                .map(|arg| rmcp::model::PromptArgument {
                                    name: arg.name,
                                    description: arg.description,
                                    required: Some(arg.required),
                                })
                                .collect();

                            let mcp_prompt = rmcp::model::Prompt {
                                name: prompt.name,
                                description: prompt.description,
                                arguments: Some(arguments),
                            };
                            mcp_prompts.push(mcp_prompt);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get prompts for server {}: {}", server_id, e);
                    // Continue with other servers instead of failing completely
                }
            }
        }
    }

    tracing::info!(
        "Returning {} prompts from discovery aggregation endpoint",
        mcp_prompts.len()
    );

    Ok(Json(mcp_prompts))
}

/// Aggregate all resource templates from all enabled servers
///
/// Returns a combined list of resource templates from all servers that are
/// currently enabled and accessible. Resource templates define URI patterns
/// for dynamic resource generation.
pub async fn all_resource_templates(
    State(state): State<Arc<AppState>>
) -> Result<Json<Vec<rmcp::model::ResourceTemplate>>, ApiError> {
    // Get discovery service
    let discovery_service = match &state.discovery_service {
        Some(service) => service,
        None => {
            return Err(ApiError::InternalError(
                "Discovery service not available".to_string(),
            ));
        }
    };

    // Get database from state
    let db = match state.http_proxy.as_ref().and_then(|p| p.database.clone()) {
        Some(db) => db,
        None => {
            return Err(ApiError::InternalError(
                "Database not available".to_string(),
            ));
        }
    };

    // Get all enabled servers
    let all_servers = server::get_all_servers(&db.pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get servers: {e}")))?;

    let mut mcp_resource_templates = Vec::new();
    let params = DiscoveryParams::default();

    // Collect resource templates from all servers
    for server in all_servers {
        if let Some(server_id) = &server.id {
            match discovery_service
                .get_server_resource_templates(server_id, params.clone())
                .await
            {
                Ok(templates_response) => {
                    for template in templates_response {
                        // Convert discovery ProcessedResourceTemplateInfo to rmcp::model::ResourceTemplate
                        let raw_template = rmcp::model::RawResourceTemplate {
                            uri_template: template.uri_template,
                            name: template.name.unwrap_or_else(|| "Unknown".to_string()),
                            description: template.description,
                            mime_type: template.mime_type,
                        };

                        let annotations = template.annotations.and_then(|a| {
                            serde_json::from_value::<rmcp::model::Annotations>(a).ok()
                        });

                        let mcp_template = rmcp::model::ResourceTemplate {
                            raw: raw_template,
                            annotations,
                        };
                        mcp_resource_templates.push(mcp_template);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to get resource templates for server {}: {}",
                        server_id,
                        e
                    );
                    // Continue with other servers instead of failing completely
                }
            }
        }
    }

    tracing::info!(
        "Returning {} resource templates from discovery aggregation endpoint",
        mcp_resource_templates.len()
    );

    Ok(Json(mcp_resource_templates))
}
