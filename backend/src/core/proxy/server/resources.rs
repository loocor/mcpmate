use super::*;
use crate::core::capability::naming::{NamingKind, resolve_capability_route};
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
    let mut resources: Vec<rmcp::model::Resource> = Vec::new();
    let mut aggregate = crate::core::capability::aggregate::AggregateListStatus::new("resources");

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
                name_domain: crate::core::capability::runtime::NameDomain::External,
            };
            let redb = redb.clone();
            let pool = pool.clone();
            let db = db.clone();
            tasks.push(async move {
                let resources = crate::core::capability::runtime::list(&ctx, &redb, &pool, &db)
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| {
                        result
                            .items
                            .into_resources()
                            .ok_or_else(|| "Resource listing returned a different capability kind".to_string())
                    });
                (server_id, server_name, resources)
            });
        }

        for (server_id, server_name, result) in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            let resource_batch = match result {
                Ok(resource_batch) => resource_batch,
                Err(error) => {
                    aggregate.record_failure(&server_id, &server_name, error);
                    continue;
                }
            };
            let server_resources = async {
                if !unify_mode {
                    return Ok(resource_batch);
                }
                let mut exposed = Vec::new();
                for resource in resource_batch {
                    let raw_resource_uri = crate::core::proxy::server::resolve_direct_surface_value(
                        NamingKind::Resource,
                        &server_id,
                        resource.uri.as_ref(),
                    )
                    .await?;
                    if crate::core::proxy::server::unify_directly_exposed_resource_allowed(
                        client.unify_workspace.as_ref(),
                        &unify_direct_exposure_eligible_server_ids,
                        &server_id,
                        raw_resource_uri.as_ref(),
                    ) {
                        exposed.push(resource);
                    }
                }
                Ok::<_, anyhow::Error>(exposed)
            }
            .await;
            match server_resources {
                Ok(server_resources) => {
                    aggregate.record_success();
                    resources.extend(server_resources);
                }
                Err(error) => aggregate.record_failure(&server_id, &server_name, error),
            }
        }
    }

    resources = vis.filter_resources_with_snapshot(&snapshot, resources, Vec::new()).0;
    aggregate
        .finish_for_result(!resources.is_empty())
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;

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
    let Some(db_ref) = &server.database else {
        tracing::warn!("Database not available for server filtering; returning empty list");
        return Ok(ListResourceTemplatesResult {
            resource_templates: Vec::new(),
            next_cursor: None,
            ..Default::default()
        });
    };
    let unify_direct_exposure_eligible_server_ids = if unify_mode {
        crate::core::proxy::server::load_unify_direct_exposure_eligible_server_ids(db_ref).await?
    } else {
        HashSet::new()
    };

    let mut resource_templates: Vec<rmcp::model::ResourceTemplate> = Vec::new();
    let mut aggregate = crate::core::capability::aggregate::AggregateListStatus::new("resource templates");

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
                name_domain: crate::core::capability::runtime::NameDomain::External,
            };
            let redb = redb.clone();
            let pool = pool.clone();
            let db = db.clone();
            tasks.push(async move {
                let templates = crate::core::capability::runtime::list(&ctx, &redb, &pool, &db)
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| {
                        result
                            .items
                            .into_resource_templates()
                            .ok_or_else(|| "Resource template listing returned a different capability kind".to_string())
                    });
                (server_id, server_name, templates)
            });
        }

        for (server_id, server_name, result) in futures::stream::iter(tasks)
            .buffer_unordered(crate::core::capability::facade::concurrency_limit())
            .collect::<Vec<_>>()
            .await
        {
            let template_batch = match result {
                Ok(template_batch) => template_batch,
                Err(error) => {
                    aggregate.record_failure(&server_id, &server_name, error);
                    continue;
                }
            };
            let server_templates = async {
                if !unify_mode {
                    return Ok(template_batch);
                }
                let mut exposed = Vec::new();
                for resource_template in template_batch {
                    let raw_uri_template = crate::core::proxy::server::resolve_direct_surface_value(
                        NamingKind::ResourceTemplate,
                        &server_id,
                        resource_template.name.as_ref(),
                    )
                    .await?;
                    if crate::core::proxy::server::unify_directly_exposed_template_allowed(
                        client.unify_workspace.as_ref(),
                        &unify_direct_exposure_eligible_server_ids,
                        &server_id,
                        &raw_uri_template,
                    ) {
                        exposed.push(resource_template);
                    }
                }
                Ok::<_, anyhow::Error>(exposed)
            }
            .await;
            match server_templates {
                Ok(server_templates) => {
                    aggregate.record_success();
                    resource_templates.extend(server_templates);
                }
                Err(error) => aggregate.record_failure(&server_id, &server_name, error),
            }
        }
    }

    let resource_templates = vis
        .filter_resources_with_snapshot(&snapshot, Vec::new(), resource_templates)
        .1;
    aggregate
        .finish_for_result(!resource_templates.is_empty())
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;

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
    let unify_mode = matches!(client.config_mode.as_deref(), Some("unify"));
    tracing::debug!("Reading resource: {}", request.uri);

    let route = resolve_capability_route(NamingKind::Resource, &request.uri)
        .await
        .map_err(|error| McpError::internal_error(format!("Failed to resolve external resource URI: {error}"), None))?;
    let server_filter = route.server_id;
    let lookup_uri = route.upstream_value;
    let canonical_uri = request.uri.clone();
    let mut filter = HashSet::new();
    filter.insert(server_filter.clone());
    let resource_mapping = crate::core::capability::facade::build_resource_mapping_filtered(
        &server.connection_pool,
        server.database.as_ref(),
        Some(&filter),
    )
    .await
    .map_err(|error| McpError::internal_error(error.to_string(), None))?;

    if unify_mode {
        let Some(db) = &server.database else {
            return Err(McpError::invalid_params(
                "Unify resource direct exposure requires database-backed server metadata".to_string(),
                None,
            ));
        };
        let eligible_server_ids =
            crate::core::proxy::server::load_unify_direct_exposure_eligible_server_ids(db).await?;
        if !crate::core::proxy::server::unify_directly_exposed_resource_allowed(
            client.unify_workspace.as_ref(),
            &eligible_server_ids,
            &server_filter,
            lookup_uri.as_ref(),
        ) {
            return Err(McpError::invalid_params(
                format!("Resource '{}' is not directly exposed for this client", canonical_uri),
                None,
            ));
        }
    }

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

    let connection_selection = client.connection_selection(server_filter.clone());

    match crate::core::capability::facade::read_upstream_resource(
        &server.connection_pool,
        &resource_mapping,
        &lookup_uri,
        Some(&server_filter),
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
