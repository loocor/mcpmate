use async_trait::async_trait;

use crate::error::LlmResult;

#[derive(Debug, Clone)]
pub struct PreparedLlmCredential {
    pub alias: String,
    pub created_owned: bool,
}

#[async_trait]
pub trait LlmCredentialStore: Send + Sync {
    async fn resolve_reference(
        &self,
        alias: &str,
    ) -> LlmResult<String>;

    async fn verify_reference(
        &self,
        alias: &str,
    ) -> LlmResult<()>;

    async fn create_owned_provider_key(
        &self,
        provider_name: &str,
        api_key_value: &str,
    ) -> LlmResult<String>;

    async fn replace_provider_usage(
        &self,
        provider_id: &str,
        secret_alias: Option<&str>,
    ) -> LlmResult<()>;

    async fn delete_owned_reference_if_unused(
        &self,
        alias: &str,
    ) -> LlmResult<()>;
}

pub fn extract_whole_secret_alias(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    trimmed
        .strip_prefix("[[secret:")
        .and_then(|inner| inner.strip_suffix("]]"))
        .filter(|alias| !alias.is_empty())
}

pub fn contains_secret_placeholder(value: &str) -> bool {
    value.contains("[[secret:")
}
