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
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmProviderIdReq {
    pub id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmTestGenerateReq {
    pub provider_id: String,
    pub server_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub template_name: Option<String>,
    #[serde(default)]
    pub custom_scenario: Option<String>,
    #[serde(default = "default_count")]
    pub count: u32,
}

fn default_count() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmTestCaseData {
    pub id: String,
    pub params: serde_json::Value,
    pub description: String,
    pub test_type: String,
    pub expected_behavior: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmTestRunReq {
    pub provider_id: String,
    pub server_id: String,
    pub tool_name: String,
    pub cases: Vec<LlmTestCaseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmTestRunData {
    pub run_id: String,
    pub total_cases: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmTestResultData {
    pub case_id: String,
    pub params: serde_json::Value,
    pub actual_response: Option<serde_json::Value>,
    pub latency_ms: u64,
    pub status: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmTestEvent {
    pub run_id: String,
    pub event_type: String,
    pub case_id: Option<String>,
    pub result: Option<LlmTestResultData>,
    pub completed: u32,
    pub total: u32,
    pub error: Option<String>,
}
