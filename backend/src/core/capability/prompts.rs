//! Prompt capability helpers
//!
//! Aggregates mapping, status, and call utilities for upstream prompts.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{Context, Result, anyhow};
use futures::{StreamExt, future::BoxFuture};
use rmcp::model::{GetPromptRequestParams, GetPromptResult, PaginatedRequestParams, Prompt};
use serde_json::Value as JsonValue;
use sqlx::{Pool, Sqlite};
use tokio::sync::Mutex;
use tracing;

use crate::core::capability::facade::{
    collect_capability_from_instance_peer, concurrency_limit, is_method_not_supported,
};
use crate::core::pool::{UpstreamConnection, UpstreamConnectionPool};

#[derive(Debug, Clone)]
pub struct PromptMapping {
    pub server_name: String,
    pub server_id: Option<String>,
    pub instance_id: String,
    pub prompt: Prompt,
    pub upstream_prompt_name: String,
}

#[derive(Debug, Clone)]
pub struct PromptTemplateMapping {
    pub server_name: String,
    pub server_id: Option<String>,
    pub instance_id: String,
    pub prompt_template: Prompt,
}

pub async fn is_prompt_enabled(
    pool: &Pool<Sqlite>,
    server_name: &str,
    prompt_name: &str,
) -> Result<bool> {
    tracing::debug!(
        "Checking if prompt '{}' from server '{}' is enabled",
        prompt_name,
        server_name
    );

    let active_profile = crate::config::profile::get_active_profile(pool)
        .await
        .context("Failed to get active profile")?;

    if active_profile.is_empty() {
        tracing::debug!("No active profile found, prompt is disabled");
        return Ok(false);
    }

    let server = crate::config::server::get_server(pool, server_name)
        .await
        .context(format!("Failed to get server '{server_name}'"))?;

    let server_id = match server {
        Some(server) => match server.id {
            Some(id) => id,
            None => {
                tracing::warn!("Server '{}' has no ID, prompt is disabled", server_name);
                return Ok(false);
            }
        },
        None => {
            tracing::debug!("Server '{}' not found, prompt is disabled", server_name);
            return Ok(false);
        }
    };

    for profile in active_profile {
        if let Some(profile_id) = &profile.id {
            let enabled_prompts = crate::config::profile::get_enabled_prompts_for_profile(pool, profile_id)
                .await
                .context(format!("Failed to get enabled prompts for profile '{profile_id}'"))?;

            for prompt in enabled_prompts {
                if prompt.server_id == server_id && prompt.prompt_name == prompt_name {
                    tracing::debug!(
                        "Prompt '{}' from server '{}' is enabled in profile '{}'",
                        prompt_name,
                        server_name,
                        profile.name
                    );
                    return Ok(true);
                }
            }
        }
    }

    tracing::debug!(
        "Prompt '{}' from server '{}' is not enabled in any active profile",
        prompt_name,
        server_name
    );
    Ok(false)
}

pub async fn get_prompt_status(
    pool: &Pool<Sqlite>,
    server_name: &str,
    prompt_name: &str,
) -> Result<(String, bool)> {
    tracing::debug!(
        "Getting prompt status for '{}' from server '{}'",
        prompt_name,
        server_name
    );

    let enabled = is_prompt_enabled(pool, server_name, prompt_name).await?;
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
                return Err(anyhow!("Server '{}' has no ID", server_name));
            }
        },
        None => {
            return Err(anyhow!("Server '{}' not found", server_name));
        }
    };

    for profile in active_profile {
        if let Some(profile_id) = &profile.id {
            let prompts = crate::config::profile::get_prompts_for_profile(pool, profile_id)
                .await
                .context(format!("Failed to get prompts for profile '{profile_id}'"))?;

            for prompt in prompts {
                if prompt.server_id == server_id && prompt.prompt_name == prompt_name {
                    if let Some(prompt_id) = prompt.id {
                        return Ok((prompt_id, enabled));
                    }
                }
            }
        }
    }

    Err(anyhow!(
        "Prompt '{}' from server '{}' not found in any active profile",
        prompt_name,
        server_name
    ))
}

pub async fn build_prompt_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> HashMap<String, PromptMapping> {
    build_prompt_mapping_filtered(connection_pool, None).await
}

