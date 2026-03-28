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
        profile::{self},
    },
    core::pool::UpstreamConnectionPool,
};

use crate::mcper::builtin::{
    helpers::{load_profile_capability_counts, load_profile_detail_components},
    registry::BuiltinService,
    types::{PromptDetail, ResourceDetail, ServerDetail, ToolDetail},
};

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

            let counts = load_profile_capability_counts(&self.database.pool, &profile_id).await?;

            summaries.push(ProfileSummary {
                id: profile_id,
                name: prof.name.clone(),
                description: prof.description.clone(),
                is_active: prof.is_active,
                profile_type: prof.profile_type.to_string(),
                server_count: counts.server_count,
                tool_count: counts.tool_count,
                prompt_count: counts.prompt_count,
                resource_count: counts.resource_count,
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

        let detail_components = load_profile_detail_components(&self.database.pool, &profile_id).await?;

        let details = ProfileDetails {
            id: profile_id.clone(),
            name: profile.name,
            description: profile.description,
            is_active: profile.is_active,
            profile_type: profile.profile_type.to_string(),
            servers: detail_components.servers,
            tools: detail_components.tools,
            prompts: detail_components.prompts,
            resources: detail_components.resources,
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
                "List profiles with capability counts",
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
                "Get profile details: servers, tools, prompts, resources",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_id": {
                                "type": "string",
                                "description": "Profile ID to inspect"
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
                                "description": "Profile ID to switch"
                            },
                            "activate": {
                                "type": "boolean",
                                "description": "Activate (true) or deactivate (false)"
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
