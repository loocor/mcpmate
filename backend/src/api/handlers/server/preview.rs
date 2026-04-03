use super::shared::*;
use crate::api::models::server::{
    ServerCapabilityMeta, ServerPreviewData, ServerPreviewItemData, ServerPreviewItemReq, ServerPreviewReq,
    ServerPreviewResp, ServerPromptsData, ServerResourceTemplatesData, ServerResourcesData, ServerToolsData,
};

/// Preview capabilities for arbitrary server configs (no DB/REDB/pool side-effects)
pub async fn preview_servers(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ServerPreviewReq>,
) -> Result<Json<ServerPreviewResp>, ApiError> {
    let timeout = req.timeout_ms.map(std::time::Duration::from_millis);
    let include_details = req.include_details.unwrap_or(true);
    let db_pool = state.database.as_ref().map(|db| db.pool.clone());

    // Process sequentially to avoid uncontrolled concurrency; can add a small semaphore later
    let mut items_out: Vec<ServerPreviewItemData> = Vec::with_capacity(req.servers.len());
    for item in req.servers {
        items_out.push(preview_one(item, timeout, include_details, db_pool.as_ref()).await);
    }

    Ok(Json(ServerPreviewResp::success(ServerPreviewData { items: items_out })))
}

async fn preview_one(
    item: ServerPreviewItemReq,
    timeout: Option<std::time::Duration>,
    include_details: bool,
    db_pool: Option<&sqlx::SqlitePool>,
) -> ServerPreviewItemData {
    // Map kind -> ServerType
    let kind = match crate::common::server::ServerType::from_client_format(item.kind.as_str()) {
        Ok(k) => k,
        Err(_) => {
            return empty_with_error(item.name, format!("Invalid server kind: {}", item.kind));
        }
    };

    // Call preview (no side effects)
    // Build optional HTTP client with default headers if provided
    let effective_headers = if let (Some(pool), Some(server_id)) = (db_pool, item.server_id.as_deref()) {
        crate::config::server::oauth::get_effective_server_headers(pool, server_id, item.headers.clone())
            .await
            .ok()
            .flatten()
    } else {
        item.headers.clone()
    };

    let mut client: Option<reqwest::Client> = None;
    if matches!(kind, crate::common::server::ServerType::StreamableHttp) {
        if let Some(headers) = effective_headers.as_ref() {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (k, v) in headers.iter() {
                if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_bytes()) {
                    if let Ok(value) = reqwest::header::HeaderValue::from_str(v) {
                        header_map.insert(name, value);
                    }
                }
            }
            let builder = reqwest::Client::builder().default_headers(header_map);
            if let Ok(built) = builder.build() {
                client = Some(built);
            }
        }
    }

    // Compute preview timeouts (fallbacks if not provided)
    let stdio_timeout = timeout;
    let http_to = timeout.map(|t| {
        // Split a single timeout into connection/service/tools windows
        // Connection: min(10s, total), Service+Tools: total
        let conn = std::cmp::min(std::time::Duration::from_secs(10), t);
        (conn, t, t)
    });

    let cfg = crate::core::models::MCPServerConfig {
        kind,
        command: item.command.clone(),
        url: item.url.clone(),
        args: item.args.clone(),
        env: item.env.clone(),
        headers: effective_headers,
    };

    let snap = crate::config::server::capabilities::discover_from_config_preview(
        &item.name,
        &cfg,
        kind,
        client,
        http_to,
        stdio_timeout,
    )
    .await;

    match snap {
        Ok(s) => build_item(item.name, s, include_details),
        Err(e) => empty_with_error(item.name, e.to_string()),
    }
}

fn build_item(
    name: String,
    snap: crate::config::server::capabilities::CapabilitySnapshot,
    include_details: bool,
) -> ServerPreviewItemData {
    // tools
    let tool_items: Vec<serde_json::Value> = if include_details {
        snap.tools
            .iter()
            .map(super::capability::tool_json_from_cached)
            .collect()
    } else {
        Vec::new()
    };

    // resources
    let resource_items: Vec<serde_json::Value> = if include_details {
        snap.resources
            .iter()
            .map(|r| {
                serde_json::json!({
                    "uri": r.uri,
                    "name": r.name,
                    "description": r.description,
                    "mime_type": r.mime_type,
                    "enabled": r.enabled,
                    "cached_at": r.cached_at.to_rfc3339(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let template_items: Vec<serde_json::Value> = if include_details {
        snap.resource_templates
            .iter()
            .map(|t| {
                serde_json::json!({
                    "uri_template": t.uri_template,
                    "name": t.name,
                    "description": t.description,
                    "mime_type": t.mime_type,
                    "enabled": t.enabled,
                    "cached_at": t.cached_at.to_rfc3339(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let prompt_items: Vec<serde_json::Value> = if include_details {
        snap.prompts
            .iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.name,
                    "description": p.description,
                    "arguments": p.arguments.iter().map(|a| serde_json::json!({
                        "name": a.name,
                        "description": a.description,
                        "required": a.required,
                    })).collect::<Vec<_>>()
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    let meta = ServerCapabilityMeta {
        cache_hit: false,
        strategy: "preview".to_string(),
        source: "live".to_string(),
    };

    ServerPreviewItemData {
        name,
        ok: true,
        error: None,
        tools: ServerToolsData {
            items: tool_items,
            state: "ok".to_string(),
            meta: meta.clone(),
        },
        resources: ServerResourcesData {
            items: resource_items,
            state: "ok".to_string(),
            meta: meta.clone(),
        },
        resource_templates: ServerResourceTemplatesData {
            items: template_items,
            state: "ok".to_string(),
            meta: meta.clone(),
        },
        prompts: ServerPromptsData {
            items: prompt_items,
            state: "ok".to_string(),
            meta,
        },
    }
}

fn empty_with_error(
    name: String,
    err: String,
) -> ServerPreviewItemData {
    let meta = ServerCapabilityMeta {
        cache_hit: false,
        strategy: "preview".to_string(),
        source: "none".to_string(),
    };
    ServerPreviewItemData {
        name,
        ok: false,
        error: Some(err),
        tools: ServerToolsData {
            items: Vec::new(),
            state: "error".to_string(),
            meta: meta.clone(),
        },
        resources: ServerResourcesData {
            items: Vec::new(),
            state: "error".to_string(),
            meta: meta.clone(),
        },
        resource_templates: ServerResourceTemplatesData {
            items: Vec::new(),
            state: "error".to_string(),
            meta: meta.clone(),
        },
        prompts: ServerPromptsData {
            items: Vec::new(),
            state: "error".to_string(),
            meta,
        },
    }
}
