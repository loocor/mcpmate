//! Resource capability helpers
//!
//! Provides mapping builders, status checks, and read helpers for upstream resources.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{Context, Result};
use futures::{StreamExt, future::BoxFuture};
use rmcp::model::{PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResult, Resource, ResourceTemplate};
use sqlx::{Pool, Sqlite};
use tokio::sync::Mutex;
use tracing;

use crate::core::capability::facade::{
    collect_capability_from_instance_peer, concurrency_limit, is_method_not_supported,
};
use crate::core::pool::{UpstreamConnection, UpstreamConnectionPool};

#[derive(Debug, Clone)]
pub struct ResourceMapping {
    pub server_name: String,
    pub server_id: Option<String>,
    pub instance_id: String,
    pub resource: Resource,
    pub upstream_resource_uri: String,
}

#[derive(Debug, Clone)]
pub struct ResourceTemplateMapping {
    pub server_name: String,
    pub server_id: Option<String>,
    pub instance_id: String,
    pub resource_template: ResourceTemplate,
}

pub async fn is_resource_enabled(
    pool: &Pool<Sqlite>,
    server_name: &str,
    resource_uri: &str,
) -> Result<bool> {
    tracing::debug!(
        "Checking if resource '{}' from server '{}' is enabled",
        resource_uri,
        server_name
    );

    let active_profile = crate::config::profile::get_active_profile(pool)
        .await
        .context("Failed to get active profile")?;

    if active_profile.is_empty() {
        tracing::debug!("No active profile found, resource is disabled");
        return Ok(false);
    }

    let server = crate::config::server::get_server(pool, server_name)
        .await
        .context(format!("Failed to get server '{server_name}'"))?;

    let server_id = match server {
        Some(server) => match server.id {
            Some(id) => id,
            None => {
                tracing::warn!("Server '{}' has no ID, resource is disabled", server_name);
                return Ok(false);
            }
        },
        None => {
            tracing::debug!("Server '{}' not found, resource is disabled", server_name);
            return Ok(false);
        }
    };

    for profile in active_profile {
        if let Some(profile_id) = &profile.id {
            let enabled_resources = crate::config::profile::get_enabled_resources_for_profile(pool, profile_id)
                .await
                .context(format!("Failed to get enabled resources for profile '{profile_id}'"))?;

            for resource in enabled_resources {
                if resource.server_id == server_id && resource.resource_uri == resource_uri {
                    tracing::debug!(
                        "Resource '{}' from server '{}' is enabled in profile '{}'",
                        resource_uri,
                        server_name,
                        profile.name
                    );
                    return Ok(true);
                }
            }

            // Fallback: if any enabled resource template matches this URI, treat as enabled
            if crate::config::profile::resource_template::resource_matches_enabled_templates(
                pool,
                profile_id,
                &server_id,
                resource_uri,
            )
            .await
            .unwrap_or(false)
            {
                tracing::debug!(
                    "Resource '{}' from server '{}' allowed by enabled template in profile '{}'",
                    resource_uri,
                    server_name,
                    profile.name
                );
                return Ok(true);
            }
        }
    }

    tracing::debug!(
        "Resource '{}' from server '{}' is not enabled in any active profile",
        resource_uri,
        server_name
    );
    Ok(false)
}

