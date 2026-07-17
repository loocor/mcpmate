use super::*;
use crate::core::capability::naming::NamingKind;
use crate::core::capability::resource_registry::{
    ResolvedResourceRoute, resolve_resource_route, rewrite_read_resource_result,
};
use futures::StreamExt;
use rmcp::ErrorData as McpError;
use rmcp::model::{
    ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResult,
};
use rmcp::service::RequestContext;
use std::collections::HashSet;

#[derive(Debug)]
pub(super) struct ResolvedExternalResourceTarget {
    pub(super) server_id: String,
    pub(super) route: ResolvedResourceRoute,
}

impl ResolvedExternalResourceTarget {
    pub(super) fn upstream_uri(&self) -> &str {
        &self.route.upstream_uri
    }

    pub(super) fn canonical_uri(&self) -> &str {
        &self.route.external_uri
    }
}

pub(super) async fn resolve_external_resource_target(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    external_uri: &str,
) -> anyhow::Result<ResolvedExternalResourceTarget> {
    let route = resolve_resource_route(pool, external_uri).await?;
    let server_id = route.server_id.clone();
    Ok(ResolvedExternalResourceTarget { server_id, route })
}

pub(super) async fn resolve_authorized_external_resource_target(
    server: &ProxyServer,
    client: &crate::core::proxy::server::common::ClientContext,
    external_uri: &str,
) -> Result<ResolvedExternalResourceTarget, McpError> {
    let Some(db) = &server.database else {
        return Err(McpError::internal_error(
            "Resource routing requires database-backed registry metadata".to_string(),
            None,
        ));
    };
    let target = resolve_external_resource_target(&db.pool, external_uri)
        .await
        .map_err(|error| McpError::invalid_params(format!("Invalid external resource URI: {error}"), None))?;
    let canonical_uri = target.canonical_uri().to_string();

    if matches!(client.config_mode.as_deref(), Some("unify")) {
        let eligible_server_ids =
            crate::core::proxy::server::load_unify_direct_exposure_eligible_server_ids(db).await?;
        if !crate::core::proxy::server::unify_directly_exposed_resource_route_allowed(
            client.unify_workspace.as_ref(),
            &eligible_server_ids,
            &target.server_id,
            &target.route,
        ) {
            return Err(McpError::invalid_params(
                format!("Resource '{canonical_uri}' is not directly exposed for this client"),
                None,
            ));
        }
    }

    let visibility = crate::core::profile::visibility::ProfileVisibilityService::new(
        server.database.clone(),
        server.profile_service.clone(),
    );
    let snapshot = visibility
        .resolve_snapshot_for_client(client)
        .await
        .map_err(|error| McpError::internal_error(error.to_string(), None))?;
    if let Err(error) = visibility
        .assert_resource_allowed_with_snapshot(&snapshot, &canonical_uri)
        .await
    {
        tracing::warn!(
            resource = %canonical_uri,
            client_id = %client.client_id,
            profile_id = ?client.profile_id,
            error = %error,
            "External resource access denied by visibility policy"
        );
        return Err(McpError::invalid_params(
            format!("Resource '{canonical_uri}' is not available for this client"),
            None,
        ));
    }

    Ok(target)
}

fn map_resource_read_error(error: anyhow::Error) -> McpError {
    for source in error.chain() {
        if let Some(rmcp::service::ServiceError::McpError(upstream)) =
            source.downcast_ref::<rmcp::service::ServiceError>()
            && upstream.code == rmcp::model::ErrorCode::RESOURCE_NOT_FOUND
        {
            return upstream.clone();
        }
    }
    McpError::internal_error(error.to_string(), None)
}

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
                        resource_template.uri_template.as_ref(),
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
    tracing::debug!("Reading resource: {}", request.uri);

    let target = resolve_authorized_external_resource_target(server, &client, &request.uri).await?;
    let server_filter = target.server_id.clone();
    let lookup_uri = target.upstream_uri().to_string();

    let connection_selection = client.connection_selection(server_filter.clone());

    match crate::core::capability::facade::read_routed_resource(
        &server.connection_pool,
        &server_filter,
        &lookup_uri,
        connection_selection.as_ref(),
    )
    .await
    {
        Ok(mut result) => {
            let db = server.database.as_ref().ok_or_else(|| {
                McpError::internal_error(
                    "Resource response projection requires registry metadata".to_string(),
                    None,
                )
            })?;
            rewrite_read_resource_result(&db.pool, &target.route, &mut result)
                .await
                .map_err(|error| McpError::internal_error(error.to_string(), None))?;
            Ok(result)
        }
        Err(e) => {
            tracing::error!("Failed to read resource '{}': {}", request.uri, e);
            Err(map_resource_read_error(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn route_pool() -> sqlx::Pool<sqlx::Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect route database");
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .expect("initialize server tables");
        crate::config::profile::init::initialize_profile_tables(&pool)
            .await
            .expect("initialize profile tables");
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'everything', 'stdio')")
            .execute(&pool)
            .await
            .expect("insert server");
        pool
    }

    #[tokio::test]
    async fn canonical_static_resource_resolves_from_registry() {
        let pool = route_pool().await;
        let canonical = crate::config::server::capabilities::upsert_shadow_resource(
            &pool,
            "server-a",
            "everything",
            "demo://resource/static/document/architecture.md",
            None,
            None,
            None,
        )
        .await
        .expect("insert listed resource");

        let target = resolve_external_resource_target(&pool, &canonical)
            .await
            .expect("resolve resource target");

        assert_eq!(target.server_id, "server-a");
        assert_eq!(target.upstream_uri(), "demo://resource/static/document/architecture.md");
        assert_eq!(target.canonical_uri(), canonical);
    }

    #[tokio::test]
    async fn template_derived_resource_resolves_from_registry_without_static_row() {
        let pool = route_pool().await;
        let template = crate::config::server::capabilities::upsert_shadow_resource_template(
            &pool,
            "server-a",
            "everything",
            "demo://resource/dynamic/text/{resourceId}",
            Some("Dynamic Text Resource"),
            None,
        )
        .await
        .expect("insert template route");
        let canonical = template.replace("{resourceId}", "42");

        let target = resolve_external_resource_target(&pool, &canonical)
            .await
            .expect("resolve template target");

        assert_eq!(target.server_id, "server-a");
        assert_eq!(target.upstream_uri(), "demo://resource/dynamic/text/42");
    }

    #[tokio::test]
    async fn raw_or_unknown_external_resource_routes_fail_closed() {
        let pool = route_pool().await;
        assert!(
            resolve_external_resource_target(&pool, "file:///guide.md")
                .await
                .is_err()
        );
        assert!(
            resolve_external_resource_target(&pool, "mcpmate://resources/everything/ZGVtbzovL3Jlc291cmNlL3N0YXRpYw",)
                .await
                .is_err()
        );

        assert!(
            resolve_external_resource_target(&pool, "mcpmate://resources/everything/demo/static/document/missing.md",)
                .await
                .is_err()
        );
    }

    #[test]
    fn resource_not_found_error_code_survives_proxy_mapping() {
        let upstream = rmcp::ErrorData::resource_not_found("missing", None);
        let error = anyhow::Error::new(rmcp::service::ServiceError::McpError(upstream.clone()))
            .context("Failed to read resource from upstream server");

        assert_eq!(map_resource_read_error(error), upstream);
    }
}
