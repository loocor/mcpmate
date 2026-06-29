use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::config::{ANTHROPIC_THINKING_TOKEN_RESERVE, LlmProviderThinkingConfig, LlmProviderThinkingMode};
use crate::provider::{ConnectivityResult, LlmProvider};
use crate::types::*;

pub struct AnthropicProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
    thinking: LlmProviderThinkingConfig,
}

impl AnthropicProvider {
    pub fn new(
        base_url: &str,
        api_key: &str,
        model: &str,
        thinking: LlmProviderThinkingConfig,
    ) -> Self {
        let base = if base_url.is_empty() {
            "https://api.anthropic.com".to_string()
        } else {
            base_url.trim_end_matches('/').to_string()
        };
        Self {
            client: Client::new(),
            base_url: base,
            api_key: api_key.to_string(),
            model: model.to_string(),
            thinking,
        }
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<LlmThinkingPayload>,
    stream: bool,
}

#[derive(Serialize)]
struct LlmThinkingPayload {
    #[serde(rename = "type")]
    thinking_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    budget_tokens: Option<u32>,
}

fn thinking_payload(config: &LlmProviderThinkingConfig) -> Option<LlmThinkingPayload> {
    match config.mode {
        LlmProviderThinkingMode::Default => None,
        LlmProviderThinkingMode::Disabled => Some(LlmThinkingPayload {
            thinking_type: "disabled",
            budget_tokens: None,
        }),
        LlmProviderThinkingMode::Enabled => Some(LlmThinkingPayload {
            thinking_type: "enabled",
            budget_tokens: config.budget_tokens,
        }),
    }
}

fn validate_thinking_budget(
    thinking: &LlmProviderThinkingConfig,
    max_tokens: u32,
) -> Result<()> {
    if thinking.mode != LlmProviderThinkingMode::Enabled {
        return Ok(());
    }

    let budget_tokens = thinking
        .budget_tokens
        .context("Anthropic thinking mode requires budget_tokens")?;
    if budget_tokens == 0 {
        anyhow::bail!("Anthropic thinking budget_tokens must be greater than zero");
    }
    if budget_tokens.saturating_add(ANTHROPIC_THINKING_TOKEN_RESERVE) > max_tokens {
        anyhow::bail!(
            "Anthropic thinking budget_tokens ({}) must leave at least {} tokens for output within max_tokens ({})",
            budget_tokens,
            ANTHROPIC_THINKING_TOKEN_RESERVE,
            max_tokens
        );
    }
    Ok(())
}

fn thinking_compatible_temperature(
    thinking: &LlmProviderThinkingConfig,
    temperature: Option<f32>,
) -> Option<f32> {
    match thinking.mode {
        LlmProviderThinkingMode::Enabled => None,
        LlmProviderThinkingMode::Default | LlmProviderThinkingMode::Disabled => temperature,
    }
}

fn validate_thinking_tool_use(
    thinking: &LlmProviderThinkingConfig,
    tools: Option<&Vec<LlmTool>>,
) -> Result<()> {
    if thinking.mode == LlmProviderThinkingMode::Enabled && tools.is_some_and(|items| !items.is_empty()) {
        anyhow::bail!("Anthropic thinking with tool use is not supported yet");
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        #[serde(default)]
        signature: Option<String>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: String },
}

#[derive(Serialize, Deserialize, Clone)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
}

#[derive(Deserialize)]
struct AnthropicModel {
    id: String,
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

fn parse_models_response(
    body: &str,
    context_label: &str,
) -> Result<Vec<String>> {
    let models_resp: AnthropicModelsResponse = serde_json::from_str(body).with_context(|| {
        format!(
            "Failed to parse {context_label} models response: {}",
            summarize_response_body(body)
        )
    })?;

    let mut models: Vec<String> = models_resp.data.into_iter().map(|m| m.id).collect();
    models.sort();
    Ok(models)
}

fn sibling_openai_models_url(base_url: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(base_url).ok()?;
    if url.path().trim_end_matches('/') != "/anthropic" {
        return None;
    }
    url.set_path("/v1/models");
    url.set_query(None);
    Some(url.to_string())
}

async fn list_openai_compatible_models(
    client: &Client,
    api_key: &str,
    url: &str,
) -> Result<Vec<String>> {
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
        .context("Failed to list OpenAI-compatible models for Anthropic provider")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!(
            "OpenAI-compatible models endpoint returned error (status {}): {}",
            status,
            summarize_response_body(&body)
        );
    }