pub async fn build_prompt_mapping_filtered(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    enabled_server_ids: Option<&HashSet<String>>,
) -> HashMap<String, PromptMapping> {
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

        for (instance_id, status, _supports_resources, supports_prompts, peer_opt) in instances {
            if !matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) || !supports_prompts {
                continue;
            }
            let Some(peer) = peer_opt else { continue };

            let server_id_clone = server_id.clone();
            let instance_id_clone = instance_id.clone();
            tasks.push(async move {
                let fetch_page =
                    move |p: rmcp::service::Peer<rmcp::service::RoleClient>,
                          cursor: Option<String>|
                          -> BoxFuture<'static, anyhow::Result<(Vec<Prompt>, Option<String>)>> {
                        Box::pin(async move {
                            let result = p
                                .list_prompts(Some(PaginatedRequestParams::default().with_cursor(cursor)))
                                .await?;
                            Ok((result.prompts, result.next_cursor))
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
                    |prompt, srv_name, srv_id, inst_id| {
                        let upstream_name = prompt.name.to_string();
                        PromptMapping {
                            server_name: srv_name.to_string(),
                            server_id: Some(srv_id.to_string()),
                            instance_id: inst_id.to_string(),
                            upstream_prompt_name: upstream_name,
                            prompt,
                        }
                    },
                    &server_id_clone,
                    &server_name,
                    &instance_id_clone,
                    is_method_not_supported,
                )
                .await;

                outcome.items
            });
        }
    }

    let results: Vec<Vec<PromptMapping>> = futures::stream::iter(tasks)
        .buffer_unordered(concurrency_limit())
        .collect()
        .await;

    let mut prompt_mapping: HashMap<String, PromptMapping> = HashMap::new();
    for prompts in results {
        for prompt in prompts {
            let name = &prompt.prompt.name;
            if let Some(existing) = prompt_mapping.get(name) {
                tracing::warn!(
                    "Prompt name conflict detected: '{}' provided by '{}' and '{}'. Keeping first occurrence.",
                    name,
                    existing.server_name,
                    prompt.server_name
                );
                continue;
            }
            prompt_mapping.insert(name.clone(), prompt);
        }
    }

    tracing::debug!("Built prompt mapping with {} prompts", prompt_mapping.len());
    prompt_mapping
}

pub async fn build_prompt_template_mapping(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>
) -> Vec<PromptTemplateMapping> {
    let snapshot = {
        let pool = connection_pool.lock().await;
        pool.get_snapshot()
    };

    let mut tasks = Vec::new();
    for (server_id, instances) in snapshot {
        for (instance_id, status, _supports_resources, supports_prompts, peer_opt) in instances {
            if !matches!(status, crate::core::foundation::types::ConnectionStatus::Ready) || !supports_prompts {
                continue;
            }
            let Some(peer) = peer_opt else { continue };

            let server_id_clone = server_id.clone();
            tasks.push(async move {
                let fetch_page =
                    move |p: rmcp::service::Peer<rmcp::service::RoleClient>,
                          cursor: Option<String>|
                          -> BoxFuture<'static, anyhow::Result<(Vec<Prompt>, Option<String>)>> {
                        Box::pin(async move {
                            let result = p
                                .list_prompts(Some(PaginatedRequestParams::default().with_cursor(cursor)))
                                .await?;
                            Ok((result.prompts, result.next_cursor))
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
                    |prompt, srv_name, srv_id, inst_id| PromptTemplateMapping {
                        server_name: srv_name.to_string(),
                        server_id: Some(srv_id.to_string()),
                        instance_id: inst_id.to_string(),
                        prompt_template: prompt,
                    },
                    &server_id_clone,
                    &server_name,
                    &instance_id,
                    is_method_not_supported,
                )
                .await;

                outcome.items
            });
        }
    }

    let results: Vec<Vec<PromptTemplateMapping>> = futures::stream::iter(tasks)
        .buffer_unordered(concurrency_limit())
        .collect()
        .await;

    let mut templates = Vec::new();
    for mut mapping in results {
        templates.append(&mut mapping);
    }
    templates
}

pub async fn get_all_prompts(connection_pool: &Arc<Mutex<UpstreamConnectionPool>>) -> Vec<Prompt> {
    build_prompt_mapping(connection_pool)
        .await
        .into_values()
        .map(|mapping| mapping.prompt)
        .collect()
}

pub async fn get_upstream_prompt(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    prompt_mapping: &HashMap<String, PromptMapping>,
    prompt_name: &str,
    arguments: Option<serde_json::Map<String, JsonValue>>,
    target_server_id: Option<&str>,
    connection_selection: Option<&crate::core::capability::ConnectionSelection>,
) -> Result<GetPromptResult> {
    tracing::debug!("Getting prompt '{}' with arguments: {:?}", prompt_name, arguments);

    validate_prompt_name(prompt_name)?;

    let mapping_opt = prompt_mapping.get(prompt_name);
    if let Some(mapping) = mapping_opt {
        tracing::debug!(
            "Routing prompt request for '{}' to instance {} (server: {})",
            prompt_name,
            mapping.instance_id,
            mapping.server_name
        );
    } else {
        tracing::debug!(
            "Routing prompt request for '{}' with no mapping; will use target server if provided",
            prompt_name
        );
    }

    let (service, upstream_prompt_name) = {
        let mut pool = connection_pool.lock().await;
        let (server_key_owned, upstream_name_owned, desired_instance_id_opt): (String, String, Option<String>) =
            if let Some(mapping) = mapping_opt {
                (
                    mapping.server_id.as_ref().unwrap_or(&mapping.server_name).to_string(),
                    mapping.upstream_prompt_name.clone(),
                    Some(mapping.instance_id.clone()),
                )
            } else if let Some(server_id) = target_server_id {
                (server_id.to_string(), prompt_name.to_string(), None)
            } else {
                return Err(anyhow!("Prompt '{}' not found in any connected server", prompt_name));
            };
        let server_key = &server_key_owned;

        let mut target_instance_id: Option<String> = None;
        if let Some(instances) = pool.connections.get(server_key) {
            if let Some(desired) = &desired_instance_id_opt {
                if let Some(conn) = instances.get(desired) {
                    if conn.is_connected() && conn.service.is_some() {
                        target_instance_id = Some(desired.clone());
                    }
                }
            }
        }

        if target_instance_id.is_none() {
            if let Some(selection) = connection_selection {
                let scoped_selection = crate::core::capability::ConnectionSelection {
                    server_id: server_key.to_string(),
                    affinity_key: selection.affinity_key.clone(),
                };
                if let Err(e) = pool.ensure_connected_with_selection(&scoped_selection).await {
                    return Err(anyhow!(
                        "Failed to ensure scoped connection for server '{}': {}",
                        server_key,
                        e
                    ));
                }
                let iid = pool
                    .select_instance_id(&scoped_selection)
                    .map_err(|e| anyhow!("Failed to select scoped instance for server '{}': {}", server_key, e))?;
                target_instance_id = Some(iid);
            } else {
                if let Err(e) = pool.ensure_connected(server_key).await {
                    return Err(anyhow!(
                        "Failed to ensure connection for server '{}': {}",
                        server_key,
                        e
                    ));
                }
                let iid = pool
                    .get_default_instance_id(server_key)
                    .map_err(|e| anyhow!("Failed to get default instance for server '{}': {}", server_key, e))?;
                target_instance_id = Some(iid);
            }
        }

        let iid = target_instance_id.expect("instance id must be set");
        let instances = pool
            .connections
            .get(server_key)
            .ok_or_else(|| anyhow!("Server '{}' not found in pool", server_key))?;
        let conn = instances
            .get(&iid)
            .ok_or_else(|| anyhow!("Instance '{}' not found for server '{}'", iid, server_key))?;
        let service = conn
            .service
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("No service available for instance '{}' of server '{}'", iid, server_key))?;

        (service, upstream_name_owned)
    };

    let mut request_params = GetPromptRequestParams::new(upstream_prompt_name);
    if let Some(arguments) = arguments {
        request_params = request_params.with_arguments(arguments);
    }

    let result = service
        .get_prompt(request_params)
        .await
        .context(format!("Failed to get prompt '{}' from upstream server", prompt_name))?;

    tracing::debug!(
        "Successfully got prompt '{}' with {} messages",
        prompt_name,
        result.messages.len()
    );

    Ok(result)
}

