use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LlmProviderType {
    OpenAiChat,
    OpenAiResponses,
    Anthropic,
}

impl std::fmt::Display for LlmProviderType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::OpenAiChat => write!(f, "openai_chat"),
            Self::OpenAiResponses => write!(f, "openai_responses"),
            Self::Anthropic => write!(f, "anthropic"),
        }
    }
}

impl std::str::FromStr for LlmProviderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "openai_chat" | "openai_compatible" => Ok(Self::OpenAiChat),
            "openai_responses" => Ok(Self::OpenAiResponses),
            "anthropic" => Ok(Self::Anthropic),
            _ => Err(anyhow::anyhow!("Unknown provider type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderDefaultParams {
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub thinking: LlmProviderThinkingConfig,
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

impl Default for LlmProviderDefaultParams {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            thinking: LlmProviderThinkingConfig::default(),
        }
    }
}

impl LlmProviderDefaultParams {
    pub fn from_json(json_str: &Option<String>) -> anyhow::Result<Self> {
        let Some(json_str) = json_str.as_deref() else {
            return Ok(Self::default());
        };

        serde_json::from_str(json_str).map_err(|err| anyhow::anyhow!("Invalid LLM provider default params: {err}"))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LlmProviderThinkingMode {
    #[default]
    Default,
    Disabled,
    Enabled,
}

pub const ANTHROPIC_THINKING_TOKEN_RESERVE: u32 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LlmProviderThinkingConfig {
    #[serde(default)]
    pub mode: LlmProviderThinkingMode,
    #[serde(default)]
    pub budget_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct LlmProviderSpec {
    pub provider_type: String,
    pub base_url: String,
    pub model_id: String,
    pub default_params: LlmProviderDefaultParams,
}
