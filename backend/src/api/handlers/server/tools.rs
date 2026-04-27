// Server tools handlers
// Provides handlers for server tool inspect endpoints

use crate::api::{
    models::server::{ServerCapabilityMeta, ServerCapabilityReq, ServerToolsData, ServerToolsResp},
    routes::AppState,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use super::common::{InspectQuery, create_inspect_response};

/// List all tools for a specific server
pub async fn server_tools(
    State(app_state): State<Arc<AppState>>,
    Query(request): Query<ServerCapabilityReq>,
) -> Result<Json<ServerToolsResp>, StatusCode> {
    let result = server_tools_core(&request, &app_state).await?;
    Ok(Json(result))
}

/// Core business logic for server tools operation
async fn server_tools_core(
    request: &ServerCapabilityReq,
    state: &Arc<AppState>,
) -> Result<ServerToolsResp, StatusCode> {
    // Convert ServerCapabilityReq to InspectQuery for compatibility with existing logic
    let query = InspectQuery {
        refresh: request.refresh.as_ref().map(|r| (*r).into()),
        format: None,
        include_meta: None,
        timeout: None,
    };

    // Validate and get server info using unified function
    let (db, server_info, params) = super::common::get_server_info_for_inspect(state, &request.id, &query).await?;

    // Check if server supports tools capability
    if let Some(response) = super::common::check_capability_or_error(
        &db.pool,
        &server_info,
        crate::common::capability::CapabilityToken::Tools,
        &params,
    )
    .await
    {
        let tools_resp = ServerToolsResp::success(json_to_server_tools_resp(response));
        return Ok(attach_raw_tool_names(&db, &server_info.server_id, tools_resp).await);
    }

    // Through CapabilityService: handler only requests result; service orchestrates cache/runtime/temp
    let refresh = match params.refresh {
        Some(super::common::RefreshStrategy::Force) => Some(crate::core::capability::runtime::RefreshStrategy::Force),
        _ => Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
    };
    let service = crate::core::capability::CapabilityService::new(
        state.redb_cache.clone(),
        state.connection_pool.clone(),
        db.clone(),
    );
    let list_result = service
        .list(&crate::core::capability::runtime::ListCtx {
            capability: crate::core::capability::CapabilityType::Tools,
            server_id: server_info.server_id.clone(),
            refresh,
            timeout: Some(std::time::Duration::from_secs(10)),
            validation_session: Some(crate::core::capability::service::CAPABILITY_VALIDATION_SESSION.to_string()),
            runtime_identity: None,
            connection_selection: None,
        })
        .await;
    match list_result {
        Ok(list_result) => {
            let crate::core::capability::runtime::ListResult { items, meta } = list_result;
            if let Some(tool_items) = items.into_tools() {
                if !tool_items.is_empty() {
                    let json_items: Vec<serde_json::Value> = tool_items
                        .into_iter()
                        .map(|tool| serde_json::to_value(tool).unwrap_or(serde_json::Value::Null))
                        .collect();
                    let response =
                        create_inspect_response(json_items, meta.cache_hit, params.refresh, meta.source.as_str());
                    let tools_resp = ServerToolsResp::success(json_to_server_tools_resp(response));
                    return Ok(attach_raw_tool_names(&db, &server_info.server_id, tools_resp).await);
                }
            } else {
                tracing::warn!("Capability runtime returned non-tool items for tool capability");
            }

            // Temporary instance is handled inside CapabilityService; handler does not touch pool
        }
        Err(e) => {
            tracing::error!("Failed to list tools via unified entry: {}", e);
        }
    }

    // No handler-side temporary creation

    // No offline fallback to avoid stale uncertainty; return empty

    // No data available
    let result = create_inspect_response(
        Vec::new(),
        false,
        params.refresh,
        crate::common::constants::strategies::NONE,
    );
    let tools_resp = ServerToolsResp::success(json_to_server_tools_resp(result));
    Ok(attach_raw_tool_names(&db, &server_info.server_id, tools_resp).await)
}

/// Helper function to convert Json response to ServerToolsResp
fn json_to_server_tools_resp(json_response: axum::Json<serde_json::Value>) -> ServerToolsData {
    let json_value = json_response.0;

    let items = json_value
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let state = json_value
        .get("state")
        .and_then(|s| s.as_str())
        .unwrap_or("ok")
        .to_string();

    let meta_value = json_value.get("meta").cloned().unwrap_or_default();
    let meta = ServerCapabilityMeta {
        cache_hit: meta_value.get("cache_hit").and_then(|v| v.as_bool()).unwrap_or(false),
        strategy: meta_value
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        source: meta_value
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
    };

    ServerToolsData { items, state, meta }
}

fn enrich_server_tool_items_with_raw_tool_name(
    items: Vec<Value>,
    persisted_tools: &[crate::config::models::ServerTool],
) -> Vec<Value> {
    let raw_tool_name_by_visible_name = persisted_tools
        .iter()
        .map(|tool| (tool.unique_name.as_str(), tool.tool_name.as_str()))
        .collect::<HashMap<_, _>>();
    let persisted_by_unique_name = persisted_tools
        .iter()
        .map(|tool| (tool.unique_name.as_str(), tool))
        .collect::<HashMap<_, _>>();
    let persisted_by_raw_name = persisted_tools
        .iter()
        .map(|tool| (tool.tool_name.as_str(), tool))
        .collect::<HashMap<_, _>>();

    items
        .into_iter()
        .map(|item| {
            let Some(object) = item.as_object() else {
                return item;
            };

            let Some(visible_name) = object.get("name").and_then(Value::as_str) else {
                return item;
            };

            let persisted_tool = persisted_by_unique_name
                .get(visible_name)
                .copied()
                .or_else(|| persisted_by_raw_name.get(visible_name).copied())
                .or_else(|| {
                    object
                        .get("tool_name")
                        .and_then(Value::as_str)
                        .and_then(|raw_tool_name| persisted_by_raw_name.get(raw_tool_name).copied())
                });

            let raw_tool_name = object
                .get("tool_name")
                .and_then(Value::as_str)
                .or_else(|| raw_tool_name_by_visible_name.get(visible_name).copied())
                .or_else(|| persisted_tool.map(|tool| tool.tool_name.as_str()));

            let Some(raw_tool_name) = raw_tool_name else {
                return item;
            };

            let mut object = object.clone();
            object.insert("tool_name".to_string(), Value::String(raw_tool_name.to_string()));
            if let Some(tool) = persisted_tool {
                object.insert("unique_name".to_string(), Value::String(tool.unique_name.clone()));
                object.insert("id".to_string(), Value::String(tool.id.clone()));
            }
            Value::Object(object)
        })
        .collect()
}

async fn attach_raw_tool_names(
    db: &Arc<crate::config::database::Database>,
    server_id: &str,
    mut response: ServerToolsResp,
) -> ServerToolsResp {
    let Some(data) = response.data.take() else {
        return response;
    };

    let persisted_tools = match crate::config::server::tools::get_server_tools(&db.pool, server_id).await {
        Ok(tools) => tools,
        Err(error) => {
            tracing::warn!(server_id = %server_id, error = %error, "Failed to load persisted server tools for response enrichment");
            response.data = Some(data);
            return response;
        }
    };

    response.data = Some(ServerToolsData {
        items: enrich_server_tool_items_with_raw_tool_name(data.items, &persisted_tools),
        state: data.state,
        meta: data.meta,
    });
    response
}

#[cfg(test)]
mod tests {
    use super::enrich_server_tool_items_with_raw_tool_name;
    use crate::config::models::ServerTool;
    use serde_json::json;

    #[test]
    fn preserves_display_name_and_attaches_raw_tool_name() {
        let items = vec![json!({
            "name": "21magic_component_builder",
            "description": "Build a component",
        })];
        let persisted_tools = vec![ServerTool {
            id: "stool_1".to_string(),
            server_id: "server-1".to_string(),
            server_name: "21magic".to_string(),
            tool_name: "component_builder".to_string(),
            unique_name: "21magic_component_builder".to_string(),
            description: Some("Build a component".to_string()),
            created_at: None,
            updated_at: None,
        }];

        let enriched = enrich_server_tool_items_with_raw_tool_name(items, &persisted_tools);
        let item = enriched
            .first()
            .and_then(|value| value.as_object())
            .expect("tool item object");

        assert_eq!(
            item.get("name").and_then(|value| value.as_str()),
            Some("21magic_component_builder")
        );
        assert_eq!(
            item.get("tool_name").and_then(|value| value.as_str()),
            Some("component_builder")
        );
        assert_eq!(
            item.get("unique_name").and_then(|value| value.as_str()),
            Some("21magic_component_builder")
        );
        assert_eq!(item.get("id").and_then(|value| value.as_str()), Some("stool_1"));
    }

    #[test]
    fn enriches_raw_tool_name_items_with_unique_identifiers() {
        let items = vec![json!({
            "name": "click",
            "description": "Click an element",
        })];
        let persisted_tools = vec![ServerTool {
            id: "stool_devtools_click".to_string(),
            server_id: "server-devtools".to_string(),
            server_name: "devtools".to_string(),
            tool_name: "click".to_string(),
            unique_name: "devtools_click".to_string(),
            description: Some("Click an element".to_string()),
            created_at: None,
            updated_at: None,
        }];

        let enriched = enrich_server_tool_items_with_raw_tool_name(items, &persisted_tools);
        let item = enriched
            .first()
            .and_then(|value| value.as_object())
            .expect("tool item object");

        assert_eq!(item.get("name").and_then(|value| value.as_str()), Some("click"));
        assert_eq!(item.get("tool_name").and_then(|value| value.as_str()), Some("click"));
        assert_eq!(
            item.get("unique_name").and_then(|value| value.as_str()),
            Some("devtools_click")
        );
        assert_eq!(
            item.get("id").and_then(|value| value.as_str()),
            Some("stool_devtools_click")
        );
    }
}
