use anyhow::Result;

use crate::anthropic::AnthropicProvider;
use crate::config::LlmProviderSpec;
use crate::openai::OpenAiProvider;
use crate::provider::LlmProvider;

pub fn create_provider(
    spec: &LlmProviderSpec,
    api_key: &str,
) -> Result<Box<dyn LlmProvider>> {
    match spec.provider_type.as_str() {
        "openai_chat" | "openai_compatible" => {
            Ok(Box::new(OpenAiProvider::new(&spec.base_url, api_key, &spec.model_id)))
        }
        "openai_responses" => Err(anyhow::anyhow!(
            "OpenAI Responses API (/v1/responses) is not yet implemented. Use OpenAI Chat Completions instead."
        )),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(
            &spec.base_url,
            api_key,
            &spec.model_id,
            spec.default_params.thinking.clone(),
        ))),
        _ => Err(anyhow::anyhow!("Unknown provider type: {}", spec.provider_type)),
    }
}
