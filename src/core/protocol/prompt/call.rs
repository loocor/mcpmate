//! Prompt call handling module
//!
//! Contains functions for calling upstream prompts using core architecture

use super::types::PromptMapping;
use crate::core::{connection::UpstreamConnection, pool::UpstreamConnectionPool};
use anyhow::{Context, Result};
use rmcp::model::{GetPromptRequestParam, GetPromptResult};
use serde_json::Value as JsonValue;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing;

/// Get a prompt from an upstream server using prompt mapping
///
/// This function routes the prompt request to the correct upstream server
/// based on the prompt mapping and handles the response.
///
/// # Arguments
/// * `connection_pool` - The connection pool to use
/// * `prompt_mapping` - Mapping of prompt names to server/instance information
/// * `prompt_name` - Name of the prompt to get
/// * `arguments` - Optional arguments for the prompt
///
/// # Returns
/// * `Result<GetPromptResult>` - The prompt result from the upstream server
pub async fn get_upstream_prompt(
    connection_pool: &Arc<Mutex<UpstreamConnectionPool>>,
    prompt_mapping: &HashMap<String, PromptMapping>,
    prompt_name: &str,
    arguments: Option<serde_json::Map<String, JsonValue>>,
) -> Result<GetPromptResult> {
    tracing::debug!("Getting prompt '{}' with arguments: {:?}", prompt_name, arguments);

    // Validate the prompt name format
    validate_prompt_name(prompt_name)?;

    // Find the mapping for this prompt
    let mapping = prompt_mapping
        .get(prompt_name)
        .ok_or_else(|| anyhow::anyhow!("Prompt '{}' not found in any connected server", prompt_name))?;

    tracing::debug!(
        "Routing prompt request for '{}' to instance {} (server: {})",
        prompt_name,
        mapping.instance_id,
        mapping.server_name
    );

    // Get the connection from the pool
    // Note: mapping.server_name actually contains server_id for connection pool compatibility
    let pool = connection_pool.lock().await;
    let instances = pool
        .connections
        .get(&mapping.server_name)
        .ok_or_else(|| anyhow::anyhow!("Instance {} not found for prompt {}", mapping.instance_id, prompt_name))?;

    let conn = instances
        .get(&mapping.instance_id)
        .ok_or_else(|| anyhow::anyhow!("Instance {} not found for prompt {}", mapping.instance_id, prompt_name))?;

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

    // Check if the connection is ready
    if !conn.is_connected() {
        return Err(anyhow::anyhow!(
            "Server '{}' instance '{}' is not connected (status: {})",
            mapping.server_name,
            mapping.instance_id,
            conn.status
        ));
    }

    // Prepare the request parameters
    let request_params = GetPromptRequestParam {
        name: mapping.upstream_prompt_name.clone(),
        arguments,
    };

    // Call the upstream server
    let result = service.get_prompt(request_params).await.context(format!(
        "Failed to get prompt '{}' from upstream server '{}'",
        prompt_name, mapping.server_name
    ))?;

    tracing::debug!(
        "Successfully got prompt '{}' from server '{}' with {} messages",
        prompt_name,
        mapping.server_name,
        result.messages.len()
    );

    Ok(result)
}

/// Get a prompt from a specific upstream connection (direct connection)
///
/// This function calls the upstream server to get a prompt with the specified name and arguments.
/// This is a lower-level function that works with a direct connection.
///
/// # Arguments
/// * `connection` - The upstream connection to use
/// * `prompt_name` - Name of the prompt to get
/// * `arguments` - Optional arguments for the prompt
///
/// # Returns
/// * `Result<GetPromptResult>` - The prompt result from the upstream server
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

    // Validate the prompt name format
    validate_prompt_name(prompt_name)?;

    // Check if the connection has a service
    let service = match &connection.service {
        Some(service) => service,
        None => {
            return Err(anyhow::anyhow!("No service available for upstream connection"));
        }
    };

    // Check if the connection is ready
    if !connection.is_connected() {
        return Err(anyhow::anyhow!(
            "Connection is not ready (status: {})",
            connection.status
        ));
    }

    // Prepare the request parameters
    let request_params = GetPromptRequestParam {
        name: prompt_name.to_string(),
        arguments,
    };

    // Call the upstream server
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

