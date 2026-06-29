use async_trait::async_trait;

use crate::error::LlmResult;

#[derive(Debug, Clone)]
pub struct StoredLlmProvider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub model_id: String,
    pub secret_alias: Option<String>,
    pub default_params_json: Option<String>,
    pub is_default: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateLlmProviderRecord {
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub model_id: String,
    pub secret_alias: Option<String>,
    pub default_params_json: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateLlmProviderRecord {
    pub name: Option<String>,
    pub provider_type: Option<String>,
    pub base_url: Option<String>,
    pub model_id: Option<String>,
    pub secret_alias: Option<Option<String>>,
    pub default_params_json: Option<Option<String>>,
}

#[async_trait]
pub trait LlmProviderRepository: Send + Sync {
    async fn list_providers(&self) -> LlmResult<Vec<StoredLlmProvider>>;

    async fn get_provider(
        &self,
        id: &str,
    ) -> LlmResult<Option<StoredLlmProvider>>;

    async fn create_provider(
        &self,
        record: CreateLlmProviderRecord,
    ) -> LlmResult<StoredLlmProvider>;

    async fn update_provider(
        &self,
        id: &str,
        record: UpdateLlmProviderRecord,
    ) -> LlmResult<Option<StoredLlmProvider>>;

    async fn delete_provider(
        &self,
        id: &str,
    ) -> LlmResult<bool>;

    async fn set_default_provider(
        &self,
        id: &str,
    ) -> LlmResult<()>;

    async fn get_default_provider(&self) -> LlmResult<Option<StoredLlmProvider>>;
}