pub async fn get_resource_status(
    pool: &Pool<Sqlite>,
    server_name: &str,
    resource_uri: &str,
) -> Result<(String, bool)> {
    tracing::debug!(
        "Getting resource status for '{}' from server '{}'",
        resource_uri,
        server_name
    );

    let enabled = is_resource_enabled(pool, server_name, resource_uri).await?;
    let active_profile = crate::config::profile::get_active_profile(pool)
        .await
        .context("Failed to get active profile")?;

    let server = crate::config::server::get_server(pool, server_name)
        .await
        .context(format!("Failed to get server '{server_name}'"))?;

    let server_id = match server {
        Some(server) => match server.id {
            Some(id) => id,
            None => {
                return Err(anyhow::anyhow!("Server '{}' has no ID", server_name));
            }
        },
        None => {
            return Err(anyhow::anyhow!("Server '{}' not found", server_name));
        }
    };

    for profile in active_profile {
        if let Some(profile_id) = &profile.id {
            let resources = crate::config::profile::get_resources_for_profile(pool, profile_id)
                .await
                .context(format!("Failed to get resources for profile '{profile_id}'"))?;

            for resource in resources {
                if resource.server_id == server_id && resource.resource_uri == resource_uri {
                    if let Some(resource_id) = resource.id {
                        return Ok((resource_id, enabled));
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "Resource '{}' from server '{}' not found in any active profile",
        resource_uri,
        server_name
    ))
}

pub async fn build_resource_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    database: Option<&Arc<crate::config::database::Database>>,
) -> HashMap<String, ResourceMapping> {
    build_resource_mapping_filtered(connection_pool, database, None).await
}

pub async fn build_resource_mapping_filtered(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    database: Option<&Arc<crate::config::database::Database>>,
    enabled_server_ids: Option<&HashSet<String>>,
) -> HashMap<String, ResourceMapping> {
    let snapshot = {
        let pool = connection_pool.lock().await;
        pool.get_snapshot()
    };

    let mut tasks = Vec::new();

    for (server_id, instances) in snapshot {
        if let Some(enabled_ids) = enabled_server_ids {
            if !enabled_ids.contains(&server_id) {
                tracing::debug!("Skipping disabled server: {}", server_id);
                continue;
            }
        }

        for (instance_id, status, supports_resources, _supports_prompts, peer_opt) in instances {
            if !matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) || !supports_resources {
                continue;
            }
            let Some(peer) = peer_opt else { continue };

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
                        let result = p
                            .list_resources(Some(PaginatedRequestParams::default().with_cursor(cursor)))
                            .await?;
                        Ok((result.resources, result.next_cursor))
                    })
                };

                let server_name = match crate::core::capability::resolver::to_name(&server_id_clone).await {
                    Ok(Some(name)) => name,
                    _ => server_id_clone.clone(),
                };

                let outcome = collect_capability_from_instance_peer(
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
                let mappings = outcome.items;

                if let Some(db) = database_clone {
                    let mut filtered = Vec::new();
                    for item in mappings {
                        if let Ok(enabled) =
                            is_resource_enabled(&db.pool, &item.server_name, &item.upstream_resource_uri).await
                        {
                            if enabled {
                                filtered.push(item);
                            }
                        } else {
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

    let results: Vec<Vec<ResourceMapping>> = futures::stream::iter(tasks)
        .buffer_unordered(concurrency_limit())
        .collect()
        .await;

    let mut resource_mapping: HashMap<String, ResourceMapping> = HashMap::new();
    for resources in results {
        for resource in resources {
            let uri = &resource.upstream_resource_uri;
            if let Some(existing) = resource_mapping.get(uri) {
                tracing::warn!(
                    "Resource URI conflict detected: '{}' is provided by both '{}' (instance: {}) and '{}' (instance: {}). Using the first one.",
                    uri,
                    existing.server_name,
                    existing.instance_id,
                    resource.server_name,
                    resource.instance_id
                );
                continue;
            }
            resource_mapping.insert(uri.clone(), resource);
        }
    }

    tracing::debug!("Built resource mapping with {} resources", resource_mapping.len());
    resource_mapping
}

pub async fn build_resource_template_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<ResourceTemplateMapping> {
    let snapshot = {
        let pool = connection_pool.lock().await;
        pool.get_snapshot()
    };

    let mut tasks = Vec::new();
    for (server_id, instances) in snapshot {
        for (instance_id, status, supports_resources, _supports_prompts, peer_opt) in instances {
            if !matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) || !supports_resources {
                continue;
            }
            let Some(peer) = peer_opt else { continue };

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
                            .list_resource_templates(Some(PaginatedRequestParams::default().with_cursor(cursor)))
                            .await?;
                        Ok((result.resource_templates, result.next_cursor))
                    })
                };

                let server_name = match crate::core::capability::resolver::to_name(&server_id_cloned).await {
                    Ok(Some(name)) => name,
                    _ => server_id_cloned.clone(),
                };

                let outcome = collect_capability_from_instance_peer(
                    peer.clone(),
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
                .await;

                outcome.items
            });
        }
    }

    let results: Vec<Vec<ResourceTemplateMapping>> = futures::stream::iter(tasks)
        .buffer_unordered(concurrency_limit())
        .collect()
        .await;

    let mut templates = Vec::new();
    for mut mapping in results {
        templates.append(&mut mapping);
    }
    templates
}

