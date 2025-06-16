//! Resource mapping module
//!
//! Contains functions for building resource mappings from upstream servers

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
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
/// Only enabled resources (based on configuration suits) are included in the mapping.
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
    let mut resource_mapping: HashMap<String, ResourceMapping> = HashMap::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Iterate through all servers and instances
    for (server_name, instances) in &pool.connections {
        for (instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Skip instances that don't support resources
            if !conn.supports_resources() {
                tracing::debug!(
                    "Server '{}' (instance: {}) does not support resources, skipping",
                    server_name,
                    instance_id
                );
                continue;
            }

            tracing::debug!(
                "Collecting resources from instance {} (server: {})",
                instance_id,
                server_name
            );

            // Collect all resources from this instance with pagination
            match collect_all_resources_from_instance(conn, instance_id, server_name).await {
                Ok(resources) => {
                    for resource_mapping_item in resources {
                        let uri = &resource_mapping_item.upstream_resource_uri;

                        // Filter disabled resources if database is available
                        if let Some(db) = database {
                            match super::status::is_resource_enabled(&db.pool, server_name, uri)
                                .await
                            {
                                Ok(enabled) => {
                                    if !enabled {
                                        tracing::debug!(
                                            "Filtering out disabled resource '{}' from server '{}'",
                                            uri,
                                            server_name
                                        );
                                        continue;
                                    }
                                }
                                Err(e) => {
                                    // Log the error but include the resource by default
                                    tracing::warn!(
                                        "Error checking if resource '{}' from server '{}' is enabled: {}. Including by default.",
                                        uri,
                                        server_name,
                                        e
                                    );
                                }
                            }
                        }

                        // Check for URI conflicts and log warnings
                        if let Some(existing) = resource_mapping.get(uri) {
                            tracing::warn!(
                                "Resource URI conflict detected: '{}' is provided by both '{}' (instance: {}) and '{}' (instance: {}). Using the first one.",
                                uri,
                                existing.server_name,
                                existing.instance_id,
                                server_name,
                                instance_id
                            );
                            continue; // Keep the first one, skip the conflicting one
                        }

                        resource_mapping.insert(uri.clone(), resource_mapping_item);
                    }
                }
                Err(e) => {
                    // Check if this is a "method not found" error (server doesn't support resources)
                    let error_msg = format!("{}", e);
                    if error_msg.contains("Method not found") || error_msg.contains("not supported")
                    {
                        tracing::debug!(
                            "Server '{}' (instance: {}) does not support resources: {}",
                            server_name,
                            instance_id,
                            e
                        );
                    } else {
                        tracing::warn!(
                            "Failed to collect resources from instance {} (server: {}): {}",
                            instance_id,
                            server_name,
                            e
                        );
                    }
                    // Continue with other instances even if one fails
                }
            }
        }
    }

    tracing::debug!(
        "Built resource mapping with {} resources",
        resource_mapping.len()
    );
    resource_mapping
}

/// Build resource template mapping from all available instances
///
/// This function collects resource templates from all connected upstream servers.
pub async fn build_resource_template_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<ResourceTemplateMapping> {
    let mut resource_template_mapping = Vec::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Iterate through all servers and instances
    for (server_name, instances) in &pool.connections {
        for (instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Skip instances that don't support resources
            if !conn.supports_resources() {
                tracing::debug!(
                    "Server '{}' (instance: {}) does not support resources, skipping resource templates",
                    server_name,
                    instance_id
                );
                continue;
            }

            tracing::debug!(
                "Collecting resource templates from instance {} (server: {})",
                instance_id,
                server_name
            );

            // Collect all resource templates from this instance with pagination
            match collect_all_resource_templates_from_instance(conn, instance_id, server_name).await
            {
                Ok(templates) => {
                    resource_template_mapping.extend(templates);
                }
                Err(e) => {
                    // Check if this is a "method not found" error (server doesn't support resource templates)
                    let error_msg = format!("{}", e);
                    if error_msg.contains("Method not found") || error_msg.contains("not supported")
                    {
                        tracing::debug!(
                            "Server '{}' (instance: {}) does not support resource templates: {}",
                            server_name,
                            instance_id,
                            e
                        );
                    } else {
                        tracing::warn!("{instance_id} (server: {server_name}): {e}");
                    }
                    // Continue with other instances even if one fails
                }
            }
        }
    }

    tracing::debug!(
        "Built resource template mapping with {} templates",
        resource_template_mapping.len()
    );
    resource_template_mapping
}

