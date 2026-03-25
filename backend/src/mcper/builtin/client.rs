use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use rmcp::model::{CallToolRequestParams, CallToolResult, Tool};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    clients::{models::CapabilitySource, service::ClientConfigService},
    config::{
        database::Database,
        profile::{self},
    },
    core::pool::UpstreamConnectionPool,
};

use super::{
    helpers::{load_profile_capability_counts, load_profile_detail_components},
    registry::BuiltinService,
    types::{PromptDetail, ResourceDetail, ServerDetail, ToolDetail},
};

#[derive(Debug, Clone)]
pub struct ClientBuiltinContext {
    pub client_id: String,
    pub capability_source: CapabilitySource,
    pub selected_profile_ids: Vec<String>,
    pub custom_profile_id: Option<String>,
}

pub struct ClientService {
    database: Arc<Database>,
    _connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    client_config_service: Arc<ClientConfigService>,
}

impl ClientService {
    pub fn new(
        database: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
        client_config_service: Arc<ClientConfigService>,
    ) -> Self {
        Self {
            database,
            _connection_pool: connection_pool,
            client_config_service,
        }
    }

    async fn client_configuration_get(
        &self,
        context: &ClientBuiltinContext,
    ) -> Result<CallToolResult> {
        let result = ClientConfigurationResponse {
            client_id: context.client_id.clone(),
            capability_source: context.capability_source.as_str().to_string(),
            selected_profile_ids: if matches!(context.capability_source, CapabilitySource::Profiles) {
                Some(context.selected_profile_ids.clone())
            } else {
                None
            },
            custom_profile_id: if matches!(context.capability_source, CapabilitySource::Custom) {
                context.custom_profile_id.clone()
            } else {
                None
            },
        };

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&result)
                .context("Failed to serialize client configuration response")?,
        )]))
    }

    async fn client_profiles_list(
        &self,
        context: &ClientBuiltinContext,
    ) -> Result<CallToolResult> {
        if !matches!(context.capability_source, CapabilitySource::Profiles) {
            return Err(anyhow!(
                "client_profiles_list is only available for clients using 'profiles' capability source. Current source: {}",
                context.capability_source.as_str()
            ));
        }

        let profiles = profile::get_all_profile(&self.database.pool)
            .await
            .context("Failed to list profiles")?;

        let shared_profiles: Vec<_> = profiles
            .into_iter()
            .filter(|p| p.profile_type == crate::common::profile::ProfileType::Shared)
            .collect();

        let mut summaries = Vec::new();

        for prof in shared_profiles {
            let Some(profile_id) = prof.id.clone() else {
                continue;
            };

            let counts = load_profile_capability_counts(&self.database.pool, &profile_id).await?;

            let is_selected = context.selected_profile_ids.contains(&profile_id);

            summaries.push(ProfileSummary {
                id: profile_id,
                name: prof.name.clone(),
                description: prof.description.clone(),
                is_active: prof.is_active,
                is_selected,
                server_count: counts.server_count,
                tool_count: counts.tool_count,
                prompt_count: counts.prompt_count,
                resource_count: counts.resource_count,
            });
        }

        summaries.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&summaries)
                .context("Failed to serialize profiles response")?,
        )]))
    }

    async fn client_profiles_select(
        &self,
        context: &ClientBuiltinContext,
        profile_ids: Vec<String>,
    ) -> Result<CallToolResult> {
        if !matches!(context.capability_source, CapabilitySource::Profiles) {
            return Err(anyhow!(
                "client_profiles_select is only available for clients using 'profiles' capability source. Current source: {}",
                context.capability_source.as_str()
            ));
        }

        let config = self
            .client_config_service
            .update_capability_config_and_invalidate(
                &context.client_id,
                CapabilitySource::Profiles,
                profile_ids,
            )
            .await
            .map_err(|error| anyhow!(error.to_string()))?;

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "message": format!("Selected {} profile(s) for client '{}'", config.selected_profile_ids.len(), context.client_id),
                "client_id": context.client_id,
                "selected_profile_ids": config.selected_profile_ids,
            }))
            .context("Failed to serialize response")?,
        )]))
    }

    async fn client_custom_profile_details(
        &self,
        context: &ClientBuiltinContext,
    ) -> Result<CallToolResult> {
        if !matches!(context.capability_source, CapabilitySource::Custom) {
            return Err(anyhow!(
                "client_custom_profile_details is only available for clients using 'custom' capability source. Current source: {}",
                context.capability_source.as_str()
            ));
        }

        let Some(profile_id) = &context.custom_profile_id else {
            return Err(anyhow!(
                "Client '{}' does not have a custom profile provisioned. Save the capability config first to create one.",
                context.client_id
            ));
        };

        let profile = profile::get_profile(&self.database.pool, profile_id)
            .await
            .context("Failed to get custom profile")?
            .ok_or_else(|| anyhow!("Custom profile '{}' not found", profile_id))?;

        let detail_components = load_profile_detail_components(&self.database.pool, profile_id).await?;

        let details = CustomProfileDetails {
            client_id: context.client_id.clone(),
            profile_id: profile_id.clone(),
            profile_name: profile.name,
            description: profile.description,
            servers: detail_components.servers,
            tools: detail_components.tools,
            prompts: detail_components.prompts,
            resources: detail_components.resources,
        };

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&details)
                .context("Failed to serialize custom profile details")?,
        )]))
    }
}

