use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::provider::LlmProvider;
use crate::types::*;

pub struct OpenAiProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    pub fn new(
        base_url: &str,
        api_key: &str,
        model: &str,
    ) -> Self {
        Self {
            client: Client::new(),
            base_url: normalize_openai_base_url(base_url),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }
}

fn normalize_openai_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    let base = if trimmed.is_empty() {
        "https://api.openai.com/v1"
    } else {
        trimmed
    };

    let Ok(mut url) = Url::parse(base) else {
        return base.to_string();
    };

    if url.host_str() == Some("api.openai.com") && matches!(url.path(), "" | "/") {
        url.set_path("/v1");
    }

    url.as_str().trim_end_matches('/').to_string()
}

fn summarize_response_body(body: &str) -> String {
    const MAX_CHARS: usize = 500;
    let summary: String = body.chars().take(MAX_CHARS).collect();
    if body.chars().count() <= MAX_CHARS {
        summary
    } else {
        format!("{}...", summary.trim_end())
    }
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunction,
}

#[derive(Serialize, Deserialize, Clone)]
struct OpenAiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiFunctionCall,
}

#[derive(Serialize, Deserialize, Clone)]
struct OpenAiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAiStreamResponse {
    choices: Vec<OpenAiStreamChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
}

#[derive(Deserialize, Clone)]
struct OpenAiStreamDelta {
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiStreamToolCallDelta>>,
}

#[derive(Deserialize, Clone)]
struct OpenAiStreamToolCallDelta {
    index: u32,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OpenAiStreamFunctionDelta>,
}

#[derive(Deserialize, Clone)]
struct OpenAiStreamFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Deserialize)]
struct OpenAiModel {
    id: String,
}

fn to_openai_messages(messages: &[ChatMessage]) -> Vec<OpenAiMessage> {
    messages
        .iter()
        .map(|m| OpenAiMessage {
            role: match m.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
                Role::Tool => "tool".to_string(),
            },
            content: Some(m.content.clone()),
            tool_calls: m.tool_calls.as_ref().map(|tc| {
                tc.iter()
                    .map(|t| OpenAiToolCall {
                        id: t.id.clone(),
                        call_type: "function".to_string(),
                        function: OpenAiFunctionCall {
                            name: t.function.name.clone(),
                            arguments: t.function.arguments.clone(),
                        },
                    })
                    .collect()
            }),
            tool_call_id: m.tool_call_id.clone(),
        })
        .collect()
}

fn to_openai_tools(tools: &[LlmTool]) -> Vec<OpenAiTool> {
    tools
        .iter()
        .map(|t| OpenAiTool {
            tool_type: "function".to_string(),
            function: OpenAiFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        })
        .collect()
}

fn from_openai_message(msg: &OpenAiMessage) -> ChatMessage {
    ChatMessage {
        role: match msg.role.as_str() {
            "system" => Role::System,
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => Role::Assistant,
        },
        content: msg.content.clone().unwrap_or_default(),
        tool_calls: msg.tool_calls.as_ref().map(|tc| {
            tc.iter()
                .map(|t| ToolCall {
                    id: t.id.clone(),
                    function: FunctionCall {
                        name: t.function.name.clone(),
                        arguments: t.function.arguments.clone(),
                    },
                })
                .collect()
        }),
        tool_call_id: msg.tool_call_id.clone(),
    }
}