    let body = resp
        .text()
        .await
        .context("Failed to read OpenAI-compatible models response")?;
    parse_models_response(&body, "OpenAI-compatible")
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    index: Option<u32>,
    #[serde(default)]
    message: Option<AnthropicStreamMessage>,
    #[serde(default)]
    content_block: Option<AnthropicStreamContentBlock>,
    #[serde(default)]
    delta: Option<AnthropicStreamDelta>,
    #[serde(default)]
    usage: Option<AnthropicStreamUsage>,
}

#[derive(Deserialize)]
struct AnthropicStreamMessage {
    #[serde(default)]
    usage: Option<AnthropicStreamUsage>,
}

#[derive(Deserialize, Clone)]
struct AnthropicStreamUsage {
    #[serde(default)]
    input_tokens: Option<u32>,
    #[serde(default)]
    output_tokens: Option<u32>,
}

#[derive(Deserialize, Clone)]
struct AnthropicStreamContentBlock {
    #[serde(default, rename = "type")]
    block_type: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize, Clone)]
struct AnthropicStreamDelta {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    partial_json: Option<String>,
    #[serde(default)]
    input_json: Option<String>,
}

fn empty_stream_delta() -> ChatDelta {
    ChatDelta {
        role: None,
        content: None,
        tool_calls: None,
    }
}

fn anthropic_tool_start_chunk(
    event: &AnthropicStreamEvent,
    block: &AnthropicStreamContentBlock,
) -> Option<ChatChunk> {
    if block.block_type.as_deref() != Some("tool_use") {
        return None;
    }

    Some(ChatChunk {
        delta: ChatDelta {
            role: None,
            content: None,
            tool_calls: Some(vec![ToolCallDelta {
                index: event.index.unwrap_or(0),
                id: block.id.clone(),
                function: Some(FunctionCallDelta {
                    name: block.name.clone(),
                    arguments: None,
                }),
            }]),
        },
        usage: None,
    })
}

fn anthropic_content_delta_chunk(
    index: Option<u32>,
    delta: &AnthropicStreamDelta,
) -> Option<ChatChunk> {
    if let Some(text) = delta.text.clone() {
        return Some(ChatChunk {
            delta: ChatDelta {
                role: None,
                content: Some(text),
                tool_calls: None,
            },
            usage: None,
        });
    }

    let arguments = delta.partial_json.clone().or_else(|| delta.input_json.clone())?;
    Some(ChatChunk {
        delta: ChatDelta {
            role: None,
            content: None,
            tool_calls: Some(vec![ToolCallDelta {
                index: index.unwrap_or(0),
                id: None,
                function: Some(FunctionCallDelta {
                    name: None,
                    arguments: Some(arguments),
                }),
            }]),
        },
        usage: None,
    })
}

fn anthropic_usage_chunk(
    prompt_tokens: Option<u32>,
    usage: &AnthropicStreamUsage,
) -> Option<ChatChunk> {
    let completion_tokens = usage.output_tokens?;
    let prompt_tokens = usage.input_tokens.or(prompt_tokens).unwrap_or(0);
    Some(ChatChunk {
        delta: empty_stream_delta(),
        usage: Some(TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }),
    })
}

