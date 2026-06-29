use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::types::{ChatChunk, ChatRequest, ChatResponse};

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn provider_type(&self) -> &str;
    fn model_id(&self) -> &str;

    async fn chat_completion(
        &self,
        request: ChatRequest,
    ) -> Result<ChatResponse>;

    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>>;

    async fn list_models(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }

    async fn test_connectivity(&self) -> Result<ConnectivityResult> {
        let start = std::time::Instant::now();
        let request = ChatRequest {
            messages: vec![crate::types::ChatMessage {
                role: crate::types::Role::User,
                content: "Hi".to_string(),
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: None,
            temperature: Some(0.0),
            max_tokens: Some(5),
        };
        let (success, error) = match self.chat_completion(request).await {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        Ok(ConnectivityResult {
            success,
            latency_ms: start.elapsed().as_millis() as u64,
            model: self.model_id().to_string(),
            error,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectivityResult {
    pub success: bool,
    pub latency_ms: u64,
    pub model: String,
    pub error: Option<String>,
}
