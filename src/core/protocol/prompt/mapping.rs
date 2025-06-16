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
    let prompt_mapping = HashMap::new();

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

            // Add prompts from this instance to the mapping
            // Note: Prompts are fetched dynamically in core architecture.
            // For now, we'll create an empty mapping since prompts are handled on-demand.
            tracing::debug!(
                "Prompt mapping for instance {} (server: {}) - using on-demand fetching",
                instance_id,
                server_name
            );
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
    let all_prompts = Vec::new();

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

            // Collect all prompts from this instance
            // Note: In core architecture, prompts are fetched on-demand rather than cached.
            // For now, we'll return an empty list since prompts are handled dynamically.
            tracing::debug!(
                "Collecting prompts from instance {} (server: {}) - using on-demand fetching",
                instance_id,
                server_name
            );
        }
    }

    tracing::debug!("Collected {} total prompts", all_prompts.len());
    all_prompts
}
