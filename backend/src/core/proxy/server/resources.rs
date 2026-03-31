use super::*;
use crate::core::capability::naming::{NamingKind, generate_unique_name, resolve_unique_name};
use futures::StreamExt;
use rmcp::ErrorData as McpError;
use rmcp::model::{
    ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult,
};
use rmcp::service::RequestContext;
use std::collections::HashSet;

pub(super) async fn list_resources(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParams>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<ListResourcesResult, McpError> {
    let client = server.resolve_bound_client_context(&_context).await?;
    if matches!(client.config_mode.as_deref(), Some("smart")) {
        return Ok(ListResourcesResult {
            resources: Vec::new(),
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
    let mut resources: Vec<rmcp::model::Resource> = Vec::new();

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
                crate::core::capability::CapabilityType::Resources,
            ) {
                continue;
            }
            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::Resources,
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
                        if let Some(items) = result.items.into_resources() {
                            for mut r in items {
                                let unique_uri =
                                    generate_unique_name(NamingKind::Resource, &server_name_cloned, &r.uri);
                                r.raw.uri = unique_uri;
                                out.push(r);
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
            resources.append(&mut v);
        }
    }

    resources = vis.filter_resources_with_snapshot(&snapshot, resources, Vec::new()).0;

    // Apply pagination
    let page = server.paginator.paginate_resources(&_request, resources)?;

    tracing::info!(
        total = page.items.len(),
        has_next = page.next_cursor.is_some(),
        "Proxy listed resources"
    );

    Ok(ListResourcesResult {
        resources: page.items,
        next_cursor: page.next_cursor,
        ..Default::default()
    })
}

pub(super) async fn list_resource_templates(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParams>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<ListResourceTemplatesResult, McpError> {
    let client = server.resolve_bound_client_context(&_context).await?;
    if matches!(client.config_mode.as_deref(), Some("smart")) {
        return Ok(ListResourceTemplatesResult {
            resource_templates: Vec::new(),
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
    let Some(_db) = &server.database else {
        tracing::warn!("Database not available for server filtering; returning empty list");
        return Ok(ListResourceTemplatesResult {
            resource_templates: Vec::new(),
            next_cursor: None,
            ..Default::default()
        });
    };

    let mut resource_templates: Vec<rmcp::model::ResourceTemplate> = Vec::new();

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
                crate::core::capability::CapabilityType::ResourceTemplates,
            ) {
                continue;
            }
            let ctx = crate::core::capability::runtime::ListCtx {
                capability: crate::core::capability::CapabilityType::ResourceTemplates,
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
                        if let Some(items) = result.items.into_resource_templates() {
                            for mut t in items {
                                let unique = generate_unique_name(
                                    NamingKind::ResourceTemplate,
                                    &server_name_cloned,
                                    &t.uri_template,
                                );
                                t.raw.name = unique; // carry unique name for visibility filtering
                                out.push(t);
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
            resource_templates.append(&mut v);
        }
    }

    let resource_templates = vis
        .filter_resources_with_snapshot(&snapshot, Vec::new(), resource_templates)
        .1;

    // Apply pagination
    let page = server
        .paginator
        .paginate_resource_templates(&_request, resource_templates)?;

    tracing::info!(
        total = page.items.len(),
        has_next = page.next_cursor.is_some(),
        "Proxy listed resource templates"
    );

    Ok(ListResourceTemplatesResult {
        resource_templates: page.items,
        next_cursor: page.next_cursor,
        ..Default::default()
    })
}

pub(super) async fn read_resource(
    server: &ProxyServer,
    request: ReadResourceRequestParams,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<ReadResourceResult, McpError> {
    let client = server.resolve_bound_client_context(&_context).await?;
    if matches!(client.config_mode.as_deref(), Some("smart")) {
        return Err(McpError::invalid_params(
            "Smart mode does not expose resources directly; use UCAN broker tools instead".to_string(),
            None,
        ));
    }
    tracing::debug!("Reading resource: {}", request.uri);

    let mut lookup_uri = request.uri.clone();
    let mut server_filter: Option<String> = None;
    if server.database.is_some() {
        match resolve_unique_name(NamingKind::Resource, &request.uri).await {
            Ok((server_name, upstream_uri)) => {
                lookup_uri = upstream_uri;
                if let Ok(Some(server_id)) = crate::core::capability::resolver::to_id(&server_name).await {
                    server_filter = Some(server_id);
                }
            }
            Err(err) => {
                // Try scheme-based server hint: <scheme>://...
                let mut hinted: Option<String> = None;
                if let Some(pos) = request.uri.find("://") {
                    let scheme = &request.uri[..pos];
                    if let Ok(Some(sid)) = crate::core::capability::resolver::to_id(scheme).await {
                        hinted = Some(sid);
                    } else if let Some(db) = &server.database {
                        // Resolve by templates: find server that owns a template using this scheme
                        if let Ok(row) = sqlx::query_scalar::<_, String>(
                            "SELECT sc.id FROM server_resource_templates srt JOIN server_config sc ON sc.id=srt.server_id WHERE srt.uri_template LIKE ? LIMIT 1",
                        )
                        .bind(format!("{}://%", scheme))
                        .fetch_optional(&db.pool)
                        .await
                        {
                            hinted = row;
                        }
                    }
                }
                if let Some(sid) = hinted {
                    server_filter = Some(sid);
                } else {
                    tracing::trace!("Resource URI '{}' not unique; resolver error: {}", request.uri, err);
                }
            }
        }
    }

    let resource_mapping = if let Some(server_id) = server_filter.clone() {
        let mapping = {
            let mut filter = HashSet::new();
            filter.insert(server_id.clone());
            crate::core::capability::facade::build_resource_mapping_filtered(
                &server.connection_pool,
                server.database.as_ref(),
                Some(&filter),
            )
            .await
        };
        if mapping.contains_key(&lookup_uri) {
            mapping
        } else {
            crate::core::capability::facade::build_resource_mapping(&server.connection_pool, server.database.as_ref())
                .await
        }
    } else {
        crate::core::capability::facade::build_resource_mapping(&server.connection_pool, server.database.as_ref()).await
    };

    let canonical_uri = if resource_mapping.contains_key(&request.uri) {
        request.uri.clone()
    } else if let Some(mapping) = resource_mapping.get(&lookup_uri) {
        generate_unique_name(
            NamingKind::Resource,
            &mapping.server_name,
            &mapping.upstream_resource_uri,
        )
    } else {
        request.uri.clone()
    };

    let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
        server.database.clone(),
        server.profile_service.clone(),
    );
    let snapshot = vis
        .resolve_snapshot_for_client(&client)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
    if let Err(error) = vis
        .assert_resource_allowed_with_snapshot(&snapshot, &canonical_uri)
        .await
    {
        tracing::warn!(
            resource = %canonical_uri,
            client_id = %client.client_id,
            profile_id = ?client.profile_id,
            error = %error,
            "ProxyServer::read_resource denied by visibility policy"
        );
        return Err(McpError::invalid_params(
            format!("Resource '{}' is not available for this client", canonical_uri),
            None,
        ));
    }

    let connection_selection = server_filter
        .as_ref()
        .and_then(|server_id| client.connection_selection(server_id.clone()));

    match crate::core::capability::facade::read_upstream_resource(
        &server.connection_pool,
        &resource_mapping,
        &lookup_uri,
        server_filter.as_deref(),
        connection_selection.as_ref(),
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(e) => {
            tracing::error!("Failed to read resource '{}': {}", request.uri, e);
            Err(McpError::internal_error(e.to_string(), None))
        }
    }
}