pub async fn get_upstream_prompt_direct(
    connection: &UpstreamConnection,
    prompt_name: &str,
    arguments: Option<serde_json::Map<String, JsonValue>>,
) -> Result<GetPromptResult> {
    tracing::debug!(
        "Getting prompt '{}' from direct connection with arguments: {:?}",
        prompt_name,
        arguments
    );

    validate_prompt_name(prompt_name)?;

    let service = connection
        .service
        .as_ref()
        .ok_or_else(|| anyhow!("No service available for upstream connection"))?;

    if !connection.is_connected() {
        return Err(anyhow!("Connection is not ready (status: {})", connection.status));
    }

    let mut request_params = GetPromptRequestParams::new(prompt_name.to_string());
    if let Some(arguments) = arguments {
        request_params = request_params.with_arguments(arguments);
    }

    let result = service
        .get_prompt(request_params)
        .await
        .context(format!("Failed to get prompt '{}' from upstream server", prompt_name))?;

    tracing::debug!(
        "Successfully got prompt '{}' from upstream server with {} messages",
        prompt_name,
        result.messages.len()
    );

    Ok(result)
}

pub fn get_available_prompt_names(prompt_mapping: &HashMap<String, PromptMapping>) -> Vec<String> {
    let mut names: Vec<String> = prompt_mapping.keys().cloned().collect();
    names.sort();
    names
}

pub fn validate_prompt_name(prompt_name: &str) -> Result<()> {
    if prompt_name.is_empty() {
        return Err(anyhow!("Prompt name cannot be empty"));
    }
    if prompt_name.contains(['\n', '\r', '\0']) {
        return Err(anyhow!("Prompt name contains invalid characters"));
    }
    if prompt_name.len() > 256 {
        return Err(anyhow!("Prompt name is too long"));
    }
    Ok(())
}
