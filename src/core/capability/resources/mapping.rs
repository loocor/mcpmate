//! Resource mapping module
//!
//! Contains functions for building resource mappings from upstream servers

use std::{collections::HashMap, sync::Arc};

use crate::core::capability::internal::{
    collect_capability_from_instance_peer, concurrency_limit, is_method_not_supported,
};
use futures::{StreamExt, future::BoxFuture};
use rmcp::model::PaginatedRequestParam;
use tokio::sync::Mutex;
use tracing;

use super::types::{ResourceMapping, ResourceTemplateMapping};
use crate::core::pool::UpstreamConnectionPool;

/// Build a mapping of resource URIs to server/instance information
///
/// This function builds a mapping of resource URIs to the server and instance
/// that provides them. It is used to route resource read requests to the appropriate upstream server.
/// Unlike tools, resources use URI as unique identifier, so conflicts are handled by logging warnings.
/// Only enabled resources (based on profile) are included in the mapping.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
/// * `database` - Optional database connection for filtering enabled resources
///
/// # Returns
/// * `HashMap<String, ResourceMapping>` - A mapping of resource URIs to server/instance information
pub async fn build_resource_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    database: Option<&Arc<crate::config::database::Database>>,
) -> HashMap<String, ResourceMapping> {
    build_resource_mapping_filtered(connection_pool, database, None).await
}

pub async fn build_resource_mapping_filtered(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    database: Option<&Arc<crate::config::database::Database>>,
    enabled_server_ids: Option<&std::collections::HashSet<String>>,
) -> HashMap<String, ResourceMapping> {
    // Step 1: Take light snapshot (only lock acquisition point)
    let snapshot = {
        let pool = connection_pool.lock().await;
        pool.get_snapshot()
    };

    // Step 2: Build tasks for parallel execution (no locks held)
    let mut tasks = Vec::new();

    for (server_id, instances) in snapshot {
        // Pre-filter by enabled server IDs if provided
        if let Some(enabled_ids) = enabled_server_ids {
            if !enabled_ids.contains(&server_id) {
                tracing::debug!("Skipping disabled server: {}", server_id);
                continue;
            }
        }

        for (instance_id, status, supports_resources, _supports_prompts, peer_opt) in instances {
            // Pre-filter: skip non-ready instances or those without resource support
            if !matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) || !supports_resources {
                continue;
            }
            let Some(peer) = peer_opt else {
                continue;
            };

            // Create async task for this server/instance combination
            let server_id_clone = server_id.clone();
            let instance_id_clone = instance_id.clone();
            let database_clone = database.cloned();

            tasks.push(async move {
                let fetch_page = move |p: rmcp::service::Peer<rmcp::service::RoleClient>,
                                       cursor: Option<String>|
                      -> BoxFuture<
                    'static,
                    anyhow::Result<(Vec<rmcp::model::Resource>, Option<String>)>,
                > {
                    Box::pin(async move {
                        let result = p.list_resources(Some(PaginatedRequestParam { cursor })).await?;
                        Ok((result.resources, result.next_cursor))
                    })
                };

                let server_name = match crate::core::capability::global_server_mapping_manager()
                    .get_name_by_id(&server_id_clone)
                    .await
                {
                    Some(name) => name,
                    None => server_id_clone.clone(),
                };

                let mappings = collect_capability_from_instance_peer(
                    peer.clone(),
                    std::time::Duration::from_secs(10),
                    fetch_page,
                    |resource, srv_name, srv_id, inst_id| ResourceMapping {
                        server_name: srv_name.to_string(),
                        server_id: Some(srv_id.to_string()),
                        instance_id: inst_id.to_string(),
                        upstream_resource_uri: resource.uri.clone(),
                        resource,
                    },
                    &server_id_clone,
                    &server_name,
                    &instance_id_clone,
                    is_method_not_supported,
                )
                .await;

                // Optional filtering based on database (keep behavior for now; will move to proxy later)
                if let Some(db) = database_clone {
                    let mut filtered = Vec::new();
                    for item in mappings {
                        if let Ok(enabled) =
                            super::status::is_resource_enabled(&db.pool, &item.server_name, &item.upstream_resource_uri)
                                .await
                        {
                            if enabled {
                                filtered.push(item);
                            }
                        } else {
                            // On error, include by default
                            filtered.push(item);
                        }
                    }
                    filtered
                } else {
                    mappings
                }
            });
        }
    }

    // Step 3: Execute all tasks concurrently with bounded concurrency
    let results: Vec<Vec<ResourceMapping>> = futures::stream::iter(tasks)
        .buffer_unordered(concurrency_limit())
        .collect()
        .await;

    // Step 4: Merge results and handle conflicts
    let mut resource_mapping: HashMap<String, ResourceMapping> = HashMap::new();
    for resources in results {
        for resource in resources {
            let uri = &resource.upstream_resource_uri;

            // Check for URI conflicts and log warnings
            if let Some(existing) = resource_mapping.get(uri) {
                tracing::warn!(
                    "Resource URI conflict detected: '{}' is provided by both '{}' (instance: {}) and '{}' (instance: {}). Using the first one.",
                    uri,
                    existing.server_name,
                    existing.instance_id,
                    resource.server_name,
                    resource.instance_id
                );
                continue; // Keep the first one, skip the conflicting one
            }

            resource_mapping.insert(uri.clone(), resource);
        }
    }

    tracing::debug!("Built resource mapping with {} resources", resource_mapping.len());
    resource_mapping
}

