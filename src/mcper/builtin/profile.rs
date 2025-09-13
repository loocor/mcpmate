//! Profile Service - Configuration Profile Management
//!
//! Protocol converter that transforms existing profile API capabilities
//! into MCP tool interfaces for client consumption.

use std::sync::Arc;

use anyhow::{Context, Result};
use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};
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

    async fn list_profile(&self) -> Result<CallToolResult> {
        // TODO: Token optimization - Current implementation balances information value
        // with token consumption by providing counts + samples instead of full details.
        // For full details, consider implementing a separate tool or parameter-based detail levels.

        let profile = profile::get_all_profile(&self.database.pool)
            .await
            .context("Failed to list profile")?;

        let mut response: Vec<ProfileInfo> = Vec::new();

        // For each profile, get information using existing APIs but optimize for token usage
        for prof in profile {
            if let Some(id) = prof.id {
                // Use existing APIs to get detailed information instead of counts
                let servers = get_profile_servers(&self.database.pool, &id)
                    .await
                    .context("Failed to get profile servers")?;

                let tools = get_profile_tools(&self.database.pool, &id)
                    .await
                    .context("Failed to get profile tools")?;

                let prompts = get_prompts_for_profile(&self.database.pool, &id)
                    .await
                    .context("Failed to get profile prompts")?;

                let resources = get_resources_for_profile(&self.database.pool, &id)
                    .await
                    .context("Failed to get profile resources")?;

                // Get server names for server summaries
                let server_summaries: Vec<ServerSummary> = {
                    let mut summaries = Vec::new();
                    for server in servers {
                        // Get server name from database
                        if let Ok(Some(server_model)) =
                            crate::config::server::crud::get_server_by_id(&self.database.pool, &server.server_id).await
                        {
                            summaries.push(ServerSummary {
                                name: server_model.name,
                                enabled: server.enabled,
                            });
                        }
                    }
                    summaries
                };

                // Create tool summaries with valuable information
                let tool_summaries: Vec<ToolSummary> = tools
                    .into_iter()
                    .map(|t| ToolSummary {
                        name: t.tool_name,
                        unique_name: t.unique_name,
                        description: t.description,
                        server_name: t.server_name,
                        enabled: t.enabled,
                    })
                    .collect();

                // Create prompt summaries
                let prompt_summaries: Vec<PromptSummary> = prompts
                    .into_iter()
                    .map(|p| PromptSummary {
                        name: p.prompt_name,
                        server_name: p.server_name,
                        enabled: p.enabled,
                    })
                    .collect();

                // Create resource summaries
                let resource_summaries: Vec<ResourceSummary> = resources
                    .into_iter()
                    .map(|r| ResourceSummary {
                        uri: r.resource_uri,
                        server_name: r.server_name,
                        enabled: r.enabled,
                    })
                    .collect();

                // Create simplified version to reduce token consumption
                // Get sample tool names (first 3) to give users a taste
                let sample_tools: Vec<String> = tool_summaries
                    .iter()
                    .take(3)
                    .map(|t| format!("{} ({})", t.name, t.unique_name))
                    .collect();

                // Get sample server names (first 3)
                let sample_servers: Vec<String> = server_summaries.iter().take(3).map(|s| s.name.clone()).collect();

                response.push(ProfileInfo {
                    id: id.clone(),
                    name: prof.name,
                    description: prof.description,
                    is_active: prof.is_active,
                    profile_type: prof.profile_type.to_string(),
                    server_count: server_summaries.len() as u32,
                    tool_count: tool_summaries.len() as u32,
                    prompt_count: prompt_summaries.len() as u32,
                    resource_count: resource_summaries.len() as u32,
                    sample_tools,
                    sample_servers,
                });
            } else {
                tracing::warn!("Found profile '{}' without ID, skipping", prof.name);
            }
        }

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&response).context("Failed to serialize response")?,
        )]))
    }

    async fn switch_profile(
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
                "mcpmate_list_profile",
                "List all available profile with their current status",
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
                "mcpmate_switch_profile",
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
        request: &CallToolRequestParam,
    ) -> Result<CallToolResult> {
        match request.name.as_ref() {
            "mcpmate_list_profile" => self.list_profile().await,
            "mcpmate_switch_profile" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: SwitchProfileParams =
                    serde_json::from_value(args).context("Invalid parameters for switch_profile")?;
                self.switch_profile(params.profile_id, params.activate).await
            }
            _ => Err(anyhow::anyhow!("Unknown tool: {}", request.name)),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SwitchProfileParams {
    profile_id: String,
    activate: bool,
}

/// TODO: Optimize token usage - current version provides detailed information
/// but may consume too many tokens. Consider implementing parameter-based
/// detail levels (basic/detailed) or pagination for large datasets.
#[derive(Debug, Serialize)]
struct ProfileInfo {
    id: String,
    name: String,
    description: Option<String>,
    is_active: bool,
    profile_type: String,
    // Simplified version to reduce token consumption
    // TODO: Make this configurable or provide both basic/detailed modes
    server_count: u32,
    tool_count: u32,
    prompt_count: u32,
    resource_count: u32,
    // Sample of available tools (first 3) to give users a taste
    sample_tools: Vec<String>,
    sample_servers: Vec<String>,
}

/// Detailed version (currently unused to save tokens)
// TODO: Implement detailed profile information structure for enhanced profile management
#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct DetailedProfileInfo {
    id: String,
    name: String,
    description: Option<String>,
    is_active: bool,
    profile_type: String,
    servers: Vec<ServerSummary>,
    tools: Vec<ToolSummary>,
    prompts: Vec<PromptSummary>,
    resources: Vec<ResourceSummary>,
}

#[derive(Debug, Serialize)]
struct ServerSummary {
    name: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ToolSummary {
    name: String,
    unique_name: String,
    description: Option<String>,
    server_name: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct PromptSummary {
    name: String,
    server_name: String,
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ResourceSummary {
    uri: String,
    server_name: String,
    enabled: bool,
}
