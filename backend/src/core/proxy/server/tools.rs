use super::*;
use crate::clients::models::CapabilitySource;
use crate::core::capability::naming::{NamingKind, resolve_unique_name};
use crate::mcper::builtin::ClientBuiltinContext;
use futures::StreamExt;
use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolRequest, CallToolRequestParams, CallToolResult, ClientRequest, PaginatedRequestParams};
use rmcp::service::PeerRequestOptions;
use rmcp::service::RequestContext;

fn builtin_tool_allowed(
    config_mode: Option<&str>,
    capability_source: CapabilitySource,
    tool_name: &str,
) -> bool {
    match config_mode {
        Some("unify") => matches!(
            tool_name,
            "mcpmate_ucan_catalog" | "mcpmate_ucan_details" | "mcpmate_ucan_call"
        ),
        Some("transparent") => false,
        _ => match tool_name {
            "mcpmate_profile_list" | "mcpmate_profile_preview" => capability_source == CapabilitySource::Profiles,
            "mcpmate_scope_set" | "mcpmate_scope_add" | "mcpmate_scope_remove" => {
                capability_source == CapabilitySource::Profiles
            }
            _ => false,
        },
    }
}

fn direct_managed_tool_call_allowed(
    config_mode: Option<&str>,
    directly_exposed: bool,
) -> bool {
    !matches!(config_mode, Some("unify")) || directly_exposed
}

fn client_aware_builtin_tool_requires_runtime_refresh(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "mcpmate_profile_enable"
            | "mcpmate_profile_disable"
            | "mcpmate_profile_activate_only"
            | "mcpmate_scope_set"
            | "mcpmate_scope_add"
            | "mcpmate_scope_remove"
    )
}

fn builtin_tool_requires_runtime_refresh(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "mcpmate_profile_enable" | "mcpmate_profile_disable" | "mcpmate_profile_activate_only"
    )
}