fn to_anthropic_messages(messages: &[ChatMessage]) -> (Option<String>, Vec<AnthropicMessage>) {
    let mut system = None;
    let mut anthropic_msgs = Vec::new();

    for msg in messages {
        match msg.role {
            Role::System => {
                system = Some(msg.content.clone());
            }
            Role::User => {
                anthropic_msgs.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: AnthropicContent::Text(msg.content.clone()),
                });
            }
            Role::Assistant => {
                if let Some(tool_calls) = &msg.tool_calls {
                    let mut blocks = Vec::new();
                    if !msg.content.is_empty() {
                        blocks.push(AnthropicContentBlock::Text {
                            text: msg.content.clone(),
                        });
                    }
                    for tc in tool_calls {
                        let input: serde_json::Value =
                            serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::Value::Null);
                        blocks.push(AnthropicContentBlock::ToolUse {
                            id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            input,
                        });
                    }
                    anthropic_msgs.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: AnthropicContent::Blocks(blocks),
                    });
                } else {
                    anthropic_msgs.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: AnthropicContent::Text(msg.content.clone()),
                    });
                }
            }
            Role::Tool => {
                let tool_use_id = msg.tool_call_id.clone().unwrap_or_default();
                anthropic_msgs.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: AnthropicContent::Blocks(vec![AnthropicContentBlock::ToolResult {
                        tool_use_id,
                        content: msg.content.clone(),
                    }]),
                });
            }
        }
    }

    (system, anthropic_msgs)
}

