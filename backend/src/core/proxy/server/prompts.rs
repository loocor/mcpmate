use super::*;
use crate::core::capability::naming::{NamingKind, resolve_capability_route};
use crate::mcper::builtin::ClientBuiltinContext;
use futures::StreamExt;
use rmcp::ErrorData as McpError;
use rmcp::model::{GetPromptRequestParams, GetPromptResult, ListPromptsResult, PaginatedRequestParams};
use rmcp::service::RequestContext;
use std::collections::HashSet;

fn builtin_prompt_allowed(
    config_mode: Option<&str>,
    prompt_name: &str,
) -> bool {
    let _ = (config_mode, prompt_name);
    false
}

pub(super) async fn list_prompts(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParams>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<ListPromptsResult, McpError> {
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
    let visible_server_ids = snapshot.server_ids.iter().cloned().collect::<HashSet<_>>();
    let unify_direct_exposure_eligible_server_ids = if unify_mode {
        if let Some(db) = &server.database {
            crate::core::proxy::server::load_unify_direct_exposure_eligible_server_ids(db).await?
        } else {
            HashSet::new()
        }
    } else {
        HashSet::new()
    };
    let mut prompts: Vec<rmcp::model::Prompt> = Vec::new();

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
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;

        let redb = &server.redb_cache;
        let pool = &server.connection_pool;

        let mut tasks = Vec::new();
        for (server_id, server_name, capabilities) in enabled_servers {
            if !visible_server_ids.contains(&server_id) {
                continue;
            }
            if !super::supports_capability(
                capabilities.as_deref(),
                crate::core::capability::CapabilityType::Prompts,
            ) {
                continue;
            }
            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Prompts,
                server_id: server_id.clone(),
                refresh: Some(crate::core::capability::runtime::RefreshStrategy::CacheFirst),
                timeout: Some(std::time::Duration::from_secs(10)),
                validation_session: None,
                runtime_identity: client.runtime_identity(),
                connection_selection: client.connection_selection(server_id.clone()),
                name_domain: crate::core::capability::runtime::NameDomain::External,
            };
            let redb = redb.clone();
            let pool = pool.clone();
            let db = db.clone();
            let server_name_cloned = server_name.clone();
            tasks.push(async move {
                let prompts = crate::core::capability::runtime::list(&ctx, &redb, &pool, &db)
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| {
                        result
                            .items
                            .into_prompts()
                            .ok_or_else(|| "Prompt listing returned a different capability kind".to_string())
                    });
                (server_id, server_name_cloned, prompts)
            });
        }

        let mut aggregate = super::common::AggregateListStatus::new("prompts");
        for (server_id, server_name, result) in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            let prompt_batch = match result {
                Ok(prompt_batch) => prompt_batch,
                Err(error) => {
                    aggregate.record_failure(&server_id, &server_name, error);
                    continue;
                }
            };
            let server_prompts = async {
                if !unify_mode {
                    return Ok(prompt_batch);
                }
                let mut exposed = Vec::new();
                for prompt in prompt_batch {
                    let raw_prompt_name = crate::core::proxy::server::resolve_direct_surface_value(
                        NamingKind::Prompt,
                        &server_id,
                        prompt.name.as_ref(),
                    )
                    .await?;
                    if crate::core::proxy::server::unify_directly_exposed_prompt_allowed(
                        client.unify_workspace.as_ref(),
                        &unify_direct_exposure_eligible_server_ids,
                        &server_id,
                        raw_prompt_name.as_ref(),
                    ) {
                        exposed.push(prompt);
                    }
                }
                Ok::<_, anyhow::Error>(exposed)
            }
            .await;
            match server_prompts {
                Ok(server_prompts) => {
                    aggregate.record_success();
                    prompts.extend(server_prompts);
                }
                Err(error) => aggregate.record_failure(&server_id, &server_name, error),
            }
        }
        aggregate.finish()?;
    }

    prompts = vis.filter_prompts_with_snapshot(&snapshot, prompts);

    let builtin_prompts = server.builtin_services.prompts();
    tracing::debug!("Including {} builtin service prompts", builtin_prompts.len());
    prompts.extend(
        builtin_prompts
            .into_iter()
            .filter(|prompt| builtin_prompt_allowed(client.config_mode.as_deref(), prompt.name.as_ref())),
    );

    // Apply pagination
    let page = server.paginator.paginate_prompts(&_request, prompts)?;

    tracing::info!(
        total = page.items.len(),
        has_next = page.next_cursor.is_some(),
        "Proxy listed prompts"
    );

    Ok(ListPromptsResult {
        prompts: page.items,
        next_cursor: page.next_cursor,
        ..Default::default()
    })
}