#[async_trait::async_trait]
impl BuiltinService for ClientService {
    fn name(&self) -> &'static str {
        "mcpmate_client"
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "mcpmate_client_configuration_get",
                "Get the current client's capability configuration (capability source, selected profiles, custom profile ID). Available for clients using 'profiles' or 'custom' capability source.",
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
                "mcpmate_client_profiles_list",
                "List available shared profiles for selection. Only available for clients using 'profiles' capability source.",
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
                "mcpmate_client_profiles_select",
                "Select shared profiles for the current client. Only available for clients using 'profiles' capability source. This updates the client's selected_profile_ids and invalidates the visibility cache.",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_ids": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "List of shared profile IDs to select for this client"
                            }
                        },
                        "required": ["profile_ids"]
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            ),
            Tool::new(
                "mcpmate_client_custom_profile_details",
                "Get details of the client's custom profile. Only available for clients using 'custom' capability source. Returns servers, tools, prompts, and resources configured in the custom profile.",
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
        ]
    }

    async fn call_tool(
        &self,
        request: &CallToolRequestParams,
    ) -> Result<CallToolResult> {
        match request.name.as_ref() {
            "mcpmate_client_configuration_get"
            | "mcpmate_client_profiles_list"
            | "mcpmate_client_profiles_select"
            | "mcpmate_client_custom_profile_details" => {
                Err(anyhow!(
                    "Client-aware tool '{}' requires client context. Use call_tool_with_context instead.",
                    request.name
                ))
            }
            _ => Err(anyhow!("Unknown tool: {}", request.name)),
        }
    }

    async fn call_tool_with_context(
        &self,
        request: &CallToolRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Result<CallToolResult> {
        match request.name.as_ref() {
            "mcpmate_client_configuration_get" => {
                let ctx = context.ok_or_else(|| anyhow!("Client context required for mcpmate_client_configuration_get"))?;
                self.client_configuration_get(ctx).await
            }
            "mcpmate_client_profiles_list" => {
                let ctx = context.ok_or_else(|| anyhow!("Client context required for mcpmate_client_profiles_list"))?;
                self.client_profiles_list(ctx).await
            }
            "mcpmate_client_profiles_select" => {
                let ctx = context.ok_or_else(|| anyhow!("Client context required for mcpmate_client_profiles_select"))?;
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfilesSelectParams =
                    serde_json::from_value(args).context("Invalid parameters for profiles_select")?;
                self.client_profiles_select(ctx, params.profile_ids).await
            }
            "mcpmate_client_custom_profile_details" => {
                let ctx = context.ok_or_else(|| anyhow!("Client context required for mcpmate_client_custom_profile_details"))?;
                self.client_custom_profile_details(ctx).await
            }
            _ => Err(anyhow!("Unknown tool: {}", request.name)),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ProfilesSelectParams {
    profile_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ClientConfigurationResponse {
    client_id: String,
    capability_source: String,
    selected_profile_ids: Option<Vec<String>>,
    custom_profile_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProfileSummary {
    id: String,
    name: String,
    description: Option<String>,
    is_active: bool,
    is_selected: bool,
    server_count: u32,
    tool_count: u32,
    prompt_count: u32,
    resource_count: u32,
}

#[derive(Debug, Serialize)]
struct CustomProfileDetails {
    client_id: String,
    profile_id: String,
    profile_name: String,
    description: Option<String>,
    servers: Vec<ServerDetail>,
    tools: Vec<ToolDetail>,
    prompts: Vec<PromptDetail>,
    resources: Vec<ResourceDetail>,
}
