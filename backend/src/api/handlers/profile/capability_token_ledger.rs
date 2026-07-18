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
        naming::{NamingKind, load_external_identifier},
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
        let external_name = load_external_identifier(&db.pool, NamingKind::Prompt, &p.server_id, &p.prompt_name)
            .await
            .map_err(|error| ApiError::InternalError(error.to_string()))?;
        let payload = ledger_prompt_payload(live.as_deref(), &p, &external_name)?;
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
        let external_uri = load_external_identifier(&db.pool, NamingKind::Resource, &r.server_id, &r.resource_uri)
            .await
            .map_err(|error| ApiError::InternalError(error.to_string()))?;
        let payload = ledger_resource_payload(live.as_deref(), &r, &external_uri)?;
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
        let external_name = load_external_identifier(
            &db.pool,
            NamingKind::ResourceTemplate,
            &tmpl.server_id,
            &tmpl.resource_uri,
        )
        .await
        .map_err(|error| ApiError::InternalError(error.to_string()))?;
        let payload = ledger_template_payload(live.as_deref(), &tmpl, &external_name)?;
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
                if tool_matches_profile(tool, &row.unique_name) {
                    return serde_json::to_string(item)
                        .map_err(|e| ApiError::InternalError(format!("Failed to serialize tool capability: {e}")));
                }
            }
        }
    }

    let fallback = ToolCapability {
        name: row.unique_name.clone(),
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
    unique_name: &str,
) -> bool {
    tool.name == unique_name || tool.unique_name == unique_name
}

fn ledger_prompt_payload(
    live: Option<&[CapabilityItem]>,
    row: &crate::config::models::ProfilePrompt,
    external_name: &str,
) -> Result<String, ApiError> {
    if let Some(items) = live {
        for item in items {
            if let CapabilityItem::Prompt(p) = item {
                if p.name == external_name || p.unique_name == external_name {
                    return serde_json::to_string(item)
                        .map_err(|e| ApiError::InternalError(format!("Failed to serialize prompt capability: {e}")));
                }
            }
        }
    }

    let fallback = PromptCapability {
        name: external_name.to_string(),
        description: None,
        arguments: None,
        unique_name: external_name.to_string(),
        enabled: row.enabled,
        icons: None,
    };
    serde_json::to_string(&CapabilityItem::Prompt(fallback))
        .map_err(|e| ApiError::InternalError(format!("Failed to serialize fallback prompt: {e}")))
}

fn ledger_resource_payload(
    live: Option<&[CapabilityItem]>,
    row: &crate::config::models::ProfileResource,
    external_uri: &str,
) -> Result<String, ApiError> {
    if let Some(items) = live {
        for item in items {
            if let CapabilityItem::Resource(r) = item {
                if r.uri == external_uri || r.unique_uri == external_uri {
                    return serde_json::to_string(item)
                        .map_err(|e| ApiError::InternalError(format!("Failed to serialize resource capability: {e}")));
                }
            }
        }
    }

    let fallback = ResourceCapability {
        uri: external_uri.to_string(),
        name: None,
        description: None,
        mime_type: None,
        unique_uri: external_uri.to_string(),
        enabled: row.enabled,
        icons: None,
    };
    serde_json::to_string(&CapabilityItem::Resource(fallback))
        .map_err(|e| ApiError::InternalError(format!("Failed to serialize fallback resource: {e}")))
}

