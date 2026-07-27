use super::*;
use crate::mcper::builtin::ClientBuiltinContext;
use crate::mcper::{
    MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL, MCPMATE_PROFILE_ADD_TOOL, MCPMATE_PROFILE_DETAILS_TOOL,
    MCPMATE_PROFILE_GET_TOOL, MCPMATE_PROFILE_LIST_TOOL, MCPMATE_PROFILE_REMOVE_TOOL, MCPMATE_PROFILE_SET_TOOL,
    MCPMATE_UCAN_CALL_TOOL, MCPMATE_UCAN_CATALOG_TOOL, MCPMATE_UCAN_DETAILS_TOOL,
};
use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolRequest, CallToolRequestParams, CallToolResult, ClientRequest, PaginatedRequestParams};
use rmcp::service::PeerRequestOptions;
use rmcp::service::RequestContext;

#[cfg(test)]
fn builtin_tool_allowed(
    config_mode: Option<&str>,
    capability_source: crate::clients::models::CapabilitySource,
    tool_name: &str,
) -> bool {
    crate::mcper::builtin::names::builtin_tool_names_for_surface(
        config_mode.unwrap_or("hosted"),
        capability_source.as_str(),
    )
    .contains(&tool_name)
}

#[cfg(test)]
fn direct_managed_tool_call_allowed(
    config_mode: Option<&str>,
    directly_exposed: bool,
) -> bool {
    !matches!(config_mode, Some("unify")) || directly_exposed
}

fn client_aware_builtin_tool_requires_runtime_refresh(tool_name: &str) -> bool {
    matches!(
        tool_name,
        MCPMATE_PROFILE_SET_TOOL | MCPMATE_PROFILE_ADD_TOOL | MCPMATE_PROFILE_REMOVE_TOOL
    )
}

fn builtin_tool_requires_runtime_refresh(_tool_name: &str) -> bool {
    false
}

