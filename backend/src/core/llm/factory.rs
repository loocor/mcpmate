use anyhow::Result;

use crate::config::llm::models::LlmProviderConfig;
use super::anthropic::AnthropicProvider;
use super::openai::OpenAiProvider;
use super::provider::LlmProvider;

pub fn create_provider(config: &LlmProviderConfig, api_key: &str) -> Result<Box<dyn LlmProvider>> {
    match config.provider_type.as_str() {
        "openai_chat" | "openai_compatible" => Ok(Box::new(OpenAiProvider::new(
            &config.base_url,
            api_key,
            &config.model_id,
        ))),
        "openai_responses" => Err(anyhow::anyhow!(
            "OpenAI Responses API (/v1/responses) is not yet implemented. Use OpenAI Chat Completions instead."
        )),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(
            &config.base_url,
            api_key,
            &config.model_id,
        ))),
        _ => Err(anyhow::anyhow!(
            "Unknown provider type: {}",
            config.provider_type
        )),
    }
}
