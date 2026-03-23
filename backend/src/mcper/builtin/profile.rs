//! Profile Service - Configuration Profile Management
//!
//! Protocol converter that transforms existing profile API capabilities
//! into MCP tool interfaces for client consumption.

use std::sync::Arc;

use anyhow::{Context, Result};
use rmcp::model::{CallToolRequestParams, CallToolResult, Tool};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    config::{
        database::Database,
        profile::{self, get_profile_servers, get_profile_tools, get_prompts_for_profile, get_resources_for_profile},
    },
    core::pool::UpstreamConnectionPool,
};

use crate::mcper::builtin::registry::BuiltinService;

/// Service providing profile management via MCP tools
pub struct ProfileService {
    database: Arc<Database>,
    _connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
}

impl ProfileService {
    pub fn new(
        database: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        Self {
            database,
            _connection_pool: connection_pool,
        }
    }

    async fn profile_list(&self) -> Result<CallToolResult> {
        let profiles = profile::get_all_profile(&self.database.pool)
            .await
            .context("Failed to list profile")?;

        let mut summaries = Vec::new();

        for prof in profiles {
            let Some(profile_id) = prof.id.clone() else {
                tracing::warn!("Found profile '{}' without ID, skipping", prof.name);
                continue;
            };

            let servers = get_profile_servers(&self.database.pool, &profile_id)
                .await
                .context("Failed to get profile servers")?;

            let tools = get_profile_tools(&self.database.pool, &profile_id)
                .await
                .context("Failed to get profile tools")?;

            let prompts = get_prompts_for_profile(&self.database.pool, &profile_id)
                .await
                .context("Failed to get profile prompts")?;

            let resources = get_resources_for_profile(&self.database.pool, &profile_id)
                .await
                .context("Failed to get profile resources")?;

            summaries.push(ProfileSummary {
                id: profile_id,
                name: prof.name.clone(),
                description: prof.description.clone(),
                is_active: prof.is_active,
                profile_type: prof.profile_type.to_string(),
                server_count: servers.len() as u32,
                tool_count: tools.len() as u32,
                prompt_count: prompts.len() as u32,
                resource_count: resources.len() as u32,
            });
        }

        summaries.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&summaries).context("Failed to serialize response")?,
        )]))
    }

    async fn profile_details(
        &self,
        profile_id: String,
    ) -> Result<CallToolResult> {
        let profile = profile::get_profile(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile")?
            .ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

        let servers = get_profile_servers(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile servers")?;

        let tools = get_profile_tools(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile tools")?;

        let prompts = get_prompts_for_profile(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile prompts")?;

        let resources = get_resources_for_profile(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile resources")?;

        let mut server_details = Vec::new();
        for server in servers {
            let server_id = server.server_id.clone();
            let server_name = match crate::config::server::crud::get_server_by_id(&self.database.pool, &server_id).await
            {
                Ok(Some(server_model)) => server_model.name,
                Ok(None) => {
                    tracing::warn!("Server '{}' not found when listing profile '{}'", server_id, profile_id);
                    server_id.clone()
                }
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        server_id = %server_id,
                        profile_id = %profile_id,
                        "Failed to load server metadata, falling back to ID"
                    );
                    server_id.clone()
                }
            };

            server_details.push(ServerDetail {
                association_id: server.id.clone(),
                server_id,
                name: server_name,
                enabled: server.enabled,
            });
        }
        server_details.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.server_id.cmp(&b.server_id)));

        let mut tool_details: Vec<ToolDetail> = tools
            .into_iter()
            .map(|tool| ToolDetail {
                association_id: tool.id,
                server_tool_id: tool.server_tool_id,
                server_id: tool.server_id,
                server_name: tool.server_name,
                tool_name: tool.tool_name,
                unique_name: tool.unique_name,
                description: tool.description,
                enabled: tool.enabled,
            })
            .collect();
        tool_details.sort_by(|a, b| {
            a.server_name
                .cmp(&b.server_name)
                .then_with(|| a.tool_name.cmp(&b.tool_name))
                .then_with(|| a.unique_name.cmp(&b.unique_name))
        });

        let mut prompt_details: Vec<PromptDetail> = prompts
            .into_iter()
            .map(|prompt| PromptDetail {
                association_id: prompt.id,
                server_id: prompt.server_id,
                server_name: prompt.server_name,
                prompt_name: prompt.prompt_name,
                enabled: prompt.enabled,
            })
            .collect();
        prompt_details.sort_by(|a, b| {
            a.server_name
                .cmp(&b.server_name)
                .then_with(|| a.prompt_name.cmp(&b.prompt_name))
        });

        let mut resource_details: Vec<ResourceDetail> = resources
            .into_iter()
            .map(|resource| ResourceDetail {
                association_id: resource.id,
                server_id: resource.server_id,
                server_name: resource.server_name,
                resource_uri: resource.resource_uri,
                enabled: resource.enabled,
            })
            .collect();
        resource_details.sort_by(|a, b| {
            a.server_name
                .cmp(&b.server_name)
                .then_with(|| a.resource_uri.cmp(&b.resource_uri))
        });

        let details = ProfileDetails {
            id: profile_id.clone(),
            name: profile.name,
            description: profile.description,
            is_active: profile.is_active,
            profile_type: profile.profile_type.to_string(),
            servers: server_details,
            tools: tool_details,
            prompts: prompt_details,
            resources: resource_details,
        };

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&details).context("Failed to serialize response")?,
        )]))
    }

    async fn profile_switch(
        &self,
        profile_id: String,
        activate: bool,
    ) -> Result<CallToolResult> {
        let profile = profile::get_profile(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile")?;

        let profile = profile.ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

        if profile.is_active == activate {
            let status = if activate { "already active" } else { "already inactive" };
            return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "message": format!("Profile '{}' is {}", profile.name, status),
                    "profile_id": profile_id,
                    "profile_name": profile.name,
                    "current_status": profile.is_active,
                }))
                .context("Failed to serialize response")?,
            )]));
        }

        profile::set_profile_active(&self.database.pool, &profile_id, activate)
            .await
            .context("Failed to update profile status")?;

        // Publish event to trigger synchronization
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ProfileStatusChanged {
            profile_id: profile_id.clone(),
            enabled: activate,
        });

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "message": format!("Successfully {} profile '{}'",
                    if activate { "activated" } else { "deactivated" },
                    profile.name),
                "profile_id": profile_id,
                "profile_name": profile.name,
                "new_status": activate,
            }))
            .context("Failed to serialize response")?,
        )]))
    }
}

