//! Prompt call module
//!
//! Contains functions for handling prompt requests

use anyhow::{Context, Result};
use rmcp::model::{GetPromptRequestParam, GetPromptResult};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing;

use crate::recore::pool::UpstreamConnectionPool;
use crate::recore::protocol::prompt::types::PromptMapping;

/// Get a prompt from the appropriate upstream server
pub async fn get_upstream_prompt(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    prompt_mapping: &HashMap<String, PromptMapping>,
    name: &str,
    arguments: Option<serde_json::Value>,
) -> Result<GetPromptResult> {
    tracing::debug!("Getting prompt: {}", name);

    // Find the mapping for this prompt name
    let mapping = prompt_mapping
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Prompt not found: {}", name))?;

    tracing::debug!(
        "Routing prompt request for '{}' to instance {} (server: {})",
        name,
        mapping.instance_id,
        mapping.server_name
    );

    // Get the connection from the pool
    let pool = connection_pool.lock().await;
    let instances = pool.connections.get(&mapping.server_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Server {} not found for prompt {}",
            mapping.server_name,
            name
        )
    })?;

    let conn = instances.get(&mapping.instance_id).ok_or_else(|| {
        anyhow::anyhow!(
            "Instance {} not found for prompt {}",
            mapping.instance_id,
            name
        )
    })?;

    // Check if the connection has a service
    let service = match &conn.service {
        Some(service) => service,
        None => {
            return Err(anyhow::anyhow!(
                "No service available for instance {} (server: {})",
                mapping.instance_id,
                mapping.server_name
            ));
        }
    };

    // Forward the request to the upstream server
    let result = service
        .get_prompt(GetPromptRequestParam {
            name: mapping.upstream_prompt_name.clone(),
            arguments: arguments.and_then(|v| v.as_object().cloned()),
        })
        .await
        .context("Failed to get prompt from upstream server")?;

    tracing::debug!(
        "Successfully got prompt '{}' from server '{}'",
        name,
        mapping.server_name
    );

    Ok(result)
}

/// Validate prompt name format
pub fn validate_prompt_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow::anyhow!("Prompt name cannot be empty"));
    }

    // Check for common problematic characters
    if name.contains('\n') || name.contains('\r') {
        return Err(anyhow::anyhow!(
            "Prompt name cannot contain newline characters: {}",
            name
        ));
    }

    Ok(())
}
