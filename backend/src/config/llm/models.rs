use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub use mcpmate_llm::{
    LlmProviderDefaultParams, LlmProviderSpec, LlmProviderThinkingConfig, LlmProviderThinkingMode, LlmProviderType,
};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LlmProviderConfig {
    pub id: Option<String>,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub model_id: String,
    pub secret_alias: Option<String>,
    pub default_params_json: Option<String>,
    pub is_default: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl LlmProviderConfig {
    pub fn provider_spec(&self) -> anyhow::Result<LlmProviderSpec> {
        Ok(LlmProviderSpec {
            provider_type: self.provider_type.clone(),
            base_url: self.base_url.clone(),
            model_id: self.model_id.clone(),
            default_params: LlmProviderDefaultParams::from_json(&self.default_params_json)?,
        })
    }
}
