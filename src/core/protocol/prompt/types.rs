//! Prompt protocol types
//!
//! Contains type definitions for prompt management

use rmcp::model::Prompt;

/// Prompt mapping information
///
/// This struct represents the mapping between a prompt name and the server/instance
/// that provides it. It is used to route prompt requests to the appropriate upstream server.
/// Unlike resources, prompts use name as unique identifier, similar to tools.
#[derive(Debug, Clone)]
pub struct PromptMapping {
    /// Name of the server that provides this prompt
    pub server_name: String,
    /// ID of the instance that provides this prompt
    pub instance_id: String,
    /// Original prompt definition
    pub prompt: Prompt,
    /// Original upstream prompt name (without any modifications)
    pub upstream_prompt_name: String,
}

/// Prompt template mapping information
///
/// This struct represents the mapping for prompt templates from upstream servers.
/// It's used for collecting and organizing prompt templates from multiple servers.
#[derive(Debug, Clone)]
pub struct PromptTemplateMapping {
    /// Name of the server that provides this prompt template
    pub server_name: String,
    /// ID of the instance that provides this prompt template
    pub instance_id: String,
    /// The prompt template definition (using Prompt instead of PromptTemplate)
    pub prompt_template: rmcp::model::Prompt,
}