#[async_trait::async_trait]
impl BuiltinService for ProfileService {
    fn name(&self) -> &'static str {
        "mcpmate_profile"
    }

    fn tools(&self) -> Vec<rmcp::model::Tool> {
        vec![
            Tool::new(
                "mcpmate_profile_list",
                "List available profiles with counts for each capability type",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            ),
            Tool::new(
                "mcpmate_profile_details",
                "Get detailed servers, tools, prompts, and resources for a profile",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_id": {
                                "type": "string",
                                "description": "The ID of the profile to inspect"
                            }
                        },
                        "required": ["profile_id"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            ),
            Tool::new(
                "mcpmate_profile_switch",
                "Activate or deactivate a profile",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_id": {
                                "type": "string",
                                "description": "The ID of the profile to switch"
                            },
                            "activate": {
                                "type": "boolean",
                                "description": "Whether to activate (true) or deactivate (false) the profile"
                            }
                        },
                        "required": ["profile_id", "activate"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            ),
        ]
    }

    async fn call_tool(
        &self,
        request: &CallToolRequestParams,
    ) -> Result<CallToolResult> {
        match request.name.as_ref() {
            "mcpmate_profile_list" => self.profile_list().await,
            "mcpmate_profile_details" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfileDetailsParams =
                    serde_json::from_value(args).context("Invalid parameters for profile_details")?;
                self.profile_details(params.profile_id).await
            }
            "mcpmate_profile_switch" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfileSwitchParams =
                    serde_json::from_value(args).context("Invalid parameters for profile_switch")?;
                self.profile_switch(params.profile_id, params.activate).await
            }
            _ => Err(anyhow::anyhow!("Unknown tool: {}", request.name)),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ProfileDetailsParams {
    profile_id: String,
}

#[derive(Debug, Deserialize)]
struct ProfileSwitchParams {
    profile_id: String,
    activate: bool,
}

#[derive(Debug, Serialize)]
struct ProfileSummary {
    id: String,
    name: String,
    description: Option<String>,
    is_active: bool,
    profile_type: String,
    server_count: u32,
    tool_count: u32,
    prompt_count: u32,
    resource_count: u32,
}

#[derive(Debug, Serialize)]
struct ProfileDetails {
    id: String,
    name: String,
    description: Option<String>,
    is_active: bool,
    profile_type: String,
    servers: Vec<ServerDetail>,
    tools: Vec<ToolDetail>,
    prompts: Vec<PromptDetail>,
    resources: Vec<ResourceDetail>,
}

#[derive(Debug, Serialize)]
struct ServerDetail {
    association_id: Option<String>,
    server_id: String,
    name: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ToolDetail {
    association_id: String,
    server_tool_id: String,
    server_id: String,
    server_name: String,
    tool_name: String,
    unique_name: String,
    description: Option<String>,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct PromptDetail {
    association_id: Option<String>,
    server_id: String,
    server_name: String,
    prompt_name: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ResourceDetail {
    association_id: Option<String>,
    server_id: String,
    server_name: String,
    resource_uri: String,
    enabled: bool,
}
