use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult, Prompt, PromptMessage,
    PromptMessageRole, Tool,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    clients::{
        models::{CapabilitySource, ClientCapabilityConfig},
        service::ClientConfigService,
    },
    config::{database::Database, profile},
    core::pool::UpstreamConnectionPool,
};

use super::{
    helpers::load_profile_detail_components,
    registry::BuiltinService,
    types::{PromptDetail, ResourceDetail, ServerDetail, ToolDetail},
};

#[derive(Debug, Clone)]
pub struct ClientBuiltinContext {
    pub client_id: String,
    pub session_id: Option<String>,
    pub config_mode: Option<String>,
    pub capability_source: CapabilitySource,
    pub selected_profile_ids: Vec<String>,
    pub custom_profile_id: Option<String>,
    pub smart_workspace: Option<ClientCapabilityConfig>,
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

    async fn scope_get(
        &self,
        context: &ClientBuiltinContext,
    ) -> Result<CallToolResult> {
        let result = ClientConfigurationResponse {
            client_id: context.client_id.clone(),
            config_mode: context.config_mode.clone().unwrap_or_else(|| "hosted".to_string()),
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
            serde_json::to_string_pretty(&result).context("Failed to serialize client configuration response")?,
        )]))
    }

    async fn scope_set(
        &self,
        context: &ClientBuiltinContext,
        profile_ids: Vec<String>,
    ) -> Result<CallToolResult> {
        if !matches!(context.capability_source, CapabilitySource::Profiles) {
            return Err(anyhow!(
                "scope_set is only available for clients using 'profiles' capability source. Current source: {}",
                context.capability_source.as_str()
            ));
        }

        let selected_profile_ids = if matches!(context.config_mode.as_deref(), Some("smart")) {
            profile_ids
        } else {
            self.client_config_service
                .update_capability_config_and_invalidate(&context.client_id, CapabilitySource::Profiles, profile_ids)
                .await
                .map_err(|error| anyhow!(error.to_string()))?
                .selected_profile_ids
        };

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                    "message": format!(
                        "Updated the client working set to {} selected profile(s) for '{}'",
                        selected_profile_ids.len(),
                        context.client_id
                    ),
                    "client_id": context.client_id,
                    "selected_profile_ids": selected_profile_ids,
                    "refresh_required": true,
                    "refresh_hint": "If your client does not refresh tools automatically after MCP notifications, re-fetch tools/list now.",
                }))
            .context("Failed to serialize response")?,
        )]))
    }

    async fn scope_add(
        &self,
        context: &ClientBuiltinContext,
        profile_ids: Vec<String>,
    ) -> Result<CallToolResult> {
        if !matches!(context.capability_source, CapabilitySource::Profiles) {
            return Err(anyhow!(
                "scope_add is only available for clients using 'profiles' capability source. Current source: {}",
                context.capability_source.as_str()
            ));
        }

        let mut merged = context.selected_profile_ids.clone();
        merged.extend(profile_ids);
        merged.sort();
        merged.dedup();

        let selected_profile_ids = if matches!(context.config_mode.as_deref(), Some("smart")) {
            merged
        } else {
            self.client_config_service
                .update_capability_config_and_invalidate(&context.client_id, CapabilitySource::Profiles, merged)
                .await
                .map_err(|error| anyhow!(error.to_string()))?
                .selected_profile_ids
        };

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                    "message": format!(
                        "Added profile(s) to the client working set. '{}' now has {} selected profile(s)",
                        context.client_id,
                        selected_profile_ids.len()
                    ),
                    "client_id": context.client_id,
                    "selected_profile_ids": selected_profile_ids,
                    "refresh_required": true,
                "refresh_hint": "If your client does not refresh tools automatically after MCP notifications, re-fetch tools/list now.",
            }))
            .context("Failed to serialize response")?,
        )]))
    }

    async fn scope_remove(
        &self,
        context: &ClientBuiltinContext,
        profile_ids: Vec<String>,
    ) -> Result<CallToolResult> {
        if !matches!(context.capability_source, CapabilitySource::Profiles) {
            return Err(anyhow!(
                "scope_remove is only available for clients using 'profiles' capability source. Current source: {}",
                context.capability_source.as_str()
            ));
        }

        let remaining = context
            .selected_profile_ids
            .iter()
            .filter(|current| !profile_ids.contains(current))
            .cloned()
            .collect::<Vec<_>>();

        let selected_profile_ids = if matches!(context.config_mode.as_deref(), Some("smart")) {
            remaining
        } else {
            self.client_config_service
                .update_capability_config_and_invalidate(&context.client_id, CapabilitySource::Profiles, remaining)
                .await
                .map_err(|error| anyhow!(error.to_string()))?
                .selected_profile_ids
        };

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                    "message": format!(
                        "Removed profile(s) from the current working set without deleting the profile definitions. '{}' now has {} selected profile(s)",
                        context.client_id,
                        selected_profile_ids.len()
                    ),
                    "client_id": context.client_id,
                    "selected_profile_ids": selected_profile_ids,
                    "refresh_required": true,
                "refresh_hint": "If your client does not refresh tools automatically after MCP notifications, re-fetch tools/list now.",
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
            serde_json::to_string_pretty(&details).context("Failed to serialize custom profile details")?,
        )]))
    }

    fn smart_mode_guide_prompt() -> Prompt {
        Prompt::new(
            "mcpmate_smart_mode_guide",
            Some("Explain how Smart Mode works and when to use switch, add, remove, or custom state tools."),
            None,
        )
    }

    fn smart_mode_next_actions_prompt() -> Prompt {
        Prompt::new(
            "mcpmate_smart_mode_next_actions",
            Some("Summarize the current client working state and recommend the next Smart Mode tool to call."),
            None,
        )
    }

    fn build_smart_mode_guide(
        &self,
        context: &ClientBuiltinContext,
    ) -> GetPromptResult {
        let content = format!(
            concat!(
                "You are helping a user operate MCPMate Smart Mode for client '{client_id}'.\n\n",
                "Smart Mode concepts:\n",
                "1. Smart Mode is session-scoped and starts with builtin MCP control-plane tools only.\n",
                "2. There is no second-level selector in the UI for Smart Mode.\n",
                "3. Shared scenes are added to the current session working set through builtin tools.\n\n",
                "Current capability source: {capability_source}.\n",
                "Selected shared profiles: {selected_profiles}.\n",
                "Custom profile id: {custom_profile_id}.\n\n",
                "Tool guidance:\n",
                "- Use mcpmate_scope_set to switch to an exact working set, such as frontend only.\n",
                "- Use mcpmate_scope_add to add another shared scene, such as also enabling backend.\n",
                "- Use mcpmate_scope_remove to remove scenes from the current working set without deleting the profiles themselves.\n",
                "- Smart Mode changes stay inside the current MCP session and reset when the session ends.\n\n",
                "After any tool that changes capability visibility, if the client does not refresh tools automatically, ask it to re-fetch tools/list."
            ),
            client_id = context.client_id,
            capability_source = context.capability_source.as_str(),
            selected_profiles = if context.selected_profile_ids.is_empty() {
                "(none)".to_string()
            } else {
                context.selected_profile_ids.join(", ")
            },
            custom_profile_id = context
                .custom_profile_id
                .clone()
                .unwrap_or_else(|| "(none)".to_string()),
        );

        GetPromptResult::new(vec![PromptMessage::new_text(PromptMessageRole::User, content)])
            .with_description("Smart Mode guide for the current client")
    }

    fn build_smart_mode_next_actions(
        &self,
        context: &ClientBuiltinContext,
    ) -> GetPromptResult {
        let next_action = match context.capability_source {
            CapabilitySource::Activated => {
                "Use mcpmate_scope_set to choose an exact shared-scene working set for this Smart session."
            }
            CapabilitySource::Profiles => {
                "Use mcpmate_scope_set for an exact working set, mcpmate_scope_add to add scenes, or mcpmate_scope_remove to remove them from the current working set."
            }
            CapabilitySource::Custom => {
                "Smart Mode does not persist custom overlays. Prefer shared-scene working set tools for this session."
            }
        };

        let content = format!(
            concat!(
                "Client: {client_id}\n",
                "Capability source: {capability_source}\n",
                "Selected shared profiles: {selected_profiles}\n",
                "Custom profile id: {custom_profile_id}\n\n",
                "Recommended next action:\n",
                "{next_action}\n\n",
                "If the user says 'switch to frontend', prefer an exact replace operation.\n",
                "If the user says 'also enable backend', prefer an additive operation.\n",
                "If a tool changes visible capabilities, ask the client to re-fetch tools/list when auto-refresh is not reliable.\n",
                "Smart Mode resets when the MCP session ends; promote to Hosted if the user wants durable behavior."
            ),
            client_id = context.client_id,
            capability_source = context.capability_source.as_str(),
            selected_profiles = if context.selected_profile_ids.is_empty() {
                "(none)".to_string()
            } else {
                context.selected_profile_ids.join(", ")
            },
            custom_profile_id = context
                .custom_profile_id
                .clone()
                .unwrap_or_else(|| "(none)".to_string()),
            next_action = next_action,
        );

        GetPromptResult::new(vec![PromptMessage::new_text(PromptMessageRole::User, content)])
            .with_description("Recommended Smart Mode next actions for the current client")
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
                "mcpmate_scope_get",
                "Get the current working state for this client session, including mode, working-set source, selected shared profiles, and custom profile ID if present.",
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
                "mcpmate_scope_set",
                "Replace the current client working set with an exact list of shared profiles (profiles mode only). Use this to switch to a single scene or exact set.",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_ids": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Shared profile IDs to keep in the working set"
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
                "mcpmate_scope_add",
                "Add shared profiles to the current client working set without replacing the existing selection (profiles mode only).",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_ids": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Shared profile IDs to add to the working set"
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
                "mcpmate_scope_remove",
                "Remove shared profiles from the current working set without deleting the profile definitions themselves (profiles mode only).",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "profile_ids": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Shared profile IDs to remove from the working set"
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
                "Get custom profile details: servers, tools, prompts, resources (custom mode only)",
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

    fn prompts(&self) -> Vec<Prompt> {
        vec![Self::smart_mode_guide_prompt(), Self::smart_mode_next_actions_prompt()]
    }

    async fn call_tool(
        &self,
        request: &CallToolRequestParams,
    ) -> Result<CallToolResult> {
        match request.name.as_ref() {
            "mcpmate_scope_get"
            | "mcpmate_scope_set"
            | "mcpmate_scope_add"
            | "mcpmate_scope_remove"
            | "mcpmate_client_custom_profile_details" => Err(anyhow!(
                "Client-aware tool '{}' requires client context. Use call_tool_with_context instead.",
                request.name
            )),
            _ => Err(anyhow!("Unknown tool: {}", request.name)),
        }
    }

    async fn call_tool_with_context(
        &self,
        request: &CallToolRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Result<CallToolResult> {
        match request.name.as_ref() {
            "mcpmate_scope_get" => {
                let ctx = context.ok_or_else(|| anyhow!("Client context required for mcpmate_scope_get"))?;
                self.scope_get(ctx).await
            }
            "mcpmate_scope_set" => {
                let ctx = context.ok_or_else(|| anyhow!("Client context required for mcpmate_scope_set"))?;
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfilesSelectParams =
                    serde_json::from_value(args).context("Invalid parameters for scope_set")?;
                self.scope_set(ctx, params.profile_ids).await
            }
            "mcpmate_scope_add" => {
                let ctx = context.ok_or_else(|| anyhow!("Client context required for mcpmate_scope_add"))?;
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfilesSelectParams =
                    serde_json::from_value(args).context("Invalid parameters for scope_add")?;
                self.scope_add(ctx, params.profile_ids).await
            }
            "mcpmate_scope_remove" => {
                let ctx = context.ok_or_else(|| anyhow!("Client context required for mcpmate_scope_remove"))?;
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: ProfilesSelectParams =
                    serde_json::from_value(args).context("Invalid parameters for scope_remove")?;
                self.scope_remove(ctx, params.profile_ids).await
            }
            "mcpmate_client_custom_profile_details" => {
                let ctx = context
                    .ok_or_else(|| anyhow!("Client context required for mcpmate_client_custom_profile_details"))?;
                self.client_custom_profile_details(ctx).await
            }
            _ => Err(anyhow!("Unknown tool: {}", request.name)),
        }
    }

    async fn get_prompt_with_context(
        &self,
        request: &GetPromptRequestParams,
        context: Option<&ClientBuiltinContext>,
    ) -> Result<GetPromptResult> {
        let ctx = context.ok_or_else(|| anyhow!("Client context required for builtin Smart Mode prompts"))?;

        match request.name.as_ref() {
            "mcpmate_smart_mode_guide" => Ok(self.build_smart_mode_guide(ctx)),
            "mcpmate_smart_mode_next_actions" => Ok(self.build_smart_mode_next_actions(ctx)),
            _ => Err(anyhow!("Unknown prompt: {}", request.name)),
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
    config_mode: String,
    capability_source: String,
    selected_profile_ids: Option<Vec<String>>,
    custom_profile_id: Option<String>,
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
