use std::sync::Arc;

use async_trait::async_trait;
use axum::Json;
use axum::extract::State;
use mcpmate_llm::{
    CreateLlmProviderInput, LlmCredentialStore, LlmError, LlmErrorKind, LlmProviderDefaultParams,
    LlmProviderDefaultParamsInput, LlmProviderManager, LlmProviderModelPreviewInput, LlmProviderThinkingInput,
    LlmProviderThinkingMode, StoredLlmProvider, TracingLlmProviderEventSink, UpdateLlmProviderInput,
};
use mcpmate_secrets::{SecretReference, SecretResolver, SecretStoreDeleteError};

use crate::api::handlers::ApiError;
use crate::api::models::llm::*;
use crate::api::routes::AppState;
use crate::config::llm::repository::SqliteLlmProviderRepository;
use crate::core::secrets::store::{
    LocalSecretStore, SecretCreateInput, SecretKindInput, SecretOriginInput, SecretUsageLocationInput,
    SecretUsageUpsertInput,
};

pub(crate) type BackendLlmProviderManager =
    LlmProviderManager<SqliteLlmProviderRepository, SecureStoreLlmCredentialStore, TracingLlmProviderEventSink>;

pub(crate) fn llm_manager(state: Arc<AppState>) -> Result<BackendLlmProviderManager, ApiError> {
    let pool = state
        .database
        .as_ref()
        .map(|db| db.pool.clone())
        .ok_or_else(|| ApiError::ServiceUnavailable("Database not available".into()))?;

    Ok(LlmProviderManager::new(
        SqliteLlmProviderRepository::new(pool),
        SecureStoreLlmCredentialStore::new(state),
        TracingLlmProviderEventSink,
    ))
}

pub(crate) fn map_llm_error(err: LlmError) -> ApiError {
    let kind = err.kind();
    let message = err.to_string();
    match kind {
        LlmErrorKind::BadRequest => ApiError::BadRequest(message),
        LlmErrorKind::NotFound => ApiError::NotFound(message),
        LlmErrorKind::ServiceUnavailable => ApiError::ServiceUnavailable(message),
        LlmErrorKind::Internal => ApiError::InternalError(message),
    }
}

fn to_provider_data(provider: &StoredLlmProvider) -> Result<LlmProviderData, ApiError> {
    let params = LlmProviderDefaultParams::from_json(&provider.default_params_json)
        .map_err(|err| ApiError::InternalError(err.to_string()))?;
    let thinking_mode = match params.thinking.mode {
        LlmProviderThinkingMode::Default => "default",
        LlmProviderThinkingMode::Disabled => "disabled",
        LlmProviderThinkingMode::Enabled => "enabled",
    };

    Ok(LlmProviderData {
        id: provider.id.clone(),
        name: provider.name.clone(),
        provider_type: provider.provider_type.clone(),
        base_url: provider.base_url.clone(),
        model_id: provider.model_id.clone(),
        has_api_key: provider.secret_alias.is_some(),
        is_default: provider.is_default,
        default_params: LlmDefaultParamsData {
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            thinking: LlmThinkingData {
                mode: thinking_mode.to_string(),
                budget_tokens: params.thinking.budget_tokens,
            },
        },
        created_at: provider.created_at.clone(),
        updated_at: provider.updated_at.clone(),
    })
}

fn default_params_input(input: LlmDefaultParamsReq) -> LlmProviderDefaultParamsInput {
    LlmProviderDefaultParamsInput {
        temperature: input.temperature,
        max_tokens: input.max_tokens,
        thinking: input.thinking.map(|thinking| LlmProviderThinkingInput {
            mode: thinking.mode,
            budget_tokens: thinking.budget_tokens,
        }),
    }
}

pub async fn list_providers(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, ApiError> {
    let providers = llm_manager(state)?.list_providers().await.map_err(map_llm_error)?;

    let data = providers.iter().map(to_provider_data).collect::<Result<Vec<_>, _>>()?;
    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderCreateReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let provider = llm_manager(state)?
        .create_provider(CreateLlmProviderInput {
            name: payload.name,
            provider_type: payload.provider_type,
            base_url: payload.base_url,
            model_id: payload.model_id,
            api_key: payload.api_key,
            default_params: payload.default_params.map(default_params_input),
        })
        .await
        .map_err(map_llm_error)?;

    Ok(Json(
        serde_json::json!({ "success": true, "data": to_provider_data(&provider)? }),
    ))
}

pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderUpdateReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let provider = llm_manager(state)?
        .update_provider(UpdateLlmProviderInput {
            id: payload.id,
            name: payload.name,
            provider_type: payload.provider_type,
            base_url: payload.base_url,
            model_id: payload.model_id,
            api_key: payload.api_key,
            default_params: payload.default_params.map(default_params_input),
        })
        .await
        .map_err(map_llm_error)?;

    Ok(Json(
        serde_json::json!({ "success": true, "data": to_provider_data(&provider)? }),
    ))
}

pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderIdReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    llm_manager(state)?
        .delete_provider(&payload.id)
        .await
        .map_err(map_llm_error)?;

    Ok(Json(serde_json::json!({ "success": true, "data": null })))
}

pub async fn test_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderIdReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = llm_manager(state)?
        .test_provider(&payload.id)
        .await
        .map_err(map_llm_error)?;
    let data = LlmConnectivityResult {
        success: result.success,
        latency_ms: result.latency_ms,
        model: result.model,
        error: result.error,
    };

    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