pub(super) async fn get_prompt(
    server: &ProxyServer,
    request: GetPromptRequestParams,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<GetPromptResult, McpError> {
    let client = server.resolve_bound_client_context(&_context).await?;
    let unify_mode = matches!(client.config_mode.as_deref(), Some("unify"));
    tracing::debug!("Getting prompt: {}", request.name);

    let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
        server.database.clone(),
        server.profile_service.clone(),
    );
    let capability_config = vis
        .resolve_capability_config_for_client(&client)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let builtin_context = ClientBuiltinContext {
        client_id: client.client_id.clone(),
        session_id: client.session_id.clone(),
        config_mode: client.config_mode.clone(),
        capability_source: capability_config.capability_source,
        selected_profile_ids: capability_config.selected_profile_ids,
        custom_profile_id: capability_config.custom_profile_id,
        unify_workspace: client.unify_workspace.clone(),
    };

    if builtin_prompt_allowed(client.config_mode.as_deref(), request.name.as_ref()) {
        if let Some(result) = server
            .builtin_services
            .get_prompt_with_context(&request, Some(&builtin_context))
            .await
        {
            return result.map_err(|e| McpError::internal_error(e.to_string(), None));
        }
    }

    let route = resolve_capability_route(NamingKind::Prompt, &request.name)
        .await
        .map_err(|error| McpError::internal_error(format!("Failed to resolve external prompt name: {error}"), None))?;
    let server_filter = route.server_id;
    let lookup_name = route.upstream_value;
    let canonical_name = request.name.clone();
    let mut filter = HashSet::new();
    filter.insert(server_filter.clone());
    let prompt_mapping =
        crate::core::capability::facade::build_prompt_mapping_filtered(&server.connection_pool, Some(&filter))
            .await
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
    if !prompt_mapping.contains_key(&lookup_name) {
        return Err(McpError::invalid_params(
            format!(
                "Prompt '{}' is not available from its routed upstream server",
                canonical_name
            ),
            None,
        ));
    }

    if unify_mode {
        let Some(db) = &server.database else {
            return Err(McpError::invalid_params(
                "Unify prompt direct exposure requires database-backed server metadata".to_string(),
                None,
            ));
        };
        let eligible_server_ids =
            crate::core::proxy::server::load_unify_direct_exposure_eligible_server_ids(db).await?;
        if !crate::core::proxy::server::unify_directly_exposed_prompt_allowed(
            client.unify_workspace.as_ref(),
            &eligible_server_ids,
            &server_filter,
            lookup_name.as_ref(),
        ) {
            return Err(McpError::invalid_params(
                format!("Prompt '{}' is not directly exposed for this client", canonical_name),
                None,
            ));
        }
    }

    let snapshot = vis
        .resolve_snapshot_for_client(&client)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    if let Err(error) = vis
        .assert_prompt_allowed_with_snapshot(&snapshot, &canonical_name)
        .await
    {
        tracing::warn!(
            prompt = %canonical_name,
            client_id = %client.client_id,
            profile_id = ?client.profile_id,
            error = %error,
            "ProxyServer::get_prompt denied by visibility policy"
        );
        return Err(McpError::invalid_params(
            format!("Prompt '{}' is not available for this client", canonical_name),
            None,
        ));
    }

    let connection_selection = client.connection_selection(server_filter.clone());

    match crate::core::capability::facade::get_upstream_prompt(
        &server.connection_pool,
        &prompt_mapping,
        &lookup_name,
        request.arguments,
        Some(&server_filter),
        connection_selection.as_ref(),
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::error!("Failed to get prompt '{}': {}", request.name, e);
            Err(McpError::internal_error(e.to_string(), None))
        }
    }
}
