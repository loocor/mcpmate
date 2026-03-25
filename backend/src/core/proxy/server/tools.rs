use super::*;
use crate::core::capability::naming::{NamingKind, resolve_unique_name};
use futures::StreamExt;
use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolRequest, CallToolRequestParams, CallToolResult, ClientRequest, PaginatedRequestParams};
use rmcp::service::PeerRequestOptions;
use rmcp::service::RequestContext;

/// Determines whether a builtin tool should be visible/invocable for a given
/// capability source. Profile management tools are only available for clients
/// using the Activated capability source, as they operate on global profile state.
/// Clients configured with Profiles or Custom mode should not see or manage
/// profiles outside their configured scope.
fn builtin_tool_allowed_for_capability_source(
    capability_source: crate::clients::models::CapabilitySource,
    tool_name: &str,
) -> bool {
    let is_profile_tool = tool_name.starts_with("mcpmate_profile_");
    if is_profile_tool {
        capability_source == crate::clients::models::CapabilitySource::Activated
    } else {
        true
    }
}

pub(super) async fn list_tools(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParams>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<rmcp::model::ListToolsResult, McpError> {
    let client = server.resolve_bound_client_context(&_context).await?;
    let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
        server.database.clone(),
        server.profile_service.clone(),
    );
    let snapshot = vis
        .resolve_snapshot(&client.client_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let visible_server_ids = snapshot.server_ids.iter().cloned().collect::<std::collections::HashSet<_>>();
    let mut tools: Vec<rmcp::model::Tool> = Vec::new();

    if let Some(db) = &server.database {
        let enabled_servers: Vec<(String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, sc.name, sc.capabilities
            FROM server_config sc
            WHERE sc.enabled = 1
            ORDER BY sc.name, sc.id
            "#,
        )
        .fetch_all(&db.pool)
        .await
        .unwrap_or_default();

        let redb = &server.redb_cache;
        let pool = &server.connection_pool;

        let mut tasks = Vec::new();
        for (server_id, _server_name, capabilities) in enabled_servers {
            if !visible_server_ids.contains(&server_id) {
                continue;
            }
            if !super::supports_capability(capabilities.as_deref(), crate::core::capability::CapabilityType::Tools) {
                continue;
            }
            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Tools,
                server_id: server_id.clone(),
                refresh: Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
                timeout: Some(std::time::Duration::from_secs(10)),
                validation_session: None,
                runtime_identity: client.runtime_identity(),
                connection_selection: client.connection_selection(server_id.clone()),
            };
            let redb = redb.clone();
            let pool = pool.clone();
            let db = db.clone();
            tasks.push(async move {
                match crate::core::capability::runtime::list(&ctx, &redb, &pool, &db).await {
                    Ok(result) => result.items.into_tools().unwrap_or_default(),
                    Err(_) => Vec::new(),
                }
            });
        }

        for mut v in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            tools.append(&mut v);
        }
    }

    tools = vis.filter_tools_with_snapshot(&snapshot, tools);

    let capability_config = vis
        .resolve_capability_config(&client.client_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let builtin_tools = server
        .builtin_services
        .tools()
        .into_iter()
        .filter(|tool| builtin_tool_allowed_for_capability_source(capability_config.capability_source, tool.name.as_ref()))
        .collect::<Vec<_>>();
    tracing::debug!("Including {} builtin service tools", builtin_tools.len());
    tools.extend(builtin_tools);

    // Apply pagination (natural sort inside paginator)
    let page = server.paginator.paginate_tools(&_request, tools)?;

    tracing::info!(
        total = page.items.len(),
        has_next = page.next_cursor.is_some(),
        "Proxy listed tools (including builtin services)"
    );

    Ok(rmcp::model::ListToolsResult {
        tools: page.items,
        next_cursor: page.next_cursor,
        ..Default::default()
    })
}