fn ledger_template_payload(
    live: Option<&[CapabilityItem]>,
    row: &crate::config::models::ProfileResource,
    external_name: &str,
) -> Result<String, ApiError> {
    if let Some(items) = live {
        for item in items {
            if let CapabilityItem::ResourceTemplate(t) = item {
                if t.uri_template == external_name || t.unique_template == external_name {
                    return serde_json::to_string(item)
                        .map_err(|e| ApiError::InternalError(format!("Failed to serialize template capability: {e}")));
                }
            }
        }
    }

    let fallback = ResourceTemplateCapability {
        uri_template: external_name.to_string(),
        name: Some(external_name.to_string()),
        description: None,
        mime_type: None,
        unique_template: external_name.to_string(),
        enabled: row.enabled,
    };
    serde_json::to_string(&CapabilityItem::ResourceTemplate(fallback))
        .map_err(|e| ApiError::InternalError(format!("Failed to serialize fallback template: {e}")))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    fn profile_resource(raw: &str) -> crate::config::models::ProfileResource {
        crate::config::models::ProfileResource {
            id: Some("profile-resource".to_string()),
            profile_id: "profile".to_string(),
            server_id: "server".to_string(),
            server_name: "server".to_string(),
            resource_uri: raw.to_string(),
            enabled: true,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn fallback_payloads_expose_only_catalog_identifiers() {
        let tool = crate::config::models::ProfileToolWithDetails {
            id: "profile-tool".to_string(),
            profile_id: "profile".to_string(),
            server_tool_id: "server-tool".to_string(),
            enabled: true,
            created_at: None,
            updated_at: None,
            server_id: "server".to_string(),
            server_name: "server".to_string(),
            tool_name: "upstream_tool".to_string(),
            unique_name: "server_tool".to_string(),
            description: None,
        };
        let prompt = crate::config::models::ProfilePrompt {
            id: Some("profile-prompt".to_string()),
            profile_id: "profile".to_string(),
            server_id: "server".to_string(),
            server_name: "server".to_string(),
            prompt_name: "upstream_prompt".to_string(),
            enabled: true,
            created_at: None,
            updated_at: None,
        };
        let resource = profile_resource("file:///upstream");
        let template = profile_resource("repo://{owner}/{name}");

        let tool_payload: Value = serde_json::from_str(&ledger_tool_payload(None, &tool).unwrap()).unwrap();
        let prompt_payload: Value =
            serde_json::from_str(&ledger_prompt_payload(None, &prompt, "server_prompt").unwrap()).unwrap();
        let resource_payload: Value =
            serde_json::from_str(&ledger_resource_payload(None, &resource, "server_resource").unwrap()).unwrap();
        let template_payload: Value =
            serde_json::from_str(&ledger_template_payload(None, &template, "server_template").unwrap()).unwrap();

        assert_eq!(tool_payload["name"], "server_tool");
        assert_eq!(tool_payload["unique_name"], "server_tool");
        assert_eq!(prompt_payload["name"], "server_prompt");
        assert_eq!(prompt_payload["unique_name"], "server_prompt");
        assert_eq!(resource_payload["uri"], "server_resource");
        assert_eq!(resource_payload["unique_uri"], "server_resource");
        assert_eq!(template_payload["name"], "server_template");
        assert_eq!(template_payload["unique_template"], "server_template");
        assert_eq!(template_payload["uri_template"], "server_template");
    }

    #[test]
    fn live_template_payload_matches_uri_template_instead_of_display_name() {
        let external_template = "mcpmate://resources/template/docs/file/{path}";
        let display_name = "File";
        let live = vec![CapabilityItem::ResourceTemplate(ResourceTemplateCapability {
            uri_template: external_template.to_string(),
            name: Some(display_name.to_string()),
            description: Some("Read a file".to_string()),
            mime_type: None,
            unique_template: external_template.to_string(),
            enabled: true,
        })];
        let row = profile_resource("file:///{path}");

        let payload: Value = serde_json::from_str(
            &ledger_template_payload(Some(&live), &row, external_template).expect("serialize live template"),
        )
        .expect("parse live template payload");

        assert_eq!(payload["uri_template"], external_template);
        assert_eq!(payload["unique_template"], external_template);
        assert_eq!(payload["name"], display_name);
    }

    #[test]
    fn live_template_payload_does_not_match_display_name_as_identity() {
        let external_template = "mcpmate://resources/template/docs/file/{path}";
        let live = vec![CapabilityItem::ResourceTemplate(ResourceTemplateCapability {
            uri_template: "mcpmate://resources/template/docs/other/{path}".to_string(),
            name: Some(external_template.to_string()),
            description: Some("Wrong template".to_string()),
            mime_type: None,
            unique_template: "mcpmate://resources/template/docs/other/{path}".to_string(),
            enabled: true,
        })];
        let row = profile_resource("file:///{path}");

        let payload: Value = serde_json::from_str(
            &ledger_template_payload(Some(&live), &row, external_template).expect("serialize fallback template"),
        )
        .expect("parse fallback template payload");

        assert_eq!(payload["uri_template"], external_template);
        assert_eq!(payload["unique_template"], external_template);
        assert_ne!(payload["description"], "Wrong template");
    }
}
