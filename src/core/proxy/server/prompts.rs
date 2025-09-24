use super::*;
use crate::core::capability::naming::{NamingKind, generate_unique_name, resolve_unique_name};
use futures::StreamExt;
use rmcp::ErrorData as McpError;
use rmcp::model::{GetPromptRequestParam, GetPromptResult, ListPromptsResult, PaginatedRequestParam};
use rmcp::service::RequestContext;
use std::collections::HashSet;

pub(super) async fn list_prompts(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParam>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<ListPromptsResult, McpError> {
    let mut prompts: Vec<rmcp::model::Prompt> = Vec::new();

    if let Some(db) = &server.database {
        let enabled_servers: Vec<(String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT sc.id, sc.name, sc.capabilities
            FROM server_config sc
            JOIN profile_server ps ON ps.server_id = sc.id AND ps.enabled = 1
            JOIN profile p ON p.id = ps.profile_id AND p.is_active = 1
            WHERE sc.enabled = 1
            GROUP BY sc.id, sc.name, sc.capabilities
            "#,
        )
        .fetch_all(&db.pool)
        .await
        .unwrap_or_default();

        let redb = &server.redb_cache;
        let pool = &server.connection_pool;

        let mut tasks = Vec::new();
        for (server_id, server_name, capabilities) in enabled_servers {
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

    // Apply centralized profile visibility filter (prompts)
    let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
        server.database.clone(),
        server.profile_service.clone(),
    );
    prompts = vis.filter_prompts(prompts).await;

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
    })
}

pub(super) async fn get_prompt(
    server: &ProxyServer,
    request: GetPromptRequestParam,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<GetPromptResult, McpError> {
    tracing::debug!("Getting prompt: {}", request.name);

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

    match crate::core::capability::facade::get_upstream_prompt(
        &server.connection_pool,
        &prompt_mapping,
        &lookup_name,
        request.arguments,
        server_filter.as_deref(),
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
