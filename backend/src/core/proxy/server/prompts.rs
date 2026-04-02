use super::*;
use crate::core::capability::naming::{NamingKind, generate_unique_name, resolve_unique_name};
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
    if matches!(client.config_mode.as_deref(), Some("unify")) {
        return Ok(ListPromptsResult {
            prompts: Vec::new(),
            next_cursor: None,
            ..Default::default()
        });
    }
    let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
        server.database.clone(),
        server.profile_service.clone(),
    );
    let snapshot = vis
        .resolve_snapshot_for_client(&client)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let visible_server_ids = snapshot.server_ids.iter().cloned().collect::<HashSet<_>>();
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
        .unwrap_or_default();

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
            };
            let redb = redb.clone();
            let pool = pool.clone();
            let db = db.clone();
            let server_name_cloned = server_name.clone();
            tasks.push(async move {
                match crate::core::capability::runtime::list(&ctx, &redb, &pool, &db).await {
                    Ok(result) => {
                        let mut out = Vec::new();
                        if let Some(items) = result.items.into_prompts() {
                            for mut p in items {
                                let unique_name =
                                    generate_unique_name(NamingKind::Prompt, &server_name_cloned, &p.name);
                                p.name = unique_name;
                                out.push(p);
                            }
                        }
                        out
                    }
                    Err(_) => Vec::new(),
                }
            });
        }

        for mut v in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            prompts.append(&mut v);
        }
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
    if matches!(client.config_mode.as_deref(), Some("unify")) {
        return Err(McpError::invalid_params(
            "Unify mode does not expose prompts directly; use UCAN broker tools instead".to_string(),
            None,
        ));
    }
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

    let mut lookup_name = request.name.clone();
    let mut server_filter: Option<String> = None;
    if server.database.is_some() {
        match resolve_unique_name(NamingKind::Prompt, &request.name).await {
            Ok((server_name, upstream_name)) => {
                lookup_name = upstream_name;
                if let Ok(Some(server_id)) = crate::core::capability::resolver::to_id(&server_name).await {
                    server_filter = Some(server_id);
                }
            }
            Err(err) => {
                tracing::trace!(
                    "Prompt '{}' does not require unique-name resolution (resolve error: {})",
                    request.name,
                    err
                );
            }
        }
    }

    let prompt_mapping = if let Some(server_id) = server_filter.clone() {
        let mapping = {
            let mut filter = HashSet::new();
            filter.insert(server_id.clone());
            crate::core::capability::facade::build_prompt_mapping_filtered(&server.connection_pool, Some(&filter)).await
        };
        if mapping.contains_key(&lookup_name) {
            mapping
        } else {
            crate::core::capability::facade::build_prompt_mapping(&server.connection_pool).await
        }
    } else {
        crate::core::capability::facade::build_prompt_mapping(&server.connection_pool).await
    };

    let canonical_name = if prompt_mapping.contains_key(&request.name) {
        request.name.clone()
    } else if let Some(mapping) = prompt_mapping.get(&lookup_name) {
        generate_unique_name(NamingKind::Prompt, &mapping.server_name, &mapping.upstream_prompt_name)
    } else {
        request.name.clone()
    };

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

    let connection_selection = server_filter
        .as_ref()
        .and_then(|server_id| client.connection_selection(server_id.clone()));

    match crate::core::capability::facade::get_upstream_prompt(
        &server.connection_pool,
        &prompt_mapping,
        &lookup_name,
        request.arguments,
        server_filter.as_deref(),
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