/// Collect all resources from a single instance with pagination
async fn collect_all_resources_from_instance(
    conn: &crate::core::connection::UpstreamConnection,
    instance_id: &str,
    server_name: &str,
) -> Result<Vec<ResourceMapping>> {
    let mut all_resources = Vec::new();

    // Check if the connection has a service
    let service = match &conn.service {
        Some(service) => service,
        None => {
            return Err(anyhow::anyhow!(
                "No service available for instance {} (server: {})",
                instance_id,
                server_name
            ));
        }
    };

    let mut cursor = None;

    loop {
        let result = service
            .list_resources(Some(PaginatedRequestParam { cursor }))
            .await
            .context("Failed to list resources from upstream server")?;

        for resource in result.resources {
            let resource_mapping = ResourceMapping {
                server_name: server_name.to_string(),
                instance_id: instance_id.to_string(),
                upstream_resource_uri: resource.uri.clone(),
                resource,
            };
            all_resources.push(resource_mapping);
        }

        cursor = result.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    Ok(all_resources)
}

/// Collect all resource templates from a single instance with pagination
async fn collect_all_resource_templates_from_instance(
    conn: &crate::core::connection::UpstreamConnection,
    instance_id: &str,
    server_name: &str,
) -> Result<Vec<ResourceTemplateMapping>> {
    let mut all_templates = Vec::new();

    // Check if the connection has a service
    let service = match &conn.service {
        Some(service) => service,
        None => {
            return Err(anyhow::anyhow!(
                "No service available for instance {} (server: {})",
                instance_id,
                server_name
            ));
        }
    };

    let mut cursor = None;

    loop {
        let result = service
            .list_resource_templates(Some(PaginatedRequestParam { cursor }))
            .await
            .context("Failed to list resource templates from upstream server")?;

        for template in result.resource_templates {
            let template_mapping = ResourceTemplateMapping {
                server_name: server_name.to_string(),
                instance_id: instance_id.to_string(),
                resource_template: template,
            };
            all_templates.push(template_mapping);
        }

        cursor = result.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    Ok(all_templates)
}

/// Get all resources from all connected upstream servers
///
/// This function collects all resources from all connected upstream servers
/// and returns them as a vector of Resource objects.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `Vec<rmcp::model::Resource>` - A vector of all available resources
pub async fn get_all_resources(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<rmcp::model::Resource> {
    let mut all_resources = Vec::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Iterate through all servers and instances
    for (server_name, instances) in &pool.connections {
        for (instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Skip instances that don't support resources
            if !conn.supports_resources() {
                continue;
            }

            // Collect all resources from this instance
            match collect_all_resources_from_instance(conn, instance_id, server_name).await {
                Ok(resource_mappings) => {
                    // Convert ResourceMapping to Resource
                    for mapping in resource_mappings {
                        all_resources.push(mapping.resource);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to collect resources from instance {} (server: {}): {}",
                        instance_id,
                        server_name,
                        e
                    );
                }
            }
        }
    }

    tracing::debug!("Collected {} total resources", all_resources.len());
    all_resources
}

/// Get all resource templates from all connected upstream servers
///
/// This function collects all resource templates from all connected upstream servers
/// and returns them as a vector of ResourceTemplate objects.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `Vec<rmcp::model::ResourceTemplate>` - A vector of all available resource templates
pub async fn get_all_resource_templates(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<rmcp::model::ResourceTemplate> {
    let mut all_resource_templates = Vec::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Iterate through all servers and instances
    for (server_name, instances) in &pool.connections {
        for (instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Skip instances that don't support resources
            if !conn.supports_resources() {
                continue;
            }

            // Collect all resource templates from this instance
            match collect_all_resource_templates_from_instance(conn, instance_id, server_name).await
            {
                Ok(template_mappings) => {
                    // Convert ResourceTemplateMapping to ResourceTemplate
                    for mapping in template_mappings {
                        all_resource_templates.push(mapping.resource_template);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to collect resource templates from instance {} (server: {}): {}",
                        instance_id,
                        server_name,
                        e
                    );
                }
            }
        }
    }

    tracing::debug!(
        "Collected {} total resource templates",
        all_resource_templates.len()
    );
    all_resource_templates
}