/// Build resource template mapping from all available instances
///
/// This function collects resource templates from all connected upstream servers.
pub async fn build_resource_template_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<ResourceTemplateMapping> {
    // Take a light snapshot to avoid holding the pool lock during network I/O
    let snapshot = {
        let pool = connection_pool.lock().await;
        pool.get_snapshot()
    };

    // Build tasks without holding the lock
    let mut tasks = Vec::new();
    for (server_id, instances) in snapshot {
        for (instance_id, status, supports_resources, _supports_prompts, peer_opt) in instances {
            if !matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) || !supports_resources {
                continue;
            }
            let Some(peer) = peer_opt else {
                continue;
            };

            let server_id_cloned = server_id.clone();
            tasks.push(async move {
                let fetch_page = move |p: rmcp::service::Peer<rmcp::service::RoleClient>,
                                       cursor: Option<String>|
                      -> BoxFuture<
                    'static,
                    anyhow::Result<(Vec<rmcp::model::ResourceTemplate>, Option<String>)>,
                > {
                    Box::pin(async move {
                        let result = p
                            .list_resource_templates(Some(PaginatedRequestParam { cursor }))
                            .await?;
                        Ok((result.resource_templates, result.next_cursor))
                    })
                };

                let server_name = match crate::core::capability::global_server_mapping_manager()
                    .get_name_by_id(&server_id_cloned)
                    .await
                {
                    Some(name) => name,
                    None => server_id_cloned.clone(),
                };

                collect_capability_from_instance_peer(
                    peer,
                    std::time::Duration::from_secs(10),
                    fetch_page,
                    |template, srv_name, srv_id, inst_id| ResourceTemplateMapping {
                        server_name: srv_name.to_string(),
                        server_id: Some(srv_id.to_string()),
                        instance_id: inst_id.to_string(),
                        resource_template: template,
                    },
                    &server_id_cloned,
                    &server_name,
                    &instance_id,
                    is_method_not_supported,
                )
                .await
            });
        }
    }

    let mut resource_template_mapping = Vec::new();
    for templates in futures::stream::iter(tasks)
        .buffer_unordered(concurrency_limit())
        .collect::<Vec<_>>()
        .await
    {
        resource_template_mapping.extend(templates);
    }

    tracing::debug!(
        "Built resource template mapping with {} templates",
        resource_template_mapping.len()
    );
    resource_template_mapping
}