pub async fn read_upstream_resource(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    resource_mapping: &HashMap<String, ResourceMapping>,
    uri: &str,
    target_server_id: Option<&str>,
) -> Result<ReadResourceResult> {
    tracing::debug!("Reading resource: {}", uri);

    let mapping_opt = resource_mapping.get(uri);
    if let Some(mapping) = mapping_opt {
        tracing::debug!(
            "Routing resource read request for '{}' to instance {} (server: {})",
            uri,
            mapping.instance_id,
            mapping.server_name
        );
    } else {
        tracing::debug!(
            "Routing resource read request for '{}' with no mapping; will use target server if provided",
            uri
        );
    }

    let (service, upstream_resource_uri) = {
        let mut pool = connection_pool.lock().await;
        let (server_id_owned, upstream_uri_owned, desired_instance_id_opt): (String, String, Option<String>) =
            if let Some(mapping) = mapping_opt {
                (
                    mapping
                        .server_id
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Server ID not available for resource {}", uri))?
                        .to_string(),
                    mapping.upstream_resource_uri.clone(),
                    Some(mapping.instance_id.clone()),
                )
            } else if let Some(server_id) = target_server_id {
                (server_id.to_string(), uri.to_string(), None)
            } else {
                return Err(anyhow::anyhow!("Resource not found: {}", uri));
            };
        let server_id = &server_id_owned;

        let mut target_instance_id: Option<String> = None;
        if let Some(instances) = pool.connections.get(server_id) {
            if let Some(desired) = &desired_instance_id_opt {
                if let Some(conn) = instances.get(desired) {
                    if conn.service.is_some() {
                        target_instance_id = Some(desired.clone());
                    }
                }
            }
        }

        if target_instance_id.is_none() {
            pool.ensure_connected(server_id).await.context(format!(
                "Failed to ensure connection for server '{}' (resource '{}')",
                server_id, uri
            ))?;
            let iid = pool
                .get_default_instance_id(server_id)
                .map_err(|e| anyhow::anyhow!("Failed to get default instance for server '{}': {}", server_id, e))?;
            target_instance_id = Some(iid);
        }

        let iid = target_instance_id.expect("instance id must be set");
        let instances = pool
            .connections
            .get(server_id)
            .ok_or_else(|| anyhow::anyhow!("Server {} not found for resource {}", server_id, uri))?;
        let conn = instances
            .get(&iid)
            .ok_or_else(|| anyhow::anyhow!("Instance {} not found for resource {}", iid, uri))?;
        let service = conn
            .service
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No service available for instance {} (server: {})", iid, server_id))?;

        (service, upstream_uri_owned)
    };

    let result = service
        .read_resource(ReadResourceRequestParams::new(upstream_resource_uri))
        .await
        .context("Failed to read resource from upstream server")?;

    tracing::debug!(
        "Successfully read resource '{}', got {} content items",
        uri,
        result.contents.len()
    );

    for (i, content) in result.contents.iter().enumerate() {
        match content {
            rmcp::model::ResourceContents::TextResourceContents { mime_type, text, .. } => {
                tracing::debug!(
                    "Content {}: text (mime_type: {:?}, length: {} chars)",
                    i,
                    mime_type,
                    text.len()
                );
            }
            rmcp::model::ResourceContents::BlobResourceContents { mime_type, blob, .. } => {
                tracing::debug!(
                    "Content {}: blob (mime_type: {:?}, length: {} bytes base64)",
                    i,
                    mime_type,
                    blob.len()
                );
            }
        }
    }

    Ok(result)
}

/// Read a resource directly from a specific upstream connection (native/direct path)
pub async fn read_upstream_resource_direct(
    connection: &UpstreamConnection,
    uri: &str,
) -> Result<ReadResourceResult> {
    validate_resource_uri(uri)?;
    let service = connection
        .service
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No service available for upstream connection"))?;

    if !connection.is_connected() {
        return Err(anyhow::anyhow!(
            "Connection is not ready (status: {})",
            connection.status
        ));
    }

    let result = service
        .read_resource(ReadResourceRequestParams::new(uri.to_string()))
        .await
        .context("Failed to read resource from upstream server")?;

    Ok(result)
}

pub fn validate_resource_uri(uri: &str) -> Result<()> {
    if uri.is_empty() {
        return Err(anyhow::anyhow!("Resource URI cannot be empty"));
    }
    if !uri.contains("://") {
        return Err(anyhow::anyhow!(
            "Resource URI must contain a scheme (e.g., 'file://', 'memory://'): {}",
            uri
        ));
    }
    if uri.contains('\n') || uri.contains('\r') {
        return Err(anyhow::anyhow!(
            "Resource URI cannot contain newline characters: {}",
            uri
        ));
    }
    Ok(())
}
