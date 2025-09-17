//! Prompt mapping module
//!
//! Contains functions for building prompt mappings from upstream servers

use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use futures::{future::BoxFuture, StreamExt};
use tracing;

use super::types::{PromptMapping, PromptTemplateMapping};
use crate::core::pool::UpstreamConnectionPool;

/// Determine concurrency limit based on OS CPU cores
use crate::core::capability::internal::{concurrency_limit, collect_capability_from_instance_peer, is_method_not_supported};

/// Build a mapping of prompt names to server/instance information
pub async fn build_prompt_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> HashMap<String, PromptMapping> {
    build_prompt_mapping_filtered(connection_pool, None).await
}

pub async fn build_prompt_mapping_filtered(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    enabled_server_ids: Option<&std::collections::HashSet<String>>,
) -> HashMap<String, PromptMapping> {
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

        for (instance_id, status, _supports_resources, supports_prompts, peer_opt) in instances {
            // Pre-filter: skip non-ready instances or those without prompt support
            if !matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) || !supports_prompts {
                continue;
            }
            let Some(peer) = peer_opt else { continue; };

            // Create async task for this server/instance combination
            let server_id_clone = server_id.clone();
            let instance_id_clone = instance_id.clone();
            tasks.push(async move {
                let fetch_page = move |
                    p: rmcp::service::Peer<rmcp::service::RoleClient>,
                    cursor: Option<String>
                | -> BoxFuture<'static, anyhow::Result<(Vec<rmcp::model::Prompt>, Option<String>)>> {
                    Box::pin(async move {
                        let result = p.list_prompts(Some(rmcp::model::PaginatedRequestParam { cursor })).await?;
                        Ok((result.prompts, result.next_cursor))
                    })
                };

                let server_name = match crate::core::capability::global_server_mapping_manager()
                    .get_name_by_id(&server_id_clone)
                    .await
                {
                    Some(name) => name,
                    None => server_id_clone.clone(),
                };

                collect_capability_from_instance_peer(
                    peer,
                    std::time::Duration::from_secs(10),
                    fetch_page,
                    |prompt, srv_name, srv_id, inst_id| PromptMapping {
                        server_name: srv_name.to_string(),
                        server_id: Some(srv_id.to_string()),
                        instance_id: inst_id.to_string(),
                        upstream_prompt_name: prompt.name.to_string(),
                        prompt,
                    },
                    &server_id_clone,
                    &server_name,
                    &instance_id_clone,
                    is_method_not_supported,
                )
                .await
            });
        }
    }

    // Step 3: Execute all tasks concurrently with bounded concurrency
    let results = futures::stream::iter(tasks)
        .buffer_unordered(concurrency_limit())
        .collect::<Vec<_>>()
        .await;

    // Step 4: Merge results and handle conflicts
    let mut prompt_mapping: HashMap<String, PromptMapping> = HashMap::new();
    for prompts in results {
        for prompt in prompts {
            let name = &prompt.upstream_prompt_name;

            // Check for name conflicts and log warnings
            if let Some(existing) = prompt_mapping.get(name) {
                tracing::warn!(
                    "Prompt '{}' is provided by both '{}' (instance: {}) and '{}' (instance: {}). Using the first one.",
                    name,
                    existing.server_name,
                    existing.instance_id,
                    prompt.server_name,
                    prompt.instance_id
                );
                continue; // Keep the first one, skip the conflicting one
            }

            prompt_mapping.insert(name.clone(), prompt);
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
    for (server_id, instances) in &pool.connections {
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
                server_id
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
pub async fn get_all_prompts(connection_pool: &Arc<Mutex<UpstreamConnectionPool>>) -> Vec<rmcp::model::Prompt> {
    // Build the prompt mapping first
    let prompt_mapping = build_prompt_mapping(connection_pool).await;

    // Extract all prompts from the mapping
    let all_prompts: Vec<rmcp::model::Prompt> = prompt_mapping.into_values().map(|mapping| mapping.prompt).collect();

    tracing::debug!("Collected {} total prompts", all_prompts.len());
    all_prompts
}


// collect_prompts_from_instance_peer removed in favor of generic helper