fn to_anthropic_tools(tools: &[LlmTool]) -> Vec<AnthropicTool> {
    tools
        .iter()
        .map(|t| AnthropicTool {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: t.parameters.clone(),
        })
        .collect()
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn provider_type(&self) -> &str {
        "anthropic"
    }

    fn model_id(&self) -> &str {
        &self.model
    }

    async fn chat_completion(
        &self,
        request: ChatRequest,
    ) -> Result<ChatResponse> {
        let (system, messages) = to_anthropic_messages(&request.messages);
        let max_tokens = request.max_tokens.unwrap_or(4096);
        validate_thinking_budget(&self.thinking, max_tokens)?;
        validate_thinking_tool_use(&self.thinking, request.tools.as_ref())?;

        let anthropic_req = AnthropicRequest {
            model: self.model.clone(),
            max_tokens,
            messages,
            system,
            tools: request.tools.as_ref().map(|t| to_anthropic_tools(t)),
            temperature: thinking_compatible_temperature(&self.thinking, request.temperature),
            thinking: thinking_payload(&self.thinking),
            stream: false,
        };

        let url = format!("{}/v1/messages", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_req)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("Anthropic returned {}: {}", status, body);
            anyhow::bail!(
                "Anthropic returned error (status {}): {}",
                status,
                summarize_response_body(&body)
            );
        }

        let body = resp.text().await.context("Failed to read Anthropic response")?;
        let anthropic_resp: AnthropicResponse = serde_json::from_str(&body)
            .with_context(|| format!("Failed to parse Anthropic response: {}", summarize_response_body(&body)))?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in &anthropic_resp.content {
            match block {
                AnthropicContentBlock::Text { text } => {
                    content.push_str(text);
                }
                AnthropicContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id: id.clone(),
                        function: FunctionCall {
                            name: name.clone(),
                            arguments: serde_json::to_string(input).unwrap_or_default(),
                        },
                    });
                }
                AnthropicContentBlock::Thinking { .. } => {}
                _ => {}
            }
        }

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content,
                tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
                tool_call_id: None,
            },
            usage: anthropic_resp.usage.map(|u| TokenUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            }),
        })
    }

    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>> {
        let (system, messages) = to_anthropic_messages(&request.messages);
        let max_tokens = request.max_tokens.unwrap_or(4096);
        validate_thinking_budget(&self.thinking, max_tokens)?;
        validate_thinking_tool_use(&self.thinking, request.tools.as_ref())?;

        let anthropic_req = AnthropicRequest {
            model: self.model.clone(),
            max_tokens,
            messages,
            system,
            tools: request.tools.as_ref().map(|t| to_anthropic_tools(t)),
            temperature: thinking_compatible_temperature(&self.thinking, request.temperature),
            thinking: thinking_payload(&self.thinking),
            stream: true,
        };

        let url = format!("{}/v1/messages", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_req)
            .send()
            .await
            .context("Failed to send streaming request to Anthropic")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("Anthropic returned {}: {}", status, body);
            anyhow::bail!(
                "Anthropic returned error (status {}): {}",
                status,
                summarize_response_body(&body)
            );
        }

        let byte_stream = resp.bytes_stream();

        let stream = futures::stream::unfold(
            (byte_stream, String::new(), None),
            |(mut byte_stream, mut buffer, mut prompt_tokens)| async move {
                use futures::TryStreamExt;
                loop {
                    if let Some(newline_pos) = buffer.find('\n') {
                        let line = buffer[..newline_pos].trim().to_string();
                        buffer = buffer[newline_pos + 1..].to_string();

                        if line.is_empty() {
                            continue;
                        }

                        if let Some(json_str) = line.strip_prefix("data: ") {
                            match serde_json::from_str::<AnthropicStreamEvent>(json_str) {
                                Ok(event) => match event.event_type.as_str() {
                                    "message_start" => {
                                        if let Some(input_tokens) = event
                                            .message
                                            .as_ref()
                                            .and_then(|message| message.usage.as_ref())
                                            .and_then(|usage| usage.input_tokens)
                                        {
                                            prompt_tokens = Some(input_tokens);
                                        }
                                    }
                                    "content_block_start" => {
                                        if let Some(chunk) = event
                                            .content_block
                                            .as_ref()
                                            .and_then(|block| anthropic_tool_start_chunk(&event, block))
                                        {
                                            return Some((Ok(chunk), (byte_stream, buffer, prompt_tokens)));
                                        }
                                    }
                                    "content_block_delta" => {
                                        if let Some(ref delta) = event.delta {
                                            if let Some(chunk) = anthropic_content_delta_chunk(event.index, delta) {
                                                return Some((Ok(chunk), (byte_stream, buffer, prompt_tokens)));
                                            }
                                        }
                                    }
                                    "message_delta" => {
                                        if let Some(usage) = event.usage.as_ref() {
                                            if let Some(input_tokens) = usage.input_tokens {
                                                prompt_tokens = Some(input_tokens);
                                            }
                                            if let Some(chunk) = anthropic_usage_chunk(prompt_tokens, usage) {
                                                return Some((Ok(chunk), (byte_stream, buffer, prompt_tokens)));
                                            }
                                        }
                                    }
                                    _ => continue,
                                },
                                Err(_) => continue,
                            }
                        }
                    }

                    match byte_stream.try_next().await {
                        Ok(Some(bytes)) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));
                        }
                        Ok(None) => return None,
                        Err(e) => return Some((
                            Err(anyhow::anyhow!("Stream error: {}", e)),
                            (byte_stream, buffer, prompt_tokens),
                        )),
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/v1/models", self.base_url);

        let resp = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .context("Failed to list Anthropic models")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if status == StatusCode::NOT_FOUND {
                if let Some(openai_models_url) = sibling_openai_models_url(&self.base_url) {
                    return list_openai_compatible_models(&self.client, &self.api_key, &openai_models_url).await;
                }
            }
            anyhow::bail!(
                "Anthropic models endpoint returned error (status {}): {}",
                status,
                summarize_response_body(&body)
            );
        }

        let body = resp.text().await.context("Failed to read Anthropic models response")?;
        parse_models_response(&body, "Anthropic")
    }

    async fn test_connectivity(&self) -> Result<ConnectivityResult> {
        let start = std::time::Instant::now();
        let max_tokens = match self.thinking.mode {
            LlmProviderThinkingMode::Enabled => self
                .thinking
                .budget_tokens
                .and_then(|budget| budget.checked_add(ANTHROPIC_THINKING_TOKEN_RESERVE))
                .unwrap_or(4096),
            LlmProviderThinkingMode::Default | LlmProviderThinkingMode::Disabled => 5,
        };
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "Hi".to_string(),
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            temperature: Some(0.0),
            max_tokens: Some(max_tokens),
        };
        let (success, error) = match self.chat_completion(request).await {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        Ok(ConnectivityResult {
            success,
            latency_ms: start.elapsed().as_millis() as u64,
            model: self.model.clone(),
            error,
        })
    }
}