pub(super) async fn list_tools(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParams>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<rmcp::model::ListToolsResult, McpError> {
    let client = server.resolve_bound_client_context(&_context).await?;
    let surface = server.load_active_surface(&client).await?;
    let page = server.paginator.paginate_tools(&_request, surface.tools())?;

    tracing::info!(
        total = page.items.len(),
        has_next = page.next_cursor.is_some(),
        consumer_id = %surface.consumer_id,
        publication_id = %surface.publication_id,
        generation = surface.generation,
        "Proxy listed tools from active Surface publication"
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
    let surface_entry = server
        .require_active_surface_entry(
            &client,
            mcpmate_capability_store::CapabilityKind::Tools,
            request.name.as_ref(),
        )
        .await?;
    let is_builtin = surface_entry.source_server_id == mcpmate_capability_store::BUILTIN_CAPABILITY_SOURCE_ID;
    let call_id = crate::generate_id!("tcall");
    let started_at = std::time::Instant::now();

    tracing::debug!(
        call_id = %call_id,
        tool = %request.name,
        "ProxyServer::call_tool received request"
    );

    let is_profile_tool = matches!(
        request.name.as_ref(),
        MCPMATE_PROFILE_LIST_TOOL | MCPMATE_PROFILE_DETAILS_TOOL
    );
    let is_client_tool = matches!(
        request.name.as_ref(),
        MCPMATE_PROFILE_GET_TOOL
            | MCPMATE_PROFILE_SET_TOOL
            | MCPMATE_PROFILE_ADD_TOOL
            | MCPMATE_PROFILE_REMOVE_TOOL
            | MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL
            | MCPMATE_UCAN_CATALOG_TOOL
            | MCPMATE_UCAN_DETAILS_TOOL
            | MCPMATE_UCAN_CALL_TOOL
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

        if is_client_tool && is_builtin {
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

    if is_builtin && let Some(result) = server.builtin_services.call_tool(&request).await {
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

    let server_id = surface_entry.source_server_id;
    let original_tool_name = surface_entry.upstream_key;
    let server_name: String = sqlx::query_scalar("SELECT name FROM server_config WHERE id = ?")
        .bind(&server_id)
        .fetch_one(&server.database.as_ref().expect("database checked above").pool)
        .await
        .map_err(|e| {
            tracing::error!(
                call_id = %call_id,
                tool = %request.name,
                error = %e,
                "ProxyServer::call_tool failed to resolve pinned source server"
            );
            McpError::internal_error(format!("Failed to resolve pinned source server: {e}"), None)
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

    // Resolve tool call timeout from env (fallback 30s)
    let call_timeout_secs: u64 = std::env::var("MCPMATE_TOOL_CALL_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);

    // Acquire upstream peer (ensure connected if necessary)
    let (peer_opt, mut instance_id_opt) = {
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
        instance_id_opt = instance_id_opt.or_else(|| Some(iid.clone()));
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
        Ok(rmcp::model::ServerResult::CallToolResult(mut result)) => {
            let database = server.database.as_ref().ok_or_else(|| {
                McpError::internal_error("Tool result projection requires registry metadata".to_string(), None)
            })?;
            crate::core::capability::resource_uri::rewrite_call_tool_result(
                &database.pool,
                &server_id,
                &server_name,
                &mut result,
            )
            .await
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
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
            let error_str = e.to_string();
            tracing::error!(
                call_id = %call_id,
                tool = %request.name,
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                error = %error_str,
                "ProxyServer::call_tool upstream error"
            );
            if let Some(database) = server.database.as_ref() {
                crate::core::capability::runtime::record_capability_usage_evidence(
                    database,
                    &server_id,
                    mcpmate_capability_store::CapabilityKind::Tools,
                    instance_id_opt.as_deref(),
                    &error_str,
                )
                .await;
            }
            Err(McpError::internal_error(error_str, None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{builtin_tool_allowed, direct_managed_tool_call_allowed};
    use crate::clients::models::{CapabilitySource, UnifyDirectExposureConfig, UnifyDirectToolSurface, UnifyRouteMode};
    use crate::mcper::{
        MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL, MCPMATE_PROFILE_ADD_TOOL, MCPMATE_PROFILE_DETAILS_TOOL,
        MCPMATE_PROFILE_GET_TOOL, MCPMATE_PROFILE_LIST_TOOL, MCPMATE_PROFILE_REMOVE_TOOL, MCPMATE_PROFILE_SET_TOOL,
        MCPMATE_UCAN_CALL_TOOL, MCPMATE_UCAN_CATALOG_TOOL, MCPMATE_UCAN_DETAILS_TOOL,
    };
    use std::collections::HashSet;

    #[test]
    fn hosted_shared_discovery_tools_are_available_for_profiles_source() {
        let shared_tools = [
            MCPMATE_UCAN_CATALOG_TOOL,
            MCPMATE_UCAN_DETAILS_TOOL,
            MCPMATE_UCAN_CALL_TOOL,
        ];

        for tool in shared_tools {
            assert!(
                builtin_tool_allowed(None, CapabilitySource::Profiles, tool),
                "{tool} should be available for Profiles in hosted mode"
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
    fn hosted_mode_exposes_profile_get_for_profiles_source() {
        assert!(!builtin_tool_allowed(
            None,
            CapabilitySource::Activated,
            MCPMATE_PROFILE_GET_TOOL
        ));
        assert!(builtin_tool_allowed(
            None,
            CapabilitySource::Profiles,
            MCPMATE_PROFILE_GET_TOOL
        ));
        assert!(!builtin_tool_allowed(
            None,
            CapabilitySource::Custom,
            MCPMATE_PROFILE_GET_TOOL
        ));
    }

    #[test]
    fn hosted_mode_does_not_expose_custom_detail_tools() {
        assert!(!builtin_tool_allowed(
            None,
            CapabilitySource::Activated,
            MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL
        ));
        assert!(!builtin_tool_allowed(
            None,
            CapabilitySource::Profiles,
            MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL
        ));
        assert!(!builtin_tool_allowed(
            None,
            CapabilitySource::Custom,
            MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL
        ));
    }

    #[test]
    fn legacy_profile_list_and_details_are_not_exposed_in_any_mode() {
        let legacy_tools = [MCPMATE_PROFILE_LIST_TOOL, MCPMATE_PROFILE_DETAILS_TOOL];

        for tool in legacy_tools {
            assert!(!builtin_tool_allowed(None, CapabilitySource::Profiles, tool));
            assert!(!builtin_tool_allowed(Some("unify"), CapabilitySource::Profiles, tool));
            assert!(!builtin_tool_allowed(
                Some("transparent"),
                CapabilitySource::Profiles,
                tool
            ));
        }
    }

    #[test]
    fn transparent_mode_exposes_no_runtime_builtin_tools() {
        let transparent_denied = [
            MCPMATE_UCAN_CATALOG_TOOL,
            MCPMATE_UCAN_DETAILS_TOOL,
            MCPMATE_UCAN_CALL_TOOL,
            MCPMATE_PROFILE_GET_TOOL,
            MCPMATE_PROFILE_SET_TOOL,
            MCPMATE_PROFILE_ADD_TOOL,
            MCPMATE_PROFILE_REMOVE_TOOL,
            MCPMATE_PROFILE_LIST_TOOL,
            MCPMATE_PROFILE_DETAILS_TOOL,
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
    fn hosted_mode_exposes_profile_set_only_for_profiles_source() {
        let tool = MCPMATE_PROFILE_SET_TOOL;

        assert!(!builtin_tool_allowed(None, CapabilitySource::Activated, tool));
        assert!(builtin_tool_allowed(None, CapabilitySource::Profiles, tool));
        assert!(!builtin_tool_allowed(None, CapabilitySource::Custom, tool));
    }

    #[test]
    fn client_profiles_tools_are_only_available_for_profiles_source() {
        let profiles_tools = [
            MCPMATE_PROFILE_SET_TOOL,
            MCPMATE_PROFILE_ADD_TOOL,
            MCPMATE_PROFILE_REMOVE_TOOL,
        ];

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
        let tool = MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL;

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
        let unify_allowed = [
            MCPMATE_UCAN_CATALOG_TOOL,
            MCPMATE_UCAN_DETAILS_TOOL,
            MCPMATE_UCAN_CALL_TOOL,
        ];
        for tool in unify_allowed {
            assert!(
                builtin_tool_allowed(Some("unify"), CapabilitySource::Profiles, tool),
                "{tool} should be available in unify mode"
            );
        }

        let unify_denied = [
            MCPMATE_PROFILE_GET_TOOL,
            MCPMATE_PROFILE_SET_TOOL,
            MCPMATE_PROFILE_ADD_TOOL,
            MCPMATE_PROFILE_REMOVE_TOOL,
            MCPMATE_PROFILE_LIST_TOOL,
            MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL,
        ];
        for tool in unify_denied {
            assert!(
                !builtin_tool_allowed(Some("unify"), CapabilitySource::Profiles, tool),
                "{tool} should NOT be available in unify mode"
            );
        }
    }

    #[test]
    fn hosted_mode_exposes_shared_discovery_and_profile_tools() {
        let hosted_allowed = [
            MCPMATE_UCAN_CATALOG_TOOL,
            MCPMATE_UCAN_DETAILS_TOOL,
            MCPMATE_UCAN_CALL_TOOL,
            MCPMATE_PROFILE_GET_TOOL,
            MCPMATE_PROFILE_SET_TOOL,
            MCPMATE_PROFILE_ADD_TOOL,
            MCPMATE_PROFILE_REMOVE_TOOL,
        ];
        for tool in hosted_allowed {
            assert!(
                builtin_tool_allowed(None, CapabilitySource::Profiles, tool),
                "{tool} should be available in hosted mode with Profiles source"
            );
        }

        let hosted_denied = [
            MCPMATE_PROFILE_LIST_TOOL,
            MCPMATE_PROFILE_DETAILS_TOOL,
            MCPMATE_CLIENT_CUSTOM_PROFILE_DETAILS_TOOL,
        ];
        for tool in hosted_denied {
            assert!(
                !builtin_tool_allowed(None, CapabilitySource::Profiles, tool),
                "{tool} should NOT be available in hosted mode"
            );
        }
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
    fn unify_direct_exposure_server_level_uses_materialized_surfaces() {
        let workspace = UnifyDirectExposureConfig {
            route_mode: UnifyRouteMode::ServerLevel,
            selected_server_ids: Vec::new(),
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
