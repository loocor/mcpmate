use async_trait::async_trait;
use mcpmate_llm::{
    CreateLlmProviderRecord, LlmError, LlmErrorKind, LlmProviderRepository, LlmResult, StoredLlmProvider,
    UpdateLlmProviderRecord,
};
use sqlx::{Pool, Sqlite};

use crate::config::llm::{crud, models::LlmProviderConfig};

#[derive(Debug, Clone)]
pub struct SqliteLlmProviderRepository {
    pool: Pool<Sqlite>,
}

impl SqliteLlmProviderRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LlmProviderRepository for SqliteLlmProviderRepository {
    async fn list_providers(&self) -> LlmResult<Vec<StoredLlmProvider>> {
        crud::get_all_providers(&self.pool)
            .await
            .map_err(internal_error)?
            .into_iter()
            .map(stored_provider_from_config)
            .collect()
    }

    async fn get_provider(
        &self,
        id: &str,
    ) -> LlmResult<Option<StoredLlmProvider>> {
        crud::get_provider_by_id(&self.pool, id)
            .await
            .map_err(internal_error)?
            .map(stored_provider_from_config)
            .transpose()
    }

    async fn create_provider(
        &self,
        record: CreateLlmProviderRecord,
    ) -> LlmResult<StoredLlmProvider> {
        crud::create_provider(
            &self.pool,
            &record.name,
            &record.provider_type,
            &record.base_url,
            &record.model_id,
            record.secret_alias.as_deref(),
            record.default_params_json.as_deref(),
        )
        .await
        .map_err(internal_error)
        .and_then(stored_provider_from_config)
    }

    async fn update_provider(
        &self,
        id: &str,
        record: UpdateLlmProviderRecord,
    ) -> LlmResult<Option<StoredLlmProvider>> {
        crud::update_provider(
            &self.pool,
            id,
            record.name.as_deref(),
            record.provider_type.as_deref(),
            record.base_url.as_deref(),
            record.model_id.as_deref(),
            record.secret_alias.as_ref().map(|alias| alias.as_deref()),
            record.default_params_json.as_ref().map(|params| params.as_deref()),
        )
        .await
        .map_err(internal_error)?
        .map(stored_provider_from_config)
        .transpose()
    }

    async fn delete_provider(
        &self,
        id: &str,
    ) -> LlmResult<bool> {
        crud::delete_provider(&self.pool, id).await.map_err(internal_error)
    }

    async fn set_default_provider(
        &self,
        id: &str,
    ) -> LlmResult<()> {
        crud::set_default_provider(&self.pool, id).await.map_err(internal_error)
    }

    async fn get_default_provider(&self) -> LlmResult<Option<StoredLlmProvider>> {
        crud::get_default_provider(&self.pool)
            .await
            .map_err(internal_error)?
            .map(stored_provider_from_config)
            .transpose()
    }
}

fn stored_provider_from_config(config: LlmProviderConfig) -> LlmResult<StoredLlmProvider> {
    let id = config
        .id
        .ok_or_else(|| LlmError::internal("Stored provider is missing an id"))?;
    Ok(StoredLlmProvider {
        id,
        name: config.name,
        provider_type: config.provider_type,
        base_url: config.base_url,
        model_id: config.model_id,
        secret_alias: config.secret_alias,
        default_params_json: config.default_params_json,
        is_default: config.is_default,
        created_at: config.created_at.map(|t| t.to_rfc3339()),
        updated_at: config.updated_at.map(|t| t.to_rfc3339()),
    })
}

fn internal_error(err: anyhow::Error) -> LlmError {
    LlmError::from_anyhow(LlmErrorKind::Internal, err)
}
