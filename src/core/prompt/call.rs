// Prompt call handling module
// Contains functions for calling upstream prompts

use anyhow::{Context, Result};
use rmcp::model::{GetPromptRequestParam, GetPromptResult};
use serde_json::Value as JsonValue;
use tracing;

use crate::core::connection::UpstreamConnection;

/// Get a prompt from an upstream server
///
/// This function calls the upstream server to get a prompt with the specified name and arguments.
///
/// # Arguments
/// * `connection` - The upstream connection to use
/// * `prompt_name` - Name of the prompt to get
/// * `arguments` - Optional arguments for the prompt
///
/// # Returns
/// * `Ok(GetPromptResult)` - The prompt result from the upstream server
/// * `Err(_)` - If the prompt call failed
pub async fn get_upstream_prompt(
    connection: &UpstreamConnection,
    prompt_name: &str,
    arguments: Option<serde_json::Map<String, JsonValue>>,
) -> Result<GetPromptResult> {
    tracing::debug!(
        "Getting prompt '{}' from upstream server with arguments: {:?}",
        prompt_name,
        arguments
    );

    // Check if the connection has a service
    let service = match &connection.service {
        Some(service) => service,
        None => {
            return Err(anyhow::anyhow!(
                "No service available for upstream connection"
            ));
        }
    };

    // Prepare the request parameters
    let request_params = GetPromptRequestParam {
        name: prompt_name.to_string(),
        arguments,
    };

    // Call the upstream server
    let result = service.get_prompt(request_params).await.context(format!(
        "Failed to get prompt '{prompt_name}' from upstream server"
    ))?;

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
/// * `Ok(())` if the prompt name is valid
/// * `Err(_)` if the prompt name is invalid
pub fn validate_prompt_name(prompt_name: &str) -> Result<()> {
    if prompt_name.is_empty() {
        return Err(anyhow::anyhow!("Prompt name cannot be empty"));
    }

    // Check for invalid characters (similar to tool name validation)
    if prompt_name.contains('\n') || prompt_name.contains('\r') {
        return Err(anyhow::anyhow!(
            "Prompt name cannot contain newline characters"
        ));
    }

    // Check for extremely long names
    if prompt_name.len() > 1000 {
        return Err(anyhow::anyhow!(
            "Prompt name is too long (maximum 1000 characters)"
        ));
    }

    Ok(())
}
