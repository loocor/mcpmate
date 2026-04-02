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
    common::profile::ProfileType,
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

    fn is_switchable_shared_profile(profile_type: &ProfileType) -> bool {
        matches!(profile_type, ProfileType::Shared)
    }

    async fn profile_list(&self) -> Result<CallToolResult> {
        let profiles = profile::get_all_profile(&self.database.pool)
            .await
            .context("Failed to list profile")?;

        let mut summaries = Vec::new();

        for prof in profiles {
            if !Self::is_switchable_shared_profile(&prof.profile_type) {
                continue;
            }

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

    async fn profile_preview(
        &self,
        profile_id: String,
    ) -> Result<CallToolResult> {
        let profile = profile::get_profile(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile")?
            .ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

        if !Self::is_switchable_shared_profile(&profile.profile_type) {
            return Err(anyhow::anyhow!(
                "Profile '{}' is not a shared profile and cannot be previewed for hosted profile selection",
                profile_id
            ));
        }

        let detail_components = load_profile_detail_components(&self.database.pool, &profile_id).await?;

        let preview = ProfilePreview {
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
            serde_json::to_string_pretty(&preview).context("Failed to serialize response")?,
        )]))
    }

    async fn profile_enable(
        &self,
        profile_id: String,
    ) -> Result<CallToolResult> {
        let profile = profile::get_profile(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile")?;

        let profile = profile.ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

        if profile.is_active {
            return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "message": format!("Profile '{}' is already active", profile.name),
                    "profile_id": profile_id,
                    "profile_name": profile.name,
                    "current_status": profile.is_active,
                    "refresh_required": false,
                }))
                .context("Failed to serialize response")?,
            )]));
        }

        profile::set_profile_active(&self.database.pool, &profile_id, true)
            .await
            .context("Failed to update profile status")?;

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "message": format!("Successfully enabled profile '{}'", profile.name),
                "profile_id": profile_id,
                "profile_name": profile.name,
                "new_status": true,
                "refresh_required": true,
                "refresh_hint": "If your client does not refresh tools automatically after MCP notifications, re-fetch tools/list now.",
            }))
            .context("Failed to serialize response")?,
        )]))
    }

    async fn profile_disable(
        &self,
        profile_id: String,
    ) -> Result<CallToolResult> {
        let profile = profile::get_profile(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile")?;

        let profile = profile.ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

        if !profile.is_active {
            return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "message": format!("Profile '{}' is already inactive", profile.name),
                    "profile_id": profile_id,
                    "profile_name": profile.name,
                    "current_status": profile.is_active,
                    "refresh_required": false,
                }))
                .context("Failed to serialize response")?,
            )]));
        }

        profile::set_profile_active(&self.database.pool, &profile_id, false)
            .await
            .context("Failed to update profile status")?;

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "message": format!("Successfully disabled profile '{}'", profile.name),
                "profile_id": profile_id,
                "profile_name": profile.name,
                "new_status": false,
                "refresh_required": true,
                "refresh_hint": "If your client does not refresh tools automatically after MCP notifications, re-fetch tools/list now.",
            }))
            .context("Failed to serialize response")?,
        )]))
    }

    async fn profile_activate_only(
        &self,
        profile_id: String,
    ) -> Result<CallToolResult> {
        let target = profile::get_profile(&self.database.pool, &profile_id)
            .await
            .context("Failed to get profile")?
            .ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

        let profiles = profile::get_all_profile(&self.database.pool)
            .await
            .context("Failed to list profiles")?;

        for current in profiles {
            let Some(current_id) = current.id.clone() else {
                continue;
            };

            if current_id == profile_id || current.is_default || !current.is_active {
                continue;
            }

            profile::set_profile_active(&self.database.pool, &current_id, false)
                .await
                .with_context(|| format!("Failed to disable profile '{}'", current.name))?;
        }

        if !target.is_active {
            profile::set_profile_active(&self.database.pool, &profile_id, true)
                .await
                .context("Failed to activate profile")?;
        }

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "message": format!("Switched active scene to profile '{}'", target.name),
                "profile_id": profile_id,
                "profile_name": target.name,
                "exclusive": true,
                "refresh_required": true,
                "refresh_hint": "If your client does not refresh tools automatically after MCP notifications, re-fetch tools/list now.",
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
                "mcpmate_profile_preview",
                "Preview a profile with lightweight capability details for one reusable scene.",
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
                "mcpmate_profile_enable",
                "Enable a profile. If the target profile is exclusive, other non-default profiles may be disabled by profile rules.",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_id": {
                                "type": "string",
                                "description": "Profile ID to enable"
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
                "mcpmate_profile_disable",
                "Disable a profile and remove it from the active working set.",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_id": {
                                "type": "string",
                                "description": "Profile ID to disable"
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
                "mcpmate_profile_activate_only",
                "Switch to a single shared scene by keeping only this profile active among non-default profiles.",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_id": {
                                "type": "string",
                                "description": "Profile ID to keep as the only active non-default profile"
                            }
                        },
                        "required": ["profile_id"]
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
            "mcpmate_profile_preview" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfileDetailsParams =
                    serde_json::from_value(args).context("Invalid parameters for profile_preview")?;
                self.profile_preview(params.profile_id).await
            }
            "mcpmate_profile_enable" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfileActionParams =
                    serde_json::from_value(args).context("Invalid parameters for profile_enable")?;
                self.profile_enable(params.profile_id).await
            }
            "mcpmate_profile_disable" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfileActionParams =
                    serde_json::from_value(args).context("Invalid parameters for profile_disable")?;
                self.profile_disable(params.profile_id).await
            }
            "mcpmate_profile_activate_only" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfileActionParams =
                    serde_json::from_value(args).context("Invalid parameters for profile_activate_only")?;
                self.profile_activate_only(params.profile_id).await
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
struct ProfileActionParams {
    profile_id: String,
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
struct ProfilePreview {
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

#[cfg(test)]
mod tests {
    use super::ProfileService;
    use crate::common::profile::ProfileType;

    #[test]
    fn switchable_profiles_only_include_shared_type() {
        assert!(ProfileService::is_switchable_shared_profile(&ProfileType::Shared));
        assert!(!ProfileService::is_switchable_shared_profile(&ProfileType::HostApp));
        assert!(!ProfileService::is_switchable_shared_profile(&ProfileType::Scenario));
    }
}