pub(super) async fn list_tools(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParams>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<rmcp::model::ListToolsResult, McpError> {
    let client = server.resolve_bound_client_context(&_context).await?;
    let unify_mode = matches!(client.config_mode.as_deref(), Some("unify"));
    let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
        server.database.clone(),
        server.profile_service.clone(),
    );
    let snapshot = vis
        .resolve_snapshot_for_client(&client)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let visible_server_ids = snapshot
        .server_ids
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let unify_direct_exposure_eligible_server_ids = if unify_mode {
        if let Some(db) = &server.database {
            crate::core::proxy::server::load_unify_direct_exposure_eligible_server_ids(db).await?
        } else {
            std::collections::HashSet::new()
        }
    } else {
        std::collections::HashSet::new()
    };

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
        for (server_id, server_name, capabilities) in enabled_servers {
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
                let tools = match crate::core::capability::runtime::list(&ctx, &redb, &pool, &db).await {
                    Ok(result) => result.items.into_tools().unwrap_or_default(),
                    Err(_) => Vec::new(),
                };
                (server_id, server_name, tools)
            });
        }

        for (server_id, server_name, tool_batch) in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            for tool in tool_batch {
                let raw_tool_name = crate::core::proxy::server::resolve_direct_surface_value(
                    NamingKind::Tool,
                    &server_name,
                    tool.name.as_ref(),
                )
                .await;
                let expose_directly = !unify_mode
                    || crate::core::proxy::server::unify_directly_exposed_tool_allowed(
                        client.unify_workspace.as_ref(),
                        &unify_direct_exposure_eligible_server_ids,
                        &server_id,
                        raw_tool_name.as_ref(),
                    );
                if expose_directly {
                    tools.push(tool);
                }
            }
        }
    }

    tools = vis.filter_tools_with_snapshot(&snapshot, tools);

    let capability_config = vis
        .resolve_capability_config_for_client(&client)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let builtin_tools = server
        .builtin_services
        .tools()
        .into_iter()
        .filter(|tool| {
            builtin_tool_allowed(
                client.config_mode.as_deref(),
                capability_config.capability_source,
                tool.name.as_ref(),
            )
        })
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

    let is_profile_tool = request.name.starts_with("mcpmate_profile_");
    let is_client_tool = matches!(
        request.name.as_ref(),
        "mcpmate_scope_get"
            | "mcpmate_scope_set"
            | "mcpmate_scope_add"
            | "mcpmate_scope_remove"
            | "mcpmate_client_custom_profile_details"
            | "mcpmate_ucan_catalog"
            | "mcpmate_ucan_details"
            | "mcpmate_ucan_call"
    );

    if is_profile_tool || is_client_tool {
        let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
            server.database.clone(),
            server.profile_service.clone(),
        );
        let capability_config = vis
            .resolve_capability_config_for_client(&client)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if !builtin_tool_allowed(
            client.config_mode.as_deref(),
            capability_config.capability_source,
            &request.name,
        ) {
            let message = if is_profile_tool {
                "profile helper tools are only available for clients using the profiles capability source"
            } else {
                "client tool not available for current capability source"
            };
            return Err(McpError::invalid_params(message.to_string(), None));
        }

        if is_client_tool {
            let builtin_context = ClientBuiltinContext {
                client_id: client.client_id.clone(),
                session_id: client.session_id.clone(),
                config_mode: client.config_mode.clone(),
                capability_source: capability_config.capability_source,
                selected_profile_ids: capability_config.selected_profile_ids.clone(),
                custom_profile_id: capability_config.custom_profile_id.clone(),
                unify_workspace: client.unify_workspace.clone(),
            };

            if let Some(result) = server
                .builtin_services
                .call_tool_with_context(&request, Some(&builtin_context))
                .await
            {
                tracing::debug!(
                    call_id = %call_id,
                    tool = %request.name,
                    "ProxyServer::call_tool handled by client-aware builtin service"
                );
                return match result {
                    Ok(call_result) => {
                        if client_aware_builtin_tool_requires_runtime_refresh(request.name.as_ref()) {
                            if let Some(session_id) = client.session_id.as_deref() {
                                server
                                    .refresh_bound_session_runtime_identity(session_id, &client.client_id)
                                    .await?;
                            }
                        }
                        Ok(call_result)
                    }
                    Err(e) => {
                        tracing::error!(
                            call_id = %call_id,
                            tool = %request.name,
                            error = %e,
                            "Client-aware builtin service tool failed"
                        );
                        Err(McpError::internal_error(e.to_string(), None))
                    }
                };
            }
        }
    }

    if let Some(result) = server.builtin_services.call_tool(&request).await {
        tracing::debug!(
            call_id = %call_id,
            tool = %request.name,
            "ProxyServer::call_tool handled by builtin service"
        );
        return match result {
            Ok(call_result) => {
                if builtin_tool_requires_runtime_refresh(request.name.as_ref()) {
                    if let Some(session_id) = client.session_id.as_deref() {
                        server
                            .refresh_bound_session_runtime_identity(session_id, &client.client_id)
                            .await?;
                    }
                }
                Ok(call_result)
            }
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

    let directly_exposed = if matches!(client.config_mode.as_deref(), Some("unify")) {
        let db = server
            .database
            .as_ref()
            .ok_or_else(|| McpError::internal_error("Database not available for tool calling".to_string(), None))?;
        let eligible_server_ids = load_unify_direct_exposure_eligible_server_ids(db).await?;
        crate::core::proxy::server::unify_directly_exposed_tool_allowed(
            client.unify_workspace.as_ref(),
            &eligible_server_ids,
            &server_id,
            &original_tool_name,
        )
    } else {
        false
    };

    if !direct_managed_tool_call_allowed(client.config_mode.as_deref(), directly_exposed) {
        tracing::warn!(
            call_id = %call_id,
            tool = %request.name,
            client_id = %client.client_id,
            profile_id = ?client.profile_id,
            "ProxyServer::call_tool denied direct managed tool call in unify mode"
        );
        return Err(McpError::invalid_params(
            format!(
                "Tool '{}' is not available for direct proxy calls in unify mode",
                request.name
            ),
            None,
        ));
    }

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
        .resolve_snapshot_for_client(&client)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    if let Err(error) = vis.assert_tool_allowed_with_snapshot(&snapshot, &request.name).await {
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
                    if let Some((iid0, _st, _res, _prm, peer)) =
                        instances.iter().find(|(candidate_id, _st, _res, _prm, peer)| {
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
    let mut options = PeerRequestOptions::no_options();
    options.timeout = Some(std::time::Duration::from_secs(call_timeout_secs));
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
    use super::{builtin_tool_allowed, direct_managed_tool_call_allowed};
    use crate::clients::models::{CapabilitySource, UnifyDirectExposureConfig, UnifyDirectToolSurface, UnifyRouteMode};
    use std::collections::HashSet;

    #[test]
    fn hosted_profile_listing_tools_are_only_available_for_profiles_source() {
        let other_tools = ["mcpmate_profile_list", "mcpmate_profile_preview"];

        for tool in other_tools {
            assert!(
                builtin_tool_allowed(None, CapabilitySource::Profiles, tool),
                "{tool} should be available for Profiles"
            );
            assert!(
                !builtin_tool_allowed(None, CapabilitySource::Activated, tool),
                "{tool} should NOT be available for Activated"
            );
            assert!(
                !builtin_tool_allowed(None, CapabilitySource::Custom, tool),
                "{tool} should NOT be available for Custom"
            );
        }
    }

    #[test]
    fn unknown_builtin_tools_are_not_exposed_outside_unify_allowlist() {
        let other_tools = ["some_other_tool", "another_mcpmate_service"];

        for tool in other_tools {
            assert!(!builtin_tool_allowed(None, CapabilitySource::Activated, tool));
            assert!(!builtin_tool_allowed(None, CapabilitySource::Profiles, tool));
            assert!(!builtin_tool_allowed(None, CapabilitySource::Custom, tool));
            assert!(!builtin_tool_allowed(
                Some("transparent"),
                CapabilitySource::Profiles,
                tool
            ));
            assert!(!builtin_tool_allowed(Some("unify"), CapabilitySource::Profiles, tool));
        }
    }

    #[test]
    fn hosted_mode_does_not_expose_scope_get_or_custom_detail_tools() {
        let hosted_only_denied = ["mcpmate_scope_get", "mcpmate_client_custom_profile_details"];

        for tool in hosted_only_denied {
            assert!(!builtin_tool_allowed(None, CapabilitySource::Activated, tool));
            assert!(!builtin_tool_allowed(None, CapabilitySource::Profiles, tool));
            assert!(!builtin_tool_allowed(None, CapabilitySource::Custom, tool));
        }
    }

    #[test]
    fn profile_enablement_tools_are_not_exposed_in_hosted_or_transparent_modes() {
        let profile_tools = [
            "mcpmate_profile_enable",
            "mcpmate_profile_disable",
            "mcpmate_profile_activate_only",
        ];

        for tool in profile_tools {
            assert!(!builtin_tool_allowed(None, CapabilitySource::Activated, tool));
            assert!(!builtin_tool_allowed(None, CapabilitySource::Profiles, tool));
            assert!(!builtin_tool_allowed(None, CapabilitySource::Custom, tool));
            assert!(!builtin_tool_allowed(
                Some("transparent"),
                CapabilitySource::Activated,
                tool
            ));
        }
    }

    #[test]
    fn transparent_mode_exposes_no_runtime_builtin_tools() {
        let transparent_denied = [
            "mcpmate_profile_list",
            "mcpmate_profile_preview",
            "mcpmate_scope_set",
            "mcpmate_scope_add",
            "mcpmate_scope_remove",
            "mcpmate_ucan_catalog",
            "mcpmate_ucan_details",
            "mcpmate_ucan_call",
        ];

        for tool in transparent_denied {
            assert!(!builtin_tool_allowed(
                Some("transparent"),
                CapabilitySource::Profiles,
                tool
            ));
        }
    }

    #[test]
    fn hosted_mode_exposes_only_profile_range_adjustment_tools_for_profiles_source() {
        let tool = "mcpmate_scope_set";

        assert!(!builtin_tool_allowed(None, CapabilitySource::Activated, tool));
        assert!(builtin_tool_allowed(None, CapabilitySource::Profiles, tool));
        assert!(!builtin_tool_allowed(None, CapabilitySource::Custom, tool));
    }

    #[test]
    fn client_profiles_tools_are_only_available_for_profiles_source() {
        let profiles_tools = ["mcpmate_scope_set", "mcpmate_scope_add", "mcpmate_scope_remove"];

        for tool in profiles_tools {
            assert!(
                !builtin_tool_allowed(None, CapabilitySource::Activated, tool),
                "{tool} should NOT be available for Activated"
            );
            assert!(
                builtin_tool_allowed(None, CapabilitySource::Profiles, tool),
                "{tool} should be available for Profiles"
            );
            assert!(
                !builtin_tool_allowed(None, CapabilitySource::Custom, tool),
                "{tool} should NOT be available for Custom"
            );
        }
    }

    #[test]
    fn client_custom_profile_details_is_only_available_for_custom_source() {
        let tool = "mcpmate_client_custom_profile_details";

        assert!(
            !builtin_tool_allowed(None, CapabilitySource::Activated, tool),
            "{tool} should NOT be available for Activated"
        );
        assert!(
            !builtin_tool_allowed(None, CapabilitySource::Profiles, tool),
            "{tool} should NOT be available for Profiles"
        );
        assert!(
            !builtin_tool_allowed(None, CapabilitySource::Custom, tool),
            "{tool} should NOT be available for Custom in hosted mode"
        );
    }

    #[test]
    fn unify_mode_only_exposes_ucan_tools() {
        assert!(builtin_tool_allowed(
            Some("unify"),
            CapabilitySource::Profiles,
            "mcpmate_ucan_catalog"
        ));
        assert!(builtin_tool_allowed(
            Some("unify"),
            CapabilitySource::Profiles,
            "mcpmate_ucan_details"
        ));
        assert!(builtin_tool_allowed(
            Some("unify"),
            CapabilitySource::Profiles,
            "mcpmate_ucan_call"
        ));
        assert!(!builtin_tool_allowed(
            Some("unify"),
            CapabilitySource::Profiles,
            "mcpmate_scope_set"
        ));
        assert!(!builtin_tool_allowed(
            Some("unify"),
            CapabilitySource::Profiles,
            "mcpmate_scope_add"
        ));
        assert!(!builtin_tool_allowed(
            Some("unify"),
            CapabilitySource::Profiles,
            "mcpmate_scope_remove"
        ));
        assert!(!builtin_tool_allowed(
            Some("unify"),
            CapabilitySource::Profiles,
            "mcpmate_profile_enable"
        ));
        assert!(!builtin_tool_allowed(
            Some("unify"),
            CapabilitySource::Profiles,
            "mcpmate_profile_list"
        ));
        assert!(!builtin_tool_allowed(
            Some("unify"),
            CapabilitySource::Custom,
            "mcpmate_client_custom_profile_details"
        ));
    }

    #[test]
    fn unify_mode_blocks_direct_managed_tool_calls_but_other_modes_keep_current_proxy_path() {
        assert!(!direct_managed_tool_call_allowed(Some("unify"), false));
        assert!(direct_managed_tool_call_allowed(Some("unify"), true));
        assert!(direct_managed_tool_call_allowed(None, false));
        assert!(direct_managed_tool_call_allowed(Some("hosted"), false));
        assert!(direct_managed_tool_call_allowed(Some("transparent"), false));
    }

    #[test]
    fn unify_direct_exposure_broker_only_keeps_all_tools_brokered() {
        let workspace = UnifyDirectExposureConfig {
            route_mode: UnifyRouteMode::BrokerOnly,
            selected_server_ids: vec!["server-a".to_string()],
            selected_tool_surfaces: vec![UnifyDirectToolSurface {
                server_id: "server-a".to_string(),
                tool_name: "tool-one".to_string(),
            }],
            selected_prompt_surfaces: Vec::new(),
            selected_resource_surfaces: Vec::new(),
            selected_template_surfaces: Vec::new(),
        };
        let eligible_server_ids = HashSet::from(["server-a".to_string()]);

        assert!(!crate::core::proxy::server::unify_directly_exposed_tool_allowed(
            Some(&workspace),
            &eligible_server_ids,
            "server-a",
            "tool-one",
        ));
    }

    #[test]
    fn unify_direct_exposure_server_live_only_exposes_selected_eligible_servers() {
        let workspace = UnifyDirectExposureConfig {
            route_mode: UnifyRouteMode::ServerLive,
            selected_server_ids: vec!["server-a".to_string()],
            selected_tool_surfaces: Vec::new(),
            selected_prompt_surfaces: Vec::new(),
            selected_resource_surfaces: Vec::new(),
            selected_template_surfaces: Vec::new(),
        };
        let eligible_server_ids = HashSet::from(["server-a".to_string()]);

        assert!(crate::core::proxy::server::unify_directly_exposed_tool_allowed(
            Some(&workspace),
            &eligible_server_ids,
            "server-a",
            "tool-one",
        ));
        assert!(!crate::core::proxy::server::unify_directly_exposed_tool_allowed(
            Some(&workspace),
            &eligible_server_ids,
            "server-b",
            "tool-one",
        ));
        assert!(!crate::core::proxy::server::unify_directly_exposed_tool_allowed(
            Some(&workspace),
            &HashSet::new(),
            "server-a",
            "tool-one",
        ));
        assert!(crate::core::proxy::server::unify_directly_exposed_server_allowed(
            Some(&workspace),
            &eligible_server_ids,
            "server-a",
        ));
        assert!(!crate::core::proxy::server::unify_directly_exposed_server_allowed(
            Some(&workspace),
            &eligible_server_ids,
            "server-b",
        ));
    }

    #[test]
    fn unify_direct_exposure_capability_level_only_exposes_selected_tools() {
        let workspace = UnifyDirectExposureConfig {
            route_mode: UnifyRouteMode::CapabilityLevel,
            selected_server_ids: vec!["server-a".to_string()],
            selected_tool_surfaces: vec![UnifyDirectToolSurface {
                server_id: "server-a".to_string(),
                tool_name: "tool-one".to_string(),
            }],
            selected_prompt_surfaces: Vec::new(),
            selected_resource_surfaces: Vec::new(),
            selected_template_surfaces: Vec::new(),
        };
        let eligible_server_ids = HashSet::from(["server-a".to_string()]);

        assert!(crate::core::proxy::server::unify_directly_exposed_tool_allowed(
            Some(&workspace),
            &eligible_server_ids,
            "server-a",
            "tool-one",
        ));
        assert!(!crate::core::proxy::server::unify_directly_exposed_tool_allowed(
            Some(&workspace),
            &eligible_server_ids,
            "server-a",
            "tool-two",
        ));
    }
}
