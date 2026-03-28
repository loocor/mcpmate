//! Per-profile capability JSON payloads for client-side token counting (cl100k / gpt-tokenizer).
//! Rows align with profile component ids from tools/list, prompts/list, resources/list, templates/list.

use std::{collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
};
use serde_json::json;

use super::{common::*, unified_capability_query::query_unified_capabilities};
use crate::{
    api::{
        handlers::{ApiError, server::common::InspectParams},
        models::token_estimate::{CapabilityTokenLedgerResponse, CapabilityTokenLedgerRow, TokenEstimateQuery},
    },
    core::capability::{
        CapabilityItem, CapabilityType,
        domain::{PromptCapability, ResourceCapability, ResourceTemplateCapability, ToolCapability},
    },
};

pub async fn capability_token_ledger(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TokenEstimateQuery>,
) -> Result<Json<CapabilityTokenLedgerResponse>, ApiError> {
    let profile_id = params.profile_id;

    let db = get_database(&state).await?;
    let unified_query = state
        .unified_query
        .clone()
        .ok_or_else(|| ApiError::InternalError("Unified capability query is unavailable".to_string()))?;

    let profile = crate::config::profile::get_profile(&db.pool, &profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile: {e}")))?;

    let Some(_) = profile else {
        return Err(ApiError::NotFound(format!("Profile '{profile_id}' not found")));
    };

    let profile_servers = crate::config::profile::get_profile_servers(&db.pool, &profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile servers: {e}")))?;

    let server_enabled: HashMap<String, bool> = profile_servers
        .iter()
        .map(|ps| (ps.server_id.clone(), ps.enabled))
        .collect();

    let inspect = InspectParams::default();
    let mut items = Vec::new();

    let profile_tools = crate::config::profile::get_profile_tools(&db.pool, &profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile tools: {e}")))?;

    for t in profile_tools {
        let live = query_unified_capabilities(&unified_query, &t.server_id, CapabilityType::Tools, &inspect).await;
        let payload = ledger_tool_payload(live.as_deref(), &t)?;
        items.push(CapabilityTokenLedgerRow {
            profile_row_id: t.id,
            kind: "tool".to_string(),
            server_id: t.server_id.clone(),
            server_enabled_in_profile: *server_enabled.get(&t.server_id).unwrap_or(&false),
            payload_json: payload,
        });
    }

    let profile_prompts = crate::config::profile::get_prompts_for_profile(&db.pool, &profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile prompts: {e}")))?;

    for p in profile_prompts {
        let live = query_unified_capabilities(&unified_query, &p.server_id, CapabilityType::Prompts, &inspect).await;
        let payload = ledger_prompt_payload(live.as_deref(), &p)?;
        let row_id = p.id.clone().unwrap_or_default();
        items.push(CapabilityTokenLedgerRow {
            profile_row_id: row_id,
            kind: "prompt".to_string(),
            server_id: p.server_id.clone(),
            server_enabled_in_profile: *server_enabled.get(&p.server_id).unwrap_or(&false),
            payload_json: payload,
        });
    }

    let profile_resources = crate::config::profile::get_resources_for_profile(&db.pool, &profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile resources: {e}")))?;

    for r in profile_resources {
        let live = query_unified_capabilities(&unified_query, &r.server_id, CapabilityType::Resources, &inspect).await;
        let payload = ledger_resource_payload(live.as_deref(), &r)?;
        let row_id = r.id.clone().unwrap_or_default();
        items.push(CapabilityTokenLedgerRow {
            profile_row_id: row_id,
            kind: "resource".to_string(),
            server_id: r.server_id.clone(),
            server_enabled_in_profile: *server_enabled.get(&r.server_id).unwrap_or(&false),
            payload_json: payload,
        });
    }

    let profile_templates = crate::config::profile::get_resource_templates_for_profile(&db.pool, &profile_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get profile templates: {e}")))?;

    for tmpl in profile_templates {
        let live = query_unified_capabilities(
            &unified_query,
            &tmpl.server_id,
            CapabilityType::ResourceTemplates,
            &inspect,
        )
        .await;
        let payload = ledger_template_payload(live.as_deref(), &tmpl)?;
        let row_id = tmpl.id.clone().unwrap_or_default();
        items.push(CapabilityTokenLedgerRow {
            profile_row_id: row_id,
            kind: "template".to_string(),
            server_id: tmpl.server_id.clone(),
            server_enabled_in_profile: *server_enabled.get(&tmpl.server_id).unwrap_or(&false),
            payload_json: payload,
        });
    }

    Ok(Json(CapabilityTokenLedgerResponse {
        items,
        tokenizer_note: "Count UTF-8 tokens with gpt-tokenizer cl100k_base on payload_json.".to_string(),
    }))
}

fn ledger_tool_payload(
    live: Option<&[CapabilityItem]>,
    row: &crate::config::models::ProfileToolWithDetails,
) -> Result<String, ApiError> {
    if let Some(items) = live {
        for item in items {
            if let CapabilityItem::Tool(tool) = item {
                if tool_matches_profile(tool, &row.tool_name, &row.unique_name) {
                    return serde_json::to_string(item)
                        .map_err(|e| ApiError::InternalError(format!("Failed to serialize tool capability: {e}")));
                }
            }
        }
    }

    let fallback = ToolCapability {
        name: row.tool_name.clone(),
        description: row.description.clone(),
        input_schema: json!({}),
        unique_name: row.unique_name.clone(),
        enabled: row.enabled,
        icons: None,
    };
    serde_json::to_string(&CapabilityItem::Tool(fallback))
        .map_err(|e| ApiError::InternalError(format!("Failed to serialize fallback tool: {e}")))
}

fn tool_matches_profile(
    tool: &ToolCapability,
    tool_name: &str,
    unique_name: &str,
) -> bool {
    tool.name == tool_name
        || tool.unique_name == unique_name
        || tool.name == unique_name
        || tool.unique_name == tool_name
}

fn ledger_prompt_payload(
    live: Option<&[CapabilityItem]>,
    row: &crate::config::models::ProfilePrompt,
) -> Result<String, ApiError> {
    if let Some(items) = live {
        for item in items {
            if let CapabilityItem::Prompt(p) = item {
                if p.name == row.prompt_name {
                    return serde_json::to_string(item)
                        .map_err(|e| ApiError::InternalError(format!("Failed to serialize prompt capability: {e}")));
                }
            }
        }
    }

    let fallback = PromptCapability {
        name: row.prompt_name.clone(),
        description: None,
        arguments: None,
        unique_name: row.prompt_name.clone(),
        enabled: row.enabled,
        icons: None,
    };
    serde_json::to_string(&CapabilityItem::Prompt(fallback))
        .map_err(|e| ApiError::InternalError(format!("Failed to serialize fallback prompt: {e}")))
}

fn ledger_resource_payload(
    live: Option<&[CapabilityItem]>,
    row: &crate::config::models::ProfileResource,
) -> Result<String, ApiError> {
    if let Some(items) = live {
        for item in items {
            if let CapabilityItem::Resource(r) = item {
                if r.uri == row.resource_uri {
                    return serde_json::to_string(item)
                        .map_err(|e| ApiError::InternalError(format!("Failed to serialize resource capability: {e}")));
                }
            }
        }
    }

    let fallback = ResourceCapability {
        uri: row.resource_uri.clone(),
        name: None,
        description: None,
        mime_type: None,
        unique_uri: row.resource_uri.clone(),
        enabled: row.enabled,
        icons: None,
    };
    serde_json::to_string(&CapabilityItem::Resource(fallback))
        .map_err(|e| ApiError::InternalError(format!("Failed to serialize fallback resource: {e}")))
}

fn ledger_template_payload(
    live: Option<&[CapabilityItem]>,
    row: &crate::config::models::ProfileResource,
) -> Result<String, ApiError> {
    if let Some(items) = live {
        for item in items {
            if let CapabilityItem::ResourceTemplate(t) = item {
                if t.uri_template == row.resource_uri {
                    return serde_json::to_string(item)
                        .map_err(|e| ApiError::InternalError(format!("Failed to serialize template capability: {e}")));
                }
            }
        }
    }

    let fallback = ResourceTemplateCapability {
        uri_template: row.resource_uri.clone(),
        name: None,
        description: None,
        mime_type: None,
        unique_template: row.resource_uri.clone(),
        enabled: row.enabled,
    };
    serde_json::to_string(&CapabilityItem::ResourceTemplate(fallback))
        .map_err(|e| ApiError::InternalError(format!("Failed to serialize fallback template: {e}")))
}
