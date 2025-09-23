use super::shared::*;
use crate::api::models::server::{
    ServerCapabilityMeta, ServerPreviewData, ServerPreviewItemData, ServerPreviewItemReq, ServerPreviewReq,
    ServerPreviewResp, ServerPromptsData, ServerResourceTemplatesData, ServerResourcesData, ServerToolsData,
};

/// Preview capabilities for arbitrary server configs (no DB/REDB/pool side-effects)
pub async fn preview_servers(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ServerPreviewReq>,
) -> Result<Json<ServerPreviewResp>, ApiError> {
    let timeout = req.timeout_ms.map(std::time::Duration::from_millis);
    let include_details = req.include_details.unwrap_or(true);

    // Process sequentially to avoid uncontrolled concurrency; can add a small semaphore later
    let mut items_out: Vec<ServerPreviewItemData> = Vec::with_capacity(req.servers.len());
    for item in req.servers {
        items_out.push(preview_one(item, timeout, include_details).await);
    }

    Ok(Json(ServerPreviewResp::success(ServerPreviewData { items: items_out })))
}

async fn preview_one(
    item: ServerPreviewItemReq,
    timeout: Option<std::time::Duration>,
    include_details: bool,
) -> ServerPreviewItemData {
    // Map kind -> ServerType
    let kind = match crate::common::server::ServerType::from_client_format(item.kind.as_str()) {
        Ok(k) => k,
        Err(_) => {
            return empty_with_error(item.name, format!("Invalid server kind: {}", item.kind));
        }
    };

    // Call preview (no side effects)
    let snap = crate::config::server::preview::preview_capabilities(
        &item.name,
        kind,
        item.command.clone(),
        item.url.clone(),
        timeout,
    )
    .await;

    match snap {
        Ok(s) => build_item(item.name, s, include_details),
        Err(e) => empty_with_error(item.name, e.to_string()),
    }
}

fn build_item(name: String, snap: crate::config::server::capabilities::CapabilitySnapshot, include_details: bool) -> ServerPreviewItemData {
    // tools
    let tool_items: Vec<serde_json::Value> = if include_details {
        snap
            .tools
            .iter()
            .map(super::capability::tool_json_from_cached)
            .collect()
    } else {
        Vec::new()
    };

    // resources
    let resource_items: Vec<serde_json::Value> = if include_details {
        snap
            .resources
            .iter()
            .map(|r| serde_json::json!({
                "uri": r.uri,
                "name": r.name,
                "description": r.description,
                "mime_type": r.mime_type,
                "enabled": r.enabled,
                "cached_at": r.cached_at.to_rfc3339(),
            }))
            .collect()
    } else {
        Vec::new()
    };

    let template_items: Vec<serde_json::Value> = if include_details {
        snap
            .resource_templates
            .iter()
            .map(|t| serde_json::json!({
                "uri_template": t.uri_template,
                "name": t.name,
                "description": t.description,
                "mime_type": t.mime_type,
                "enabled": t.enabled,
                "cached_at": t.cached_at.to_rfc3339(),
            }))
            .collect()
    } else {
        Vec::new()
    };

    let prompt_items: Vec<serde_json::Value> = if include_details {
        snap
            .prompts
            .iter()
            .map(|p| serde_json::json!({
                "name": p.name,
                "description": p.description,
                "arguments": p.arguments.iter().map(|a| serde_json::json!({
                    "name": a.name,
                    "description": a.description,
                    "required": a.required,
                })).collect::<Vec<_>>()
            }))
            .collect()
    } else {
        Vec::new()
    };

    let meta = ServerCapabilityMeta { cache_hit: false, strategy: "preview".to_string(), source: "live".to_string() };

    ServerPreviewItemData {
        name,
        ok: true,
        error: None,
        tools: ServerToolsData { items: tool_items, state: "ok".to_string(), meta: meta.clone() },
        resources: ServerResourcesData { items: resource_items, state: "ok".to_string(), meta: meta.clone() },
        resource_templates: ServerResourceTemplatesData { items: template_items, state: "ok".to_string(), meta: meta.clone() },
        prompts: ServerPromptsData { items: prompt_items, state: "ok".to_string(), meta },
    }
}

fn empty_with_error(name: String, err: String) -> ServerPreviewItemData {
    let meta = ServerCapabilityMeta { cache_hit: false, strategy: "preview".to_string(), source: "none".to_string() };
    ServerPreviewItemData {
        name,
        ok: false,
        error: Some(err),
        tools: ServerToolsData { items: Vec::new(), state: "error".to_string(), meta: meta.clone() },
        resources: ServerResourcesData { items: Vec::new(), state: "error".to_string(), meta: meta.clone() },
        resource_templates: ServerResourceTemplatesData { items: Vec::new(), state: "error".to_string(), meta: meta.clone() },
        prompts: ServerPromptsData { items: Vec::new(), state: "error".to_string(), meta },
    }
}