fn from_openai_stream_tool_call_delta(delta: &OpenAiStreamToolCallDelta) -> ToolCallDelta {
    ToolCallDelta {
        index: delta.index,
        id: delta.id.clone(),
        function: delta.function.as_ref().map(|function| FunctionCallDelta {
            name: function.name.clone(),
            arguments: function.arguments.clone(),
        }),
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn provider_type(&self) -> &str {
        "openai_compatible"
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    async fn chat_completion(
        &self,
        mut request: ChatRequest,
    ) -> Result<ChatResponse> {
        if request.max_tokens.is_none() {
            request.max_tokens = Some(4096);
        }

        let openai_req = OpenAiRequest {
            model: self.model.clone(),
            messages: to_openai_messages(&request.messages),
            tools: request.tools.as_ref().map(|t| to_openai_tools(t)),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
        };

        let url = format!("{}/chat/completions", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_req)
            .send()
            .await
            .context("Failed to send request to LLM provider")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("LLM provider returned {}: {}", status, body);
            anyhow::bail!(
                "LLM provider returned error (status {}): {}",
                status,
                summarize_response_body(&body)
            );
        }

        let openai_resp: OpenAiResponse = resp.json().await.context("Failed to parse LLM provider response")?;

        let message = openai_resp
            .choices
            .first()
            .map(|c| from_openai_message(&c.message))
            .unwrap_or(ChatMessage {
                role: Role::Assistant,
                content: String::new(),
                tool_calls: None,
                tool_call_id: None,
            });

        Ok(ChatResponse {
            message,
            usage: openai_resp.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    async fn chat_completion_stream(
        &self,
        mut request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        if request.max_tokens.is_none() {
            request.max_tokens = Some(4096);
        }

        let openai_req = OpenAiRequest {
            model: self.model.clone(),
            messages: to_openai_messages(&request.messages),
            tools: request.tools.as_ref().map(|t| to_openai_tools(t)),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: true,
        };

        let url = format!("{}/chat/completions", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_req)
            .send()
            .await
            .context("Failed to send streaming request to LLM provider")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("LLM provider returned {}: {}", status, body);
            anyhow::bail!(
                "LLM provider returned error (status {}): {}",
                status,
                summarize_response_body(&body)
            );
        }

        let byte_stream = resp.bytes_stream();

        let stream = futures::stream::unfold(
            (byte_stream, String::new()),
            |(mut byte_stream, mut buffer)| async move {
                use futures::TryStreamExt;
                loop {
                    if let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].trim().to_string();
                        buffer = buffer[newline_pos + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        let data = line.strip_prefix("data: ").unwrap_or(&line);
                        if data == "[DONE]" {
                            return None;
                        }

                        match serde_json::from_str::<OpenAiStreamResponse>(data) {
                            Ok(resp) => {
                                let delta = resp
                                    .choices
                                    .first()
                                    .map(|c| ChatDelta {
                                        role: c.delta.role.as_ref().and_then(|r| match r.as_str() {
                                            "assistant" => Some(Role::Assistant),
                                            _ => None,
                                        }),
                                        content: c.delta.content.clone(),
                                        tool_calls: c
                                            .delta
                                            .tool_calls
                                            .as_ref()
                                            .map(|tc| tc.iter().map(from_openai_stream_tool_call_delta).collect()),
                                    })
                                    .unwrap_or(ChatDelta {
                                        role: None,
                                        content: None,
                                        tool_calls: None,
                                    });

                                return Some((
                                    Ok(ChatChunk {
                                        delta,
                                        usage: resp.usage.map(|u| TokenUsage {
                                            prompt_tokens: u.prompt_tokens,
                                            completion_tokens: u.completion_tokens,
                                            total_tokens: u.total_tokens,
                                        }),
                                    }),
                                    (byte_stream, buffer),
                                ));
                            }
                            Err(_) => continue,
                        }
                    }

                    match byte_stream.try_next().await {
                        Ok(Some(bytes)) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));
                        }
                        Ok(None) => return None,
                        Err(e) => return Some((Err(anyhow::anyhow!("Stream error: {}", e)), (byte_stream, buffer))),
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/models", self.base_url);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .context("Failed to list models")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to list models (status {}): {}",
                status,
                summarize_response_body(&body)
            );
        }

        let models_resp: OpenAiModelsResponse = resp.json().await.context("Failed to parse models response")?;

        let mut models: Vec<String> = models_resp.data.into_iter().map(|m| m.id).collect();
        models.sort();
        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_provider_stream_tool_call_index_when_id_is_present() {
        let delta = OpenAiStreamToolCallDelta {
            index: 3,
            id: Some("call_3".to_string()),
            function: None,
        };

        let mapped = from_openai_stream_tool_call_delta(&delta);

        assert_eq!(mapped.index, 3);
        assert_eq!(mapped.id.as_deref(), Some("call_3"));
    }
}