/// Validate prompt name format
///
/// This function validates that a prompt name follows the expected format.
/// Prompt names should be non-empty strings without special characters.
///
/// # Arguments
/// * `prompt_name` - The prompt name to validate
///
/// # Returns
/// * `Result<()>` - Ok if the prompt name is valid, Err with description if invalid
pub fn validate_prompt_name(prompt_name: &str) -> Result<()> {
    if prompt_name.is_empty() {
        return Err(anyhow::anyhow!("Prompt name cannot be empty"));
    }

    // Check for invalid characters (similar to tool name validation)
    if prompt_name.contains('\n') || prompt_name.contains('\r') {
        return Err(anyhow::anyhow!("Prompt name cannot contain newline characters"));
    }

    // Check for extremely long names
    if prompt_name.len() > 1000 {
        return Err(anyhow::anyhow!("Prompt name is too long (maximum 1000 characters)"));
    }

    // Check for invalid characters that might cause issues
    if prompt_name.contains('\0') {
        return Err(anyhow::anyhow!("Prompt name cannot contain null characters"));
    }

    Ok(())
}

/// Check if a prompt is available in the mapping
///
/// This function checks if a prompt name is available in the current prompt mapping.
///
/// # Arguments
/// * `prompt_mapping` - The prompt mapping to check
/// * `prompt_name` - The prompt name to check
///
/// # Returns
/// * `bool` - True if the prompt is available, false otherwise
pub fn is_prompt_available(
    prompt_mapping: &HashMap<String, PromptMapping>,
    prompt_name: &str,
) -> bool {
    prompt_mapping.contains_key(prompt_name)
}

/// Get all available prompt names from the mapping
///
/// This function returns a list of all available prompt names from the mapping.
///
/// # Arguments
/// * `prompt_mapping` - The prompt mapping to query
///
/// # Returns
/// * `Vec<String>` - List of available prompt names
pub fn get_available_prompt_names(prompt_mapping: &HashMap<String, PromptMapping>) -> Vec<String> {
    prompt_mapping.keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_prompt_name_valid() {
        assert!(validate_prompt_name("valid_prompt").is_ok());
        assert!(validate_prompt_name("prompt-with-dashes").is_ok());
        assert!(validate_prompt_name("prompt123").is_ok());
        assert!(validate_prompt_name("UPPERCASE_PROMPT").is_ok());
    }

    #[test]
    fn test_validate_prompt_name_invalid() {
        assert!(validate_prompt_name("").is_err());
        assert!(validate_prompt_name("prompt\nwith\nnewlines").is_err());
        assert!(validate_prompt_name("prompt\rwith\rcarriage\rreturns").is_err());
        assert!(validate_prompt_name("prompt\0with\0nulls").is_err());

        // Test extremely long name
        let long_name = "a".repeat(1001);
        assert!(validate_prompt_name(&long_name).is_err());
    }

    #[test]
    fn test_is_prompt_available() {
        let mut mapping = HashMap::new();
        mapping.insert(
            "test_prompt".to_string(),
            PromptMapping {
                server_name: "test_server".to_string(),
                instance_id: "test_instance".to_string(),
                prompt: rmcp::model::Prompt {
                    name: "test_prompt".to_string(),
                    title: Some("Test prompt".to_string()),
                    description: Some("Test prompt".to_string()),
                    arguments: None,
                },
                upstream_prompt_name: "test_prompt".to_string(),
            },
        );

        assert!(is_prompt_available(&mapping, "test_prompt"));
        assert!(!is_prompt_available(&mapping, "nonexistent_prompt"));
    }

    #[test]
    fn test_get_available_prompt_names() {
        let mut mapping = HashMap::new();
        mapping.insert(
            "prompt1".to_string(),
            PromptMapping {
                server_name: "test_server".to_string(),
                instance_id: "test_instance".to_string(),
                prompt: rmcp::model::Prompt {
                    name: "prompt1".to_string(),
                    title: Some("Test prompt 1".to_string()),
                    description: Some("Test prompt 1".to_string()),
                    arguments: None,
                },
                upstream_prompt_name: "prompt1".to_string(),
            },
        );
        mapping.insert(
            "prompt2".to_string(),
            PromptMapping {
                server_name: "test_server".to_string(),
                instance_id: "test_instance".to_string(),
                prompt: rmcp::model::Prompt {
                    name: "prompt2".to_string(),
                    title: Some("Test prompt 2".to_string()),
                    description: Some("Test prompt 2".to_string()),
                    arguments: None,
                },
                upstream_prompt_name: "prompt2".to_string(),
            },
        );

        let names = get_available_prompt_names(&mapping);
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"prompt1".to_string()));
        assert!(names.contains(&"prompt2".to_string()));
    }
}
