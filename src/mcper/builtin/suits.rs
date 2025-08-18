//! Suits Service - Configuration Suits Management
//!
//! Protocol converter that transforms existing suits API capabilities
//! into MCP tool interfaces for client consumption.

use std::sync::Arc;

use anyhow::{Context, Result};
use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    config::{
        database::Database,
        suit::{
            self, get_config_suit_servers, get_config_suit_tools, get_prompts_for_config_suit,
            get_resources_for_config_suit,
        },
    },
    core::pool::UpstreamConnectionPool,
};

use crate::mcper::builtin::registry::BuiltinService;

/// Service providing configuration suits management via MCP tools
pub struct SuitsService {
    database: Arc<Database>,
    _connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
}

impl SuitsService {
    pub fn new(
        database: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        Self {
            database,
            _connection_pool: connection_pool,
        }
    }

    async fn list_suits(&self) -> Result<CallToolResult> {
        // TODO: Token optimization - Current implementation balances information value
        // with token consumption by providing counts + samples instead of full details.
        // For full details, consider implementing a separate tool or parameter-based detail levels.

        let suits = suit::get_all_config_suits(&self.database.pool)
            .await
            .context("Failed to list configuration suits")?;

        let mut response: Vec<SuitInfo> = Vec::new();

        // For each suit, get information using existing APIs but optimize for token usage
        for s in suits {
            if let Some(id) = s.id {
                // Use existing APIs to get detailed information instead of counts
                let servers = get_config_suit_servers(&self.database.pool, &id)
                    .await
                    .context("Failed to get suit servers")?;

                let tools = get_config_suit_tools(&self.database.pool, &id)
                    .await
                    .context("Failed to get suit tools")?;

                let prompts = get_prompts_for_config_suit(&self.database.pool, &id)
                    .await
                    .context("Failed to get suit prompts")?;

                let resources = get_resources_for_config_suit(&self.database.pool, &id)
                    .await
                    .context("Failed to get suit resources")?;

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

                response.push(SuitInfo {
                    id: id.clone(),
                    name: s.name,
                    description: s.description,
                    is_active: s.is_active,
                    suit_type: s.suit_type.to_string(),
                    server_count: server_summaries.len() as u32,
                    tool_count: tool_summaries.len() as u32,
                    prompt_count: prompt_summaries.len() as u32,
                    resource_count: resource_summaries.len() as u32,
                    sample_tools,
                    sample_servers,
                });
            } else {
                tracing::warn!("Found configuration suit '{}' without ID, skipping", s.name);
            }
        }

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&response).context("Failed to serialize response")?,
        )]))
    }

    async fn switch_suit(
        &self,
        suit_id: String,
        activate: bool,
    ) -> Result<CallToolResult> {
        let suit = suit::get_config_suit(&self.database.pool, &suit_id)
            .await
            .context("Failed to get configuration suit")?;

        let suit = suit.ok_or_else(|| anyhow::anyhow!("Configuration suit not found"))?;

        if suit.is_active == activate {
            let status = if activate { "already active" } else { "already inactive" };
            return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "message": format!("Suit '{}' is {}", suit.name, status),
                    "suit_id": suit_id,
                    "suit_name": suit.name,
                    "current_status": suit.is_active,
                }))
                .context("Failed to serialize response")?,
            )]));
        }

        suit::set_config_suit_active(&self.database.pool, &suit_id, activate)
            .await
            .context("Failed to update suit status")?;

        // Publish event to trigger synchronization
        crate::core::events::EventBus::global().publish(crate::core::events::Event::ConfigSuitStatusChanged {
            suit_id: suit_id.clone(),
            enabled: activate,
        });

        Ok(CallToolResult::success(vec![rmcp::model::Content::text(
            serde_json::to_string_pretty(&serde_json::json!({
                "success": true,
                "message": format!("Successfully {} suite '{}'",
                    if activate { "activated" } else { "deactivated" },
                    suit.name),
                "suit_id": suit_id,
                "suit_name": suit.name,
                "new_status": activate,
            }))
            .context("Failed to serialize response")?,
        )]))
    }
}

#[async_trait::async_trait]
impl BuiltinService for SuitsService {
    fn name(&self) -> &'static str {
        "mcpmate_suits"
    }

    fn tools(&self) -> Vec<rmcp::model::Tool> {
        vec![
            Tool::new(
                "mcpmate_list_suits",
                "List all available configuration suits with their current status",
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
                "mcpmate_switch_suit",
                "Activate or deactivate a configuration suit",
                std::sync::Arc::new(
                    serde_json::json!({
                        "type": "object",
                        "properties": {
                            "suit_id": {
                                "type": "string",
                                "description": "The ID of the configuration suit to switch"
                            },
                            "activate": {
                                "type": "boolean",
                                "description": "Whether to activate (true) or deactivate (false) the suit"
                            }
                        },
                        "required": ["suit_id", "activate"]
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
            "mcpmate_list_suits" => self.list_suits().await,
            "mcpmate_switch_suit" => {
                let args = serde_json::Value::Object(request.arguments.clone().unwrap_or_default());
                let params: SwitchSuitParams =
                    serde_json::from_value(args).context("Invalid parameters for switch_suit")?;
                self.switch_suit(params.suit_id, params.activate).await
            }
            _ => Err(anyhow::anyhow!("Unknown tool: {}", request.name)),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SwitchSuitParams {
    suit_id: String,
    activate: bool,
}

/// TODO: Optimize token usage - current version provides detailed information
/// but may consume too many tokens. Consider implementing parameter-based
/// detail levels (basic/detailed) or pagination for large datasets.
#[derive(Debug, Serialize)]
struct SuitInfo {
    id: String,
    name: String,
    description: Option<String>,
    is_active: bool,
    suit_type: String,
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
/// TODO: Implement as optional detailed mode or separate tool
#[allow(dead_code)]
#[derive(Debug, Serialize)]
struct DetailedSuitInfo {
    id: String,
    name: String,
    description: Option<String>,
    is_active: bool,
    suit_type: String,
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