pub async fn list_models(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderIdReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let models = llm_manager(state)?
        .list_models(&payload.id)
        .await
        .map_err(map_llm_error)?;

    Ok(Json(
        serde_json::json!({ "success": true, "data": { "models": models } }),
    ))
}

pub async fn list_models_for_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderModelPreviewReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let models = llm_manager(state)?
        .list_models_for_config(LlmProviderModelPreviewInput {
            provider_id: payload.provider_id,
            provider_type: payload.provider_type,
            base_url: payload.base_url,
            model_id: payload.model_id,
            api_key: payload.api_key,
        })
        .await
        .map_err(map_llm_error)?;

    Ok(Json(
        serde_json::json!({ "success": true, "data": { "models": models } }),
    ))
}

pub async fn set_default_provider(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LlmProviderIdReq>,
) -> Result<Json<serde_json::Value>, ApiError> {
    llm_manager(state)?
        .set_default_provider(&payload.id)
        .await
        .map_err(map_llm_error)?;

    Ok(Json(serde_json::json!({ "success": true, "data": null })))
}

pub async fn get_default_provider(State(state): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, ApiError> {
    let provider = llm_manager(state)?
        .get_default_provider()
        .await
        .map_err(map_llm_error)?;

    let data = provider.as_ref().map(to_provider_data).transpose()?;
    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

#[derive(Clone)]
pub(crate) struct SecureStoreLlmCredentialStore {
    state: Arc<AppState>,
}

impl SecureStoreLlmCredentialStore {
    fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    async fn secret_store(&self) -> mcpmate_llm::LlmResult<Arc<LocalSecretStore>> {
        let store_guard = self.state.secret_store.read().await;
        store_guard
            .as_ref()
            .cloned()
            .ok_or_else(|| LlmError::service_unavailable("Secret store not available"))
    }

    async fn optional_secret_store(&self) -> Option<Arc<LocalSecretStore>> {
        let store_guard = self.state.secret_store.read().await;
        store_guard.as_ref().cloned()
    }
}

#[async_trait]
impl LlmCredentialStore for SecureStoreLlmCredentialStore {
    async fn resolve_reference(
        &self,
        alias: &str,
    ) -> mcpmate_llm::LlmResult<String> {
        let store = self.secret_store().await?;
        let reference = SecretReference::new(alias).map_err(|err| LlmError::internal(err.to_string()))?;
        let secret_value = store
            .resolve_secret(&reference)
            .map_err(|err| LlmError::internal(err.to_string()))?;
        Ok(secret_value.expose().to_string())
    }

    async fn verify_reference(
        &self,
        alias: &str,
    ) -> mcpmate_llm::LlmResult<()> {
        let store = self.secret_store().await?;
        let reference = SecretReference::new(alias)
            .map_err(|err| LlmError::bad_request(format!("Invalid secret reference: {err}")))?;
        store
            .get_secret_metadata(reference.alias())
            .await
            .map_err(|err| LlmError::bad_request(format!("Secret '{}' was not found: {err}", reference.alias())))?;
        Ok(())
    }

    async fn create_owned_provider_key(
        &self,
        provider_name: &str,
        api_key_value: &str,
    ) -> mcpmate_llm::LlmResult<String> {
        let store = self.secret_store().await?;
        let alias = format!("llm_provider_{}", uuid::Uuid::new_v4().as_simple());
        store
            .create_secret(SecretCreateInput {
                alias: alias.clone(),
                kind: SecretKindInput::ApiKey,
                value: api_key_value.to_string(),
                label: Some(format!("LLM Provider: {}", provider_name)),
                origin: Some(SecretOriginInput {
                    source: Some("llm_provider".to_string()),
                    field_group: Some("provider".to_string()),
                    field_key: Some("api_key".to_string()),
                    ..SecretOriginInput::default()
                }),
            })
            .await
            .map_err(|err| LlmError::internal(err.to_string()))?;

        Ok(alias)
    }

    async fn replace_provider_usage(
        &self,
        provider_id: &str,
        secret_alias: Option<&str>,
    ) -> mcpmate_llm::LlmResult<()> {
        let store = self.secret_store().await?;
        let usages = secret_alias
            .map(|alias| {
                vec![SecretUsageUpsertInput {
                    alias: alias.to_string(),
                    server_id: provider_id.to_string(),
                    location: SecretUsageLocationInput::LlmProviderApiKey,
                }]
            })
            .unwrap_or_default();

        store
            .replace_server_usages(provider_id, usages)
            .await
            .map_err(|err| LlmError::internal(err.to_string()))
    }

    async fn delete_owned_reference_if_unused(
        &self,
        alias: &str,
    ) -> mcpmate_llm::LlmResult<()> {
        if !alias.starts_with("llm_provider_") {
            return Ok(());
        }

        if let Some(store) = self.optional_secret_store().await {
            match store.delete_secret(alias, false).await {
                Ok(()) => {}
                Err(err) if is_secret_in_use_delete_error(&err) => {}
                Err(err) => return Err(LlmError::internal(err.to_string())),
            }
        }
        Ok(())
    }
}

fn is_secret_in_use_delete_error(err: &SecretStoreDeleteError) -> bool {
    matches!(
        err,
        SecretStoreDeleteError::InUse { .. } | SecretStoreDeleteError::UnsupportedUsage { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_secure_store_in_use_delete_errors_by_type() {
        let err = SecretStoreDeleteError::InUse {
            alias: "llm_provider_x".to_string(),
            usage_count: 1,
        };

        assert!(is_secret_in_use_delete_error(&err));
    }
}