pub(super) async fn call_tool(
    server: &ProxyServer,
    request: CallToolRequestParams,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<CallToolResult, McpError> {
    let client = server.resolve_bound_client_context(&_context).await?;
    let call_id = crate::generate_id!("tcall");
    let started_at = std::time::Instant::now();

    tracing::debug!(
        call_id = %call_id,
        tool = %request.name,
        "ProxyServer::call_tool received request"
    );

    if request.name.starts_with("mcpmate_profile_") {
        let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
            server.database.clone(),
            server.profile_service.clone(),
        );
        let capability_config = vis
            .resolve_capability_config(&client.client_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        if !builtin_tool_allowed_for_capability_source(capability_config.capability_source, &request.name) {
            return Err(McpError::invalid_params(
                "profile management tools are only available for clients using the activated capability source"
                    .to_string(),
                None,
            ));
        }
    }

    if let Some(result) = server.builtin_services.call_tool(&request).await {
        tracing::debug!(
            call_id = %call_id,
            tool = %request.name,
            "ProxyServer::call_tool handled by builtin service"
        );
        return match result {
            Ok(call_result) => Ok(call_result),
            Err(e) => {
                tracing::error!(
                    call_id = %call_id,
                    tool = %request.name,
                    error = %e,
                    "Builtin service tool failed"
                );
                Err(McpError::internal_error(e.to_string(), None))
            }
        };
    }

    if server.database.is_none() {
        tracing::error!("Database not available for tool calling");
        return Err(McpError::internal_error(
            "Database not available for tool calling".to_string(),
            None,
        ));
    }

    let (server_name, original_tool_name) =
        resolve_unique_name(NamingKind::Tool, &request.name)
            .await
            .map_err(|e| {
                tracing::error!(
                    call_id = %call_id,
                    tool = %request.name,
                    error = %e,
                    "ProxyServer::call_tool failed to resolve unique name"
                );
                McpError::internal_error(format!("Failed to resolve unique tool name: {}", e), None)
            })?;
    let server_id = crate::core::capability::resolver::to_id(&server_name)
        .await
        .ok()
        .flatten()
        .ok_or_else(|| {
            tracing::error!(
                call_id = %call_id,
                tool = %request.name,
                server_name = %server_name,
                "ProxyServer::call_tool missing server id for mapping"
            );
            McpError::internal_error("Server not found for tool mapping".to_string(), None)
        })?;

    tracing::debug!(
        call_id = %call_id,
        tool = %request.name,
        server_name = %server_name,
        server_id = %server_id,
        upstream_tool = %original_tool_name,
        client_id = %client.client_id,
        profile_id = ?client.profile_id,
        "ProxyServer::call_tool resolved mapping"
    );

    let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
        server.database.clone(),
        server.profile_service.clone(),
    );
    let snapshot = vis
        .resolve_snapshot(&client.client_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    if let Err(error) = vis
        .assert_tool_allowed_with_snapshot(&snapshot, &request.name)
        .await
    {
        tracing::warn!(
            call_id = %call_id,
            tool = %request.name,
            client_id = %client.client_id,
            profile_id = ?client.profile_id,
            error = %error,
            "ProxyServer::call_tool denied by visibility policy"
        );
        return Err(McpError::invalid_params(
            format!("Tool '{}' is not available for this client", request.name),
            None,
        ));
    }

    // Resolve tool call timeout from env (fallback 30s)
    let call_timeout_secs: u64 = std::env::var("MCPMATE_TOOL_CALL_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);

    // Acquire upstream peer (ensure connected if necessary)
    let (peer_opt, instance_id_opt) = {
        let pool_guard = server.connection_pool.lock().await;
        let snap = pool_guard.get_snapshot();
        let mut p: Option<rmcp::service::Peer<rmcp::RoleClient>> = None;
        let mut iid: Option<String> = None;
        if let Some(selection) = client.connection_selection(server_id.clone()) {
            if let Ok(Some(selected_instance_id)) = pool_guard.select_ready_instance_id(&selection) {
                if let Some(instances) = snap.get(&server_id) {
                    if let Some((iid0, _st, _res, _prm, peer)) = instances
                        .iter()
                        .find(|(candidate_id, _st, _res, _prm, peer)| {
                            **candidate_id == selected_instance_id && peer.is_some()
                        })
                    {
                        p = peer.clone();
                        iid = Some(iid0.clone());
                    }
                }
            }
        }
        if p.is_none() {
            if let Some(instances) = snap.get(&server_id) {
                if let Some((iid0, _st, _res, _prm, peer)) = instances.iter().find(|(_, st, _, _, p)| {
                    matches!(st, crate::core::foundation::types::ConnectionStatus::Ready) && p.is_some()
                }) {
                    p = peer.clone();
                    iid = Some(iid0.clone());
                }
            }
        }
        (p, iid)
    };
    let peer = if let Some(peer) = peer_opt {
        peer
    } else {
        let t_connect_begin = std::time::Instant::now();
        {
            let mut pool_guard = server.connection_pool.lock().await;
            if let Some(selection) = client.connection_selection(server_id.clone()) {
                pool_guard
                    .ensure_connected_with_selection(&selection)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            } else {
                pool_guard
                    .ensure_connected(&server_id)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            }
        }
        let pool_guard = server.connection_pool.lock().await;
        let snap = pool_guard.get_snapshot();
        let Some(instances) = snap.get(&server_id) else {
            return Err(McpError::internal_error(
                "No instance after ensure_connected".to_string(),
                None,
            ));
        };
        let Some((iid, _st, _r, _p, peer)) = instances.iter().find(|(_, st, _, _, p)| {
            matches!(st, crate::core::foundation::types::ConnectionStatus::Ready) && p.is_some()
        }) else {
            return Err(McpError::internal_error("Ready instance not found".to_string(), None));
        };
        tracing::debug!(
            call_id = %call_id,
            ensure_connected_ms = %t_connect_begin.elapsed().as_millis(),
            instance_id = %iid,
            "Ensured connection before tool call"
        );
        drop(pool_guard);
        instance_id_opt.or(Some(iid.clone()));
        peer.clone().expect("peer exists by check")
    };

    // Build cancellable request to capture progress token & request id
    let mut params = CallToolRequestParams::new(original_tool_name.clone());
    if let Some(arguments) = request.arguments.clone() {
        params = params.with_arguments(arguments);
    }
    let req = ClientRequest::CallToolRequest(CallToolRequest::new(params));
    let options = PeerRequestOptions {
        timeout: Some(std::time::Duration::from_secs(call_timeout_secs)),
        meta: None,
    };
    let handle = peer
        .send_cancellable_request(req, options)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    // Map progress token and request id to the exact downstream route for forwarding
    let downstream_route = server.build_downstream_route(&client, _context.peer.clone())?;
    server.register_call_session(handle.progress_token.clone(), handle.id.clone(), downstream_route);

    // Await response and cleanup mapping
    let token = handle.progress_token.clone();
    let req_id = handle.id.clone();
    let resp = handle.await_response().await;
    server.unregister_call_session(&token, &req_id);

    match resp {
        Ok(rmcp::model::ServerResult::CallToolResult(result)) => {
            tracing::info!(
                call_id = %call_id,
                tool = %request.name,
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                "ProxyServer::call_tool succeeded"
            );
            Ok(result)
        }
        Ok(other) => {
            tracing::error!(?other, "Unexpected server result kind for tools/call");
            Err(McpError::internal_error("Unexpected server result".to_string(), None))
        }
        Err(e) => {
            tracing::error!(
                call_id = %call_id,
                tool = %request.name,
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                error = %e,
                "ProxyServer::call_tool upstream error"
            );
            Err(McpError::internal_error(e.to_string(), None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::builtin_tool_allowed_for_capability_source;
    use crate::clients::models::CapabilitySource;

    #[test]
    fn profile_tools_are_only_available_for_activated_capability_source() {
        let profile_tools = [
            "mcpmate_profile_list",
            "mcpmate_profile_details",
            "mcpmate_profile_switch",
        ];

        for tool in profile_tools {
            assert!(
                builtin_tool_allowed_for_capability_source(CapabilitySource::Activated, tool),
                "{tool} should be available for Activated"
            );
            assert!(
                !builtin_tool_allowed_for_capability_source(CapabilitySource::Profiles, tool),
                "{tool} should NOT be available for Profiles"
            );
            assert!(
                !builtin_tool_allowed_for_capability_source(CapabilitySource::Custom, tool),
                "{tool} should NOT be available for Custom"
            );
        }
    }

    #[test]
    fn non_profile_tools_are_available_for_all_capability_sources() {
        let other_tools = ["some_other_tool", "another_mcpmate_service"];

        for tool in other_tools {
            assert!(
                builtin_tool_allowed_for_capability_source(CapabilitySource::Activated, tool),
                "{tool} should be available for Activated"
            );
            assert!(
                builtin_tool_allowed_for_capability_source(CapabilitySource::Profiles, tool),
                "{tool} should be available for Profiles"
            );
            assert!(
                builtin_tool_allowed_for_capability_source(CapabilitySource::Custom, tool),
                "{tool} should be available for Custom"
            );
        }
    }
}
