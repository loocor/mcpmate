//! Prompt mapping module
//!
//! Contains functions for building prompt mappings from upstream servers

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing;

use super::types::{PromptMapping, PromptTemplateMapping};
use crate::core::pool::UpstreamConnectionPool;

/// Build a mapping of prompt names to server/instance information
pub async fn build_prompt_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> HashMap<String, PromptMapping> {
    let mut prompt_mapping = HashMap::new();

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

            // Get prompts from this instance
            if let Some(service) = &conn.service {
                tracing::debug!(
                    "Fetching prompts from instance {} (server: {})",
                    instance_id,
                    server_name
                );

                match service.list_prompts(None).await {
                    Ok(result) => {
                        for prompt in result.prompts {
                            // Check for name conflicts
                            if prompt_mapping.contains_key(&prompt.name.to_string()) {
                                tracing::warn!(
                                    "Prompt '{}' is provided by multiple servers, using the first one",
                                    prompt.name
                                );
                                continue;
                            }

                            // Add the prompt to the mapping
                            prompt_mapping.insert(
                                prompt.name.to_string(),
                                PromptMapping {
                                    server_name: server_name.clone(),
                                    instance_id: instance_id.clone(),
                                    prompt: prompt.clone(),
                                    upstream_prompt_name: prompt.name.to_string(),
                                },
                            );

                            tracing::debug!(
                                "Added prompt '{}' from server '{}'",
                                prompt.name,
                                server_name
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to list prompts from server '{}' (instance: {}): {}",
                            server_name,
                            instance_id,
                            e
                        );
                    }
                }
            } else {
                tracing::debug!(
                    "No service available for instance {} (server: {})",
                    instance_id,
                    server_name
                );
            }
        }
    }

    tracing::debug!("Built prompt mapping with {} prompts", prompt_mapping.len());
    prompt_mapping
}

/// Build prompt template mapping from all available instances
pub async fn build_prompt_template_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<PromptTemplateMapping> {
    let prompt_template_mapping = Vec::new();

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
                continue;
            }

            // Add all prompt templates from this instance
            // Note: Prompt templates are fetched dynamically in core architecture.
            // For now, we'll create an empty template mapping since templates are handled on-demand.
            tracing::debug!(
                "Prompt template mapping for instance {} (server: {}) - using on-demand fetching",
                instance_id,
                server_name
            );
        }
    }

    tracing::debug!(
        "Built prompt template mapping with {} templates",
        prompt_template_mapping.len()
    );
    prompt_template_mapping
}

/// Get all prompts from all connected upstream servers
///
/// This function collects all prompts from all connected upstream servers
/// and returns them as a vector of Prompt objects.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
///
/// # Returns
/// * `Vec<rmcp::model::Prompt>` - A vector of all available prompts
pub async fn get_all_prompts(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<rmcp::model::Prompt> {
    // Build the prompt mapping first
    let prompt_mapping = build_prompt_mapping(connection_pool).await;

    // Extract all prompts from the mapping
    let all_prompts: Vec<rmcp::model::Prompt> = prompt_mapping
        .into_values()
        .map(|mapping| mapping.prompt)
        .collect();

    tracing::debug!("Collected {} total prompts", all_prompts.len());
    all_prompts
}
