use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use super::provider::{ConnectivityResult, LlmProvider};
use super::types::*;

pub struct AnthropicProvider {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
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
    stream: bool,
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
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
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
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<AnthropicStreamDelta>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
struct AnthropicStreamDelta {
    #[serde(default)]
    text: Option<String>,
    #[serde(default, rename = "type")]
    delta_type: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input_json: Option<String>,
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

    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse> {
        let (system, messages) = to_anthropic_messages(&request.messages);

        let anthropic_req = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens.unwrap_or(4096),
            messages,
            system,
            tools: request.tools.as_ref().map(|t| to_anthropic_tools(t)),
            temperature: request.temperature,
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
            anyhow::bail!("Anthropic returned error (status {})", status);
        }

        let anthropic_resp: AnthropicResponse = resp
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

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
                _ => {}
            }
        }

        Ok(ChatResponse {
            message: ChatMessage {
                role: Role::Assistant,
                content,
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
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

        let anthropic_req = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens.unwrap_or(4096),
            messages,
            system,
            tools: request.tools.as_ref().map(|t| to_anthropic_tools(t)),
            temperature: request.temperature,
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
            anyhow::bail!("Anthropic returned error (status {})", status);
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

                        if let Some(json_str) = line.strip_prefix("data: ") {
                            match serde_json::from_str::<AnthropicStreamEvent>(json_str) {
                                Ok(event) => match event.event_type.as_str() {
                                    "content_block_delta" => {
                                        if let Some(ref delta) = event.delta {
                                            let chunk = ChatChunk {
                                                delta: ChatDelta {
                                                    role: None,
                                                    content: delta.text.clone(),
                                                    tool_calls: None,
                                                },
                                                usage: None,
                                            };
                                            return Some((Ok(chunk), (byte_stream, buffer)));
                                        }
                                    }
                                    "message_delta" => {
                                        if let Some(usage) = event.usage {
                                            let chunk = ChatChunk {
                                                delta: ChatDelta {
                                                    role: None,
                                                    content: None,
                                                    tool_calls: None,
                                                },
                                                usage: Some(TokenUsage {
                                                    prompt_tokens: 0,
                                                    completion_tokens: usage.output_tokens,
                                                    total_tokens: usage.output_tokens,
                                                }),
                                            };
                                            return Some((Ok(chunk), (byte_stream, buffer)));
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
                        Err(e) => {
                            return Some((
                                Err(anyhow::anyhow!("Stream error: {}", e)),
                                (byte_stream, buffer),
                            ))
                        }
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    async fn test_connectivity(&self) -> Result<ConnectivityResult> {
        let start = std::time::Instant::now();
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: Role::User,
                content: "Hi".to_string(),
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            temperature: Some(0.0),
            max_tokens: Some(5),
        };
        match self.chat_completion(request).await {
            Ok(_) => Ok(ConnectivityResult {
                success: true,
                latency_ms: start.elapsed().as_millis() as u64,
                model: self.model.clone(),
                error: None,
            }),
            Err(e) => Ok(ConnectivityResult {
                success: false,
                latency_ms: start.elapsed().as_millis() as u64,
                model: self.model.clone(),
                error: Some(e.to_string()),
            }),
        }
    }
}
