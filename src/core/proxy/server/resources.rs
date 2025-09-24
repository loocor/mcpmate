use super::*;
use crate::core::capability::naming::{NamingKind, generate_unique_name, resolve_unique_name};
use futures::StreamExt;
use rmcp::ErrorData as McpError;
use rmcp::model::{
    ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParam, ReadResourceRequestParam,
    ReadResourceResult,
};
use rmcp::service::RequestContext;
use std::collections::HashSet;

pub(super) async fn list_resources(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParam>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<ListResourcesResult, McpError> {
    let mut resources: Vec<rmcp::model::Resource> = Vec::new();

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

    // Apply centralized profile visibility filter (resources)
    let vis = crate::core::profile::visibility::ProfileVisibilityService::new(
        server.database.clone(),
        server.profile_service.clone(),
    );
    resources = vis.filter_resources(resources).await;

    tracing::info!("Proxy listed {} total resources", resources.len());

    Ok(ListResourcesResult {
        resources,
        next_cursor: None,
    })
}

pub(super) async fn list_resource_templates(
    server: &ProxyServer,
    _request: Option<PaginatedRequestParam>,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<ListResourceTemplatesResult, McpError> {
    let Some(_db) = &server.database else {
        tracing::warn!("Database not available for server filtering; returning empty list");
        return Ok(ListResourceTemplatesResult {
            resource_templates: Vec::new(),
            next_cursor: None,
        });
    };

    let mut resource_templates: Vec<rmcp::model::ResourceTemplate> = Vec::new();

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
        for (server_id, _server_name, capabilities) in enabled_servers {
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
            };
            let redb = redb.clone();
            let pool = pool.clone();
            let db = db.clone();
            tasks.push(async move {
                match crate::core::capability::runtime::list(&ctx, &redb, &pool, &db).await {
                    Ok(result) => result.items.into_resource_templates().unwrap_or_default(),
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

    tracing::info!("Proxy listed {} total resource templates", resource_templates.len());

    Ok(ListResourceTemplatesResult {
        resource_templates,
        next_cursor: None,
    })
}

pub(super) async fn read_resource(
    server: &ProxyServer,
    request: ReadResourceRequestParam,
    _context: RequestContext<rmcp::RoleServer>,
) -> Result<ReadResourceResult, McpError> {
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
                tracing::trace!(
                    "Resource URI '{}' does not require unique-name resolution (resolve error: {})",
                    request.uri,
                    err
                );
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

    match crate::core::capability::facade::read_upstream_resource(
        &server.connection_pool,
        &resource_mapping,
        &lookup_uri,
        server_filter.as_deref(),
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
