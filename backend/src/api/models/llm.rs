use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmProviderData {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub model_id: String,
    pub has_api_key: bool,
    pub is_default: bool,
    pub default_params: LlmDefaultParamsData,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmDefaultParamsData {
    pub temperature: f32,
    pub max_tokens: u32,
    pub thinking: LlmThinkingData,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmProviderCreateReq {
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub model_id: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub default_params: Option<LlmDefaultParamsReq>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmProviderUpdateReq {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub provider_type: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub api_key: Option<Option<String>>,
    #[serde(default)]
    pub default_params: Option<LlmDefaultParamsReq>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmDefaultParamsReq {
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub thinking: Option<LlmThinkingReq>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmThinkingData {
    pub mode: String,
    pub budget_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmThinkingReq {
    pub mode: String,
    #[serde(default)]
    pub budget_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmProviderIdReq {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmProviderModelPreviewReq {
    #[serde(default)]
    pub provider_id: Option<String>,
    pub provider_type: String,
    pub base_url: String,
    pub model_id: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmConnectivityResult {
    pub success: bool,
    pub latency_ms: u64,
    pub model: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmModelsData {
    pub models: Vec<String>,
}
