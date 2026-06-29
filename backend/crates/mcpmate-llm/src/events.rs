use async_trait::async_trait;

use crate::error::LlmResult;

#[derive(Debug, Clone)]
pub enum LlmProviderEvent {
    ProviderCreated { provider_id: String },
    ProviderUpdated { provider_id: String },
    ProviderDeleted { provider_id: String },
    DefaultProviderSet { provider_id: String },
    ProviderTested { provider_id: String, success: bool },
    ProviderModelsListed { provider_id: String, count: usize },
    ProviderConfigModelsListed { provider_id: Option<String>, count: usize },
}

#[async_trait]
pub trait LlmProviderEventSink: Send + Sync {
    async fn emit(
        &self,
        event: LlmProviderEvent,
    ) -> LlmResult<()>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopLlmProviderEventSink;

#[async_trait]
impl LlmProviderEventSink for NoopLlmProviderEventSink {
    async fn emit(
        &self,
        _event: LlmProviderEvent,
    ) -> LlmResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TracingLlmProviderEventSink;

#[async_trait]
impl LlmProviderEventSink for TracingLlmProviderEventSink {
    async fn emit(
        &self,
        event: LlmProviderEvent,
    ) -> LlmResult<()> {
        tracing::debug!(?event, "LLM provider event");
        Ok(())
    }
}
