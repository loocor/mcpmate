//! AI extraction API handler

use crate::{
    api::routes::AppState,
    core::ai::{AiConfig, extract_mcp_config},
};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Json as ResponseJson},
};
use base64::{Engine as _, engine::general_purpose};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AIExtractConfig {
    #[schemars(description = "Maximum number of tokens to generate")]
    pub max_tokens: Option<usize>,

    #[schemars(description = "Temperature for text generation")]
    pub temperature: Option<f64>,

    #[schemars(description = "Enable debug mode")]
    pub debug: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AIExtractReq {
    #[schemars(description = "Base64 encoded text content to extract MCP configuration from")]
    pub text_base64: String,

    #[schemars(description = "Optional AI configuration")]
    pub config: Option<AIExtractConfig>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AIExtractResp {
    #[schemars(description = "Extracted MCP configuration")]
    pub config: Value,

    #[schemars(description = "Whether extraction was successful")]
    pub success: bool,

    #[schemars(description = "Optional debug information")]
    pub debug_info: Option<String>,
}

impl IntoResponse for AIExtractResp {
    fn into_response(self) -> axum::response::Response {
        ResponseJson(self).into_response()
    }
}

/// Extract MCP configuration from text using AI
pub async fn extract_config(
    State(_app_state): State<Arc<AppState>>,
    Json(request): Json<AIExtractReq>,
) -> Result<ResponseJson<AIExtractResp>, StatusCode> {
    // Decode base64 text using new base64 API
    let text = match general_purpose::STANDARD.decode(&request.text_base64) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                tracing::error!("Failed to decode base64 text as UTF-8");
                return Err(StatusCode::BAD_REQUEST);
            }
        },
        Err(_) => {
            tracing::error!("Failed to decode base64 text");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Validate decoded input
    if text.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Build AI config
    let mut ai_config = AiConfig::default();

    if let Some(config) = request.config {
        if let Some(max_tokens) = config.max_tokens {
            ai_config = ai_config.with_max_tokens(max_tokens);
        }

        if let Some(temperature) = config.temperature {
            ai_config = ai_config.with_temperature(temperature);
        }

        if let Some(debug) = config.debug {
            ai_config = ai_config.with_debug(debug);
        }
    }

    // Extract configuration
    match extract_mcp_config(&text, Some(ai_config.clone())).await {
        Ok(config) => {
            let response = AIExtractResp {
                config,
                success: true,
                debug_info: if ai_config.debug {
                    Some("Extraction completed successfully".to_string())
                } else {
                    None
                },
            };
            Ok(ResponseJson(response))
        }
        Err(e) => {
            tracing::error!("AI extraction failed: {}", e);

            // Return empty config on error
            let response = AIExtractResp {
                config: serde_json::json!({"mcpServers": {}}),
                success: false,
                debug_info: if ai_config.debug {
                    Some(format!("Extraction failed: {}", e))
                } else {
                    None
                },
            };
            Ok(ResponseJson(response))
        }
    }
}
