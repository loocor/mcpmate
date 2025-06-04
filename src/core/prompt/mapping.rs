// Prompt mapping module
// Contains functions for building and managing prompt mappings

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use rmcp::model::PaginatedRequestParam;
use tokio::sync::Mutex;
use tracing;

use super::types::{PromptMapping, PromptTemplateMapping};
use crate::core::http::pool::UpstreamConnectionPool;

/// Build a mapping of prompt names to server/instance information
///
/// This function builds a mapping of prompt names to the server and instance
/// that provides them. It is used to route prompt requests to the appropriate upstream server.
/// Only enabled prompts (based on configuration suits) are included in the mapping.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
/// * `database` - Optional database connection for filtering enabled prompts
///
/// # Returns
/// * `HashMap<String, PromptMapping>` - A mapping of prompt names to server/instance information
pub async fn build_prompt_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    database: Option<&Arc<crate::config::database::Database>>,
) -> HashMap<String, PromptMapping> {
    let mut prompt_mapping: HashMap<String, PromptMapping> = HashMap::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Iterate through all servers and instances
    for (server_name, instances) in &pool.connections {
        for (instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Skip instances that don't support prompts
            if !conn.supports_prompts() {
                tracing::debug!(
                    "Server '{}' (instance: {}) does not support prompts, skipping",
                    server_name,
                    instance_id
                );
                continue;
            }

            tracing::debug!(
                "Collecting prompts from instance {} (server: {})",
                instance_id,
                server_name
            );

            // Collect all prompts from this instance with pagination
            match collect_all_prompts_from_instance(conn, instance_id, server_name).await {
                Ok(prompts) => {
                    for prompt_mapping_item in prompts {
                        let name = &prompt_mapping_item.upstream_prompt_name;

                        // Filter disabled prompts if database is available
                        if let Some(db) = database {
                            match super::status::is_prompt_enabled(&db.pool, server_name, name)
                                .await
                            {
                                Ok(enabled) => {
                                    if !enabled {
                                        tracing::debug!(
                                            "Filtering out disabled prompt '{}' from server '{}'",
                                            name,
                                            server_name
                                        );
                                        continue;
                                    }
                                }
                                Err(e) => {
                                    // Log the error but include the prompt by default
                                    tracing::warn!(
                                        "Error checking if prompt '{}' from server '{}' is enabled: {}. Including by default.",
                                        name,
                                        server_name,
                                        e
                                    );
                                }
                            }
                        }

                        // Check for name conflicts and log warnings
                        if let Some(existing) = prompt_mapping.get(name) {
                            tracing::warn!(
                                "Prompt name conflict detected: '{}' is provided by both '{}' (instance: {}) and '{}' (instance: {}). Using the first one.",
                                name,
                                existing.server_name,
                                existing.instance_id,
                                server_name,
                                instance_id
                            );
                            continue; // Keep the first one, skip the conflicting one
                        }

                        prompt_mapping.insert(name.clone(), prompt_mapping_item);
                    }
                }
                Err(e) => {
                    // Check if this is a "method not found" error (server doesn't support prompts)
                    let error_msg = format!("{}", e);
                    if error_msg.contains("Method not found") || error_msg.contains("not supported")
                    {
                        tracing::debug!(
                            "Server '{}' (instance: {}) does not support prompts: {}",
                            server_name,
                            instance_id,
                            e
                        );
                    } else {
                        tracing::warn!(
                            "Failed to collect prompts from instance {} (server: {}): {}",
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

    tracing::debug!("Built prompt mapping with {} prompts", prompt_mapping.len());
    prompt_mapping
}

/// Build prompt template mapping from all available instances
///
/// This function collects prompt templates from all connected upstream servers.
pub async fn build_prompt_template_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<PromptTemplateMapping> {
    let mut prompt_template_mapping = Vec::new();

    // Lock the connection pool to access it
    let pool = connection_pool.lock().await;

    // Iterate through all servers and instances
    for (server_name, instances) in &pool.connections {
        for (instance_id, conn) in instances {
            // Skip instances that are not connected
            if !conn.is_connected() {
                continue;
            }

            // Skip instances that don't support prompts
            if !conn.supports_prompts() {
                tracing::debug!(
                    "Server '{}' (instance: {}) does not support prompts, skipping prompt templates",
                    server_name,
                    instance_id
                );
                continue;
            }

            tracing::debug!(
                "Collecting prompt templates from instance {} (server: {})",
                instance_id,
                server_name
            );

            // Collect all prompt templates from this instance with pagination
            match collect_all_prompt_templates_from_instance(conn, instance_id, server_name).await {
                Ok(templates) => {
                    prompt_template_mapping.extend(templates);
                }
                Err(e) => {
                    // Check if this is a "method not found" error (server doesn't support prompt templates)
                    let error_msg = format!("{}", e);
                    if error_msg.contains("Method not found") || error_msg.contains("not supported")
                    {
                        tracing::debug!(
                            "Server '{}' (instance: {}) does not support prompt templates: {}",
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
        "Built prompt template mapping with {} templates",
        prompt_template_mapping.len()
    );
    prompt_template_mapping
}

/// Collect all prompts from a single instance with pagination
async fn collect_all_prompts_from_instance(
    conn: &crate::core::connection::UpstreamConnection,
    instance_id: &str,
    server_name: &str,
) -> Result<Vec<PromptMapping>> {
    let mut all_prompts = Vec::new();

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
            .list_prompts(Some(PaginatedRequestParam { cursor }))
            .await
            .context("Failed to list prompts from upstream server")?;

        for prompt in result.prompts {
            let prompt_mapping = PromptMapping {
                server_name: server_name.to_string(),
                instance_id: instance_id.to_string(),
                upstream_prompt_name: prompt.name.clone(),
                prompt,
            };
            all_prompts.push(prompt_mapping);
        }

        cursor = result.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    Ok(all_prompts)
}

/// Collect all prompt templates from a single instance with pagination
async fn collect_all_prompt_templates_from_instance(
    conn: &crate::core::connection::UpstreamConnection,
    instance_id: &str,
    server_name: &str,
) -> Result<Vec<PromptTemplateMapping>> {
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
            .list_prompts(Some(PaginatedRequestParam { cursor }))
            .await
            .context("Failed to list prompts from upstream server")?;

        for prompt in result.prompts {
            let template_mapping = PromptTemplateMapping {
                server_name: server_name.to_string(),
                instance_id: instance_id.to_string(),
                prompt_template: prompt,
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
