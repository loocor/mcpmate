use std::net::{IpAddr, Ipv6Addr};

use crate::{
    config::{
        ANTHROPIC_THINKING_TOKEN_RESERVE, LlmProviderDefaultParams, LlmProviderSpec, LlmProviderThinkingConfig,
        LlmProviderThinkingMode, LlmProviderType,
    },
    credentials::{LlmCredentialStore, PreparedLlmCredential, contains_secret_placeholder, extract_whole_secret_alias},
    error::{LlmError, LlmErrorKind, LlmResult},
    events::{LlmProviderEvent, LlmProviderEventSink},
    factory,
    provider::{ConnectivityResult, LlmProvider},
    repository::{CreateLlmProviderRecord, LlmProviderRepository, StoredLlmProvider, UpdateLlmProviderRecord},
    types::{ChatRequest, ChatResponse},
};

#[derive(Debug, Clone)]
pub struct LlmProviderManager<R, C, E> {
    repository: R,
    credentials: C,
    events: E,
}

#[derive(Debug, Clone)]
pub struct LlmChatCompletionResult {
    pub provider: StoredLlmProvider,
    pub response: ChatResponse,
}

impl<R, C, E> LlmProviderManager<R, C, E>
where
    R: LlmProviderRepository,
    C: LlmCredentialStore,
    E: LlmProviderEventSink,
{
    pub fn new(
        repository: R,
        credentials: C,
        events: E,
    ) -> Self {
        Self {
            repository,
            credentials,
            events,
        }
    }

    pub async fn list_providers(&self) -> LlmResult<Vec<StoredLlmProvider>> {
        self.repository.list_providers().await
    }

    pub async fn create_provider(
        &self,
        input: CreateLlmProviderInput,
    ) -> LlmResult<StoredLlmProvider> {
        supported_provider_type(&input.provider_type)?;
        validate_base_url(&input.base_url)?;

        let params_json = serialize_default_params(input.default_params, &input.provider_type, None)?;
        let prepared_credential = match input.api_key.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            Some(api_key) => Some(self.prepare_credential(&input.name, api_key).await?),
            None => None,
        };
        let secret_alias = prepared_credential.as_ref().map(|credential| credential.alias.clone());

        let provider = match self
            .repository
            .create_provider(CreateLlmProviderRecord {
                name: input.name,
                provider_type: input.provider_type,
                base_url: input.base_url,
                model_id: input.model_id,
                secret_alias: secret_alias.clone(),
                default_params_json: params_json,
            })
            .await
        {
            Ok(provider) => provider,
            Err(err) => {
                self.delete_created_owned_credential(prepared_credential.as_ref()).await;
                return Err(err);
            }
        };

        if let Some(alias) = secret_alias.as_deref() {
            if let Err(err) = self.credentials.replace_provider_usage(&provider.id, Some(alias)).await {
                self.rollback_created_provider(&provider.id, prepared_credential.as_ref())
                    .await;
                return Err(err);
            }
        }

        self.events
            .emit(LlmProviderEvent::ProviderCreated {
                provider_id: provider.id.clone(),
            })
            .await?;

        Ok(provider)
    }

    pub async fn update_provider(
        &self,
        input: UpdateLlmProviderInput,
    ) -> LlmResult<StoredLlmProvider> {
        if let Some(provider_type) = input.provider_type.as_deref() {
            supported_provider_type(provider_type)?;
        }
        if let Some(base_url) = input.base_url.as_deref() {
            validate_base_url(base_url)?;
        }

        let existing = self.get_provider_or_not_found(&input.id).await?;

        let mut next_secret_alias = existing.secret_alias.clone();
        let mut old_alias_to_delete: Option<String> = None;
        let mut created_next_credential: Option<PreparedLlmCredential> = None;
        let mut should_sync_usage = false;

        if let Some(api_key_update) = input.api_key.as_ref() {
            match api_key_update.as_deref().map(str::trim) {
                Some("") | None => {
                    if existing.secret_alias.is_some() {
                        old_alias_to_delete = existing.secret_alias.clone();
                        next_secret_alias = None;
                        should_sync_usage = true;
                    }
                }
                Some(api_key) => {
                    let provider_name = input.name.as_deref().unwrap_or(&existing.name);
                    let credential = self.prepare_credential(provider_name, api_key).await?;
                    if existing.secret_alias.as_deref() != Some(credential.alias.as_str()) {
                        old_alias_to_delete = existing.secret_alias.clone();
                        next_secret_alias = Some(credential.alias.clone());
                        should_sync_usage = true;
                    }
                    if credential.created_owned {
                        created_next_credential = Some(credential);
                    }
                }
            }
        }

        let next_provider_type = input.provider_type.as_deref().unwrap_or(&existing.provider_type);
        supported_provider_type(next_provider_type)?;
        let params_json = if input.default_params.is_some() || input.provider_type.is_some() {
            let existing_params = LlmProviderDefaultParams::from_json(&existing.default_params_json)
                .map_err(|err| LlmError::from_anyhow(LlmErrorKind::Internal, err))?;
            serialize_default_params(input.default_params, next_provider_type, Some(existing_params))?
        } else {
            None
        };

        let provider = match self
            .repository
            .update_provider(
                &input.id,
                UpdateLlmProviderRecord {
                    name: input.name,
                    provider_type: input.provider_type,
                    base_url: input.base_url,
                    model_id: input.model_id,
                    secret_alias: Some(next_secret_alias.clone()),
                    default_params_json: params_json.map(Some),
                },
            )
            .await
        {
            Ok(Some(provider)) => provider,
            Ok(None) => {
                self.delete_created_owned_credential(created_next_credential.as_ref())
                    .await;
                return Err(LlmError::not_found("Provider not found"));
            }
            Err(err) => {
                self.delete_created_owned_credential(created_next_credential.as_ref())
                    .await;
                return Err(err);
            }
        };

        if should_sync_usage {
            if let Err(err) = self
                .credentials
                .replace_provider_usage(&input.id, provider.secret_alias.as_deref())
                .await
            {
                self.rollback_updated_provider(&input.id, &existing, created_next_credential.as_ref())
                    .await;
                return Err(err);
            }
        }

        if let Some(old_alias) = old_alias_to_delete.filter(|alias| provider.secret_alias.as_deref() != Some(alias)) {
            self.delete_owned_reference_best_effort(&old_alias).await;
        }

        self.events
            .emit(LlmProviderEvent::ProviderUpdated {
                provider_id: provider.id.clone(),
            })
            .await?;

        Ok(provider)
    }

    pub async fn delete_provider(
        &self,
        id: &str,
    ) -> LlmResult<()> {
        let existing = self.get_provider_or_not_found(id).await?;

        if let Some(alias) = existing.secret_alias.as_deref() {
            self.credentials.replace_provider_usage(id, None).await?;
            self.credentials.delete_owned_reference_if_unused(alias).await?;
        }

        self.repository.delete_provider(id).await?;
        self.events
            .emit(LlmProviderEvent::ProviderDeleted {
                provider_id: id.to_string(),
            })
            .await?;
        Ok(())
    }

    pub async fn test_provider(
        &self,
        id: &str,
    ) -> LlmResult<ConnectivityResult> {
        let config = self.get_provider_or_not_found(id).await?;
        let provider = self.provider_for_stored_config(&config).await?;
        let result = provider
            .test_connectivity()
            .await
            .map_err(|err| LlmError::internal(err.to_string()))?;

        self.events
            .emit(LlmProviderEvent::ProviderTested {
                provider_id: id.to_string(),
                success: result.success,
            })
            .await?;
        Ok(result)
    }

    pub async fn list_models(
        &self,
        id: &str,
    ) -> LlmResult<Vec<String>> {
        let config = self.get_provider_or_not_found(id).await?;
        let provider = self.provider_for_stored_config(&config).await?;
        let models = provider
            .list_models()
            .await
            .map_err(|err| LlmError::internal(err.to_string()))?;

        self.events
            .emit(LlmProviderEvent::ProviderModelsListed {
                provider_id: id.to_string(),
                count: models.len(),
            })
            .await?;
        Ok(models)
    }

    pub async fn list_models_for_config(
        &self,
        input: LlmProviderModelPreviewInput,
    ) -> LlmResult<Vec<String>> {
        supported_provider_type(&input.provider_type)?;
        validate_base_url(&input.base_url)?;

        let api_key = match input.api_key.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            Some(value) => self.resolve_api_key_value(value).await?,
            None => self.resolve_saved_provider_preview_key(&input).await?,
        };

        let spec = LlmProviderSpec {
            provider_type: input.provider_type,
            base_url: input.base_url,
            model_id: input.model_id,
            default_params: LlmProviderDefaultParams::default(),
        };
        let provider = factory::create_provider(&spec, &api_key).map_err(|err| LlmError::internal(err.to_string()))?;
        let models = provider
            .list_models()
            .await
            .map_err(|err| LlmError::internal(err.to_string()))?;

        self.events
            .emit(LlmProviderEvent::ProviderConfigModelsListed {
                provider_id: input.provider_id,
                count: models.len(),
            })
            .await?;
        Ok(models)
    }

    pub async fn set_default_provider(
        &self,
        id: &str,
    ) -> LlmResult<()> {
        self.get_provider_or_not_found(id).await?;
        self.repository.set_default_provider(id).await?;
        self.events
            .emit(LlmProviderEvent::DefaultProviderSet {
                provider_id: id.to_string(),
            })
            .await?;
        Ok(())
    }

    pub async fn get_default_provider(&self) -> LlmResult<Option<StoredLlmProvider>> {
        self.repository.get_default_provider().await
    }

    pub async fn chat_completion(
        &self,
        provider_id: Option<&str>,
        request: ChatRequest,
    ) -> LlmResult<LlmChatCompletionResult> {
        let config =
            match provider_id {
                Some(id) => self.get_provider_or_not_found(id).await?,
                None => self.repository.get_default_provider().await?.ok_or_else(|| {
                    LlmError::bad_request("LLM evaluation requires provider_id or a default provider")
                })?,
            };
        let provider = self.provider_for_stored_config(&config).await?;
        let response = provider
            .chat_completion(request)
            .await
            .map_err(|err| LlmError::internal(err.to_string()))?;

        Ok(LlmChatCompletionResult {
            provider: config,
            response,
        })
    }

    async fn get_provider_or_not_found(
        &self,
        id: &str,
    ) -> LlmResult<StoredLlmProvider> {
        self.repository
            .get_provider(id)
            .await?
            .ok_or_else(|| LlmError::not_found("Provider not found"))
    }

    async fn provider_for_stored_config(
        &self,
        config: &StoredLlmProvider,
    ) -> LlmResult<Box<dyn LlmProvider>> {
        let api_key = self.resolve_provider_api_key(config).await?;
        let spec = provider_spec(config)?;
        factory::create_provider(&spec, &api_key).map_err(|err| LlmError::internal(err.to_string()))
    }

    async fn prepare_credential(
        &self,
        provider_name: &str,
        api_key_value: &str,
    ) -> LlmResult<PreparedLlmCredential> {
        if let Some(alias) = extract_whole_secret_alias(api_key_value) {
            self.credentials.verify_reference(alias).await?;
            return Ok(PreparedLlmCredential {
                alias: alias.to_string(),
                created_owned: false,
            });
        }

        if contains_secret_placeholder(api_key_value) {
            return Err(LlmError::bad_request(
                "Provider API key must be plaintext or a single Secure Store secret reference",
            ));
        }

        let alias = self
            .credentials
            .create_owned_provider_key(provider_name, api_key_value)
            .await?;
        Ok(PreparedLlmCredential {
            alias,
            created_owned: true,
        })
    }

    async fn resolve_api_key_value(
        &self,
        api_key_value: &str,
    ) -> LlmResult<String> {
        if let Some(alias) = extract_whole_secret_alias(api_key_value) {
            return self.credentials.resolve_reference(alias).await;
        }

        if contains_secret_placeholder(api_key_value) {
            return Err(LlmError::bad_request(
                "Provider API key must be plaintext or a single Secure Store secret reference",
            ));
        }

        Ok(api_key_value.to_string())
    }

    async fn resolve_saved_provider_preview_key(
        &self,
        input: &LlmProviderModelPreviewInput,
    ) -> LlmResult<String> {
        let Some(provider_id) = input.provider_id.as_deref() else {
            return Ok(String::new());
        };

        let provider = self.get_provider_or_not_found(provider_id).await?;
        if supported_provider_type(&provider.provider_type)? != supported_provider_type(&input.provider_type)?
            || normalized_base_url(&provider.base_url)? != normalized_base_url(&input.base_url)?
        {
            return Err(LlmError::bad_request(
                "Stored provider API keys can only be reused with the saved provider type and base URL",
            ));
        }

        match provider.secret_alias.as_deref() {
            Some(alias) => self.credentials.resolve_reference(alias).await,
            None => Ok(String::new()),
        }
    }

    async fn resolve_provider_api_key(
        &self,
        provider: &StoredLlmProvider,
    ) -> LlmResult<String> {
        match provider.secret_alias.as_deref() {
            Some(alias) => self.credentials.resolve_reference(alias).await,
            None => Ok(String::new()),
        }
    }

    async fn rollback_created_provider(
        &self,
        provider_id: &str,
        credential: Option<&PreparedLlmCredential>,
    ) {
        match self.repository.delete_provider(provider_id).await {
            Ok(true) => {
                self.delete_created_owned_credential(credential).await;
            }
            Ok(false) => {
                tracing::warn!(
                    "Created LLM provider '{}' was not found during failed create rollback",
                    provider_id
                );
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to roll back LLM provider '{}' after secret usage sync failure: {}",
                    provider_id,
                    err
                );
            }
        }
    }

    async fn rollback_updated_provider(
        &self,
        provider_id: &str,
        existing: &StoredLlmProvider,
        created_credential: Option<&PreparedLlmCredential>,
    ) {
        let rollback = self
            .repository
            .update_provider(
                provider_id,
                UpdateLlmProviderRecord {
                    name: Some(existing.name.clone()),
                    provider_type: Some(existing.provider_type.clone()),
                    base_url: Some(existing.base_url.clone()),
                    model_id: Some(existing.model_id.clone()),
                    secret_alias: Some(existing.secret_alias.clone()),
                    default_params_json: Some(existing.default_params_json.clone()),
                },
            )
            .await;

        match rollback {
            Ok(Some(_)) => {
                self.delete_created_owned_credential(created_credential).await;
            }
            Ok(None) => {
                tracing::warn!(
                    "Updated LLM provider '{}' was not found during failed usage sync rollback",
                    provider_id
                );
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to roll back LLM provider '{}' after secret usage sync failure: {}",
                    provider_id,
                    err
                );
            }
        }
    }

    async fn delete_created_owned_credential(
        &self,
        credential: Option<&PreparedLlmCredential>,
    ) {
        let Some(credential) = credential.filter(|credential| credential.created_owned) else {
            return;
        };
        self.delete_owned_reference_best_effort(&credential.alias).await;
    }

    async fn delete_owned_reference_best_effort(
        &self,
        alias: &str,
    ) {
        if let Err(err) = self.credentials.delete_owned_reference_if_unused(alias).await {
            tracing::warn!("Failed to delete owned LLM provider secret '{}': {}", alias, err);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateLlmProviderInput {
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub model_id: String,
    pub api_key: Option<String>,
    pub default_params: Option<LlmProviderDefaultParamsInput>,
}

#[derive(Debug, Clone)]
pub struct UpdateLlmProviderInput {
    pub id: String,
    pub name: Option<String>,
    pub provider_type: Option<String>,
    pub base_url: Option<String>,
    pub model_id: Option<String>,
    pub api_key: Option<Option<String>>,
    pub default_params: Option<LlmProviderDefaultParamsInput>,
}

#[derive(Debug, Clone)]
pub struct LlmProviderDefaultParamsInput {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub thinking: Option<LlmProviderThinkingInput>,
}

#[derive(Debug, Clone)]
pub struct LlmProviderThinkingInput {
    pub mode: String,
    pub budget_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct LlmProviderModelPreviewInput {
    pub provider_id: Option<String>,
    pub provider_type: String,
    pub base_url: String,
    pub model_id: String,
    pub api_key: Option<String>,
}

fn serialize_default_params(
    input: Option<LlmProviderDefaultParamsInput>,
    provider_type: &str,
    existing: Option<LlmProviderDefaultParams>,
) -> LlmResult<Option<String>> {
    let Some(params) = build_default_params(input, provider_type, existing)? else {
        return Ok(None);
    };
    serialize_params(params)
}

fn build_default_params(
    input: Option<LlmProviderDefaultParamsInput>,
    provider_type: &str,
    existing: Option<LlmProviderDefaultParams>,
) -> LlmResult<Option<LlmProviderDefaultParams>> {
    let Some(input) = input else {
        return Ok(existing.map(|existing| preserve_supported_thinking(provider_type, existing)));
    };
    let existing = existing.unwrap_or_default();
    let max_tokens = input.max_tokens.unwrap_or(existing.max_tokens);

    Ok(Some(LlmProviderDefaultParams {
        temperature: input.temperature.unwrap_or(existing.temperature),
        max_tokens,
        thinking: match input.thinking {
            Some(thinking) => thinking_config_from_input(thinking, provider_type, max_tokens)?,
            None if provider_type == "anthropic" => existing.thinking,
            None => LlmProviderThinkingConfig::default(),
        },
    }))
}

fn preserve_supported_thinking(
    provider_type: &str,
    existing: LlmProviderDefaultParams,
) -> LlmProviderDefaultParams {
    let thinking = if provider_type == "anthropic" {
        existing.thinking
    } else {
        LlmProviderThinkingConfig::default()
    };

    LlmProviderDefaultParams { thinking, ..existing }
}

fn serialize_params(params: LlmProviderDefaultParams) -> LlmResult<Option<String>> {
    serde_json::to_string(&params)
        .map(Some)
        .map_err(|err| LlmError::internal(err.to_string()))
}

fn thinking_config_from_input(
    input: LlmProviderThinkingInput,
    provider_type: &str,
    max_tokens: u32,
) -> LlmResult<LlmProviderThinkingConfig> {
    let mode = match input.mode.as_str() {
        "default" => LlmProviderThinkingMode::Default,
        "disabled" => LlmProviderThinkingMode::Disabled,
        "enabled" => LlmProviderThinkingMode::Enabled,
        _ => return Err(LlmError::bad_request("Invalid thinking mode")),
    };

    if provider_type != "anthropic" && mode != LlmProviderThinkingMode::Default {
        return Err(LlmError::bad_request(
            "Thinking is only supported for Anthropic providers",
        ));
    }

    let budget_tokens = match mode {
        LlmProviderThinkingMode::Default | LlmProviderThinkingMode::Disabled => None,
        LlmProviderThinkingMode::Enabled => {
            let budget_tokens = input
                .budget_tokens
                .ok_or_else(|| LlmError::bad_request("Thinking budget tokens are required when thinking is enabled"))?;
            if budget_tokens == 0 {
                return Err(LlmError::bad_request(
                    "Thinking budget tokens must be greater than zero",
                ));
            }
            if budget_tokens.saturating_add(ANTHROPIC_THINKING_TOKEN_RESERVE) > max_tokens {
                return Err(LlmError::bad_request(
                    "Thinking budget tokens must leave at least 1000 tokens for output",
                ));
            }
            Some(budget_tokens)
        }
    };

    Ok(LlmProviderThinkingConfig { mode, budget_tokens })
}

fn provider_spec(config: &StoredLlmProvider) -> LlmResult<LlmProviderSpec> {
    Ok(LlmProviderSpec {
        provider_type: config.provider_type.clone(),
        base_url: config.base_url.clone(),
        model_id: config.model_id.clone(),
        default_params: LlmProviderDefaultParams::from_json(&config.default_params_json)
            .map_err(|err| LlmError::from_anyhow(LlmErrorKind::Internal, err))?,
    })
}

fn supported_provider_type(provider_type: &str) -> LlmResult<LlmProviderType> {
    let parsed: LlmProviderType = provider_type
        .parse()
        .map_err(|_| LlmError::bad_request("Invalid provider type"))?;

    if parsed == LlmProviderType::OpenAiResponses {
        return Err(LlmError::bad_request(
            "OpenAI Responses providers are not supported yet",
        ));
    }

    Ok(parsed)
}

fn validate_base_url(url: &str) -> LlmResult<()> {
    let parsed = url::Url::parse(url).map_err(|_| LlmError::bad_request("Invalid base URL"))?;

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(LlmError::bad_request("Base URL must not contain credentials"));
    }

    let host = parsed.host_str().unwrap_or("");
    let is_localhost = matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]");

    match parsed.scheme() {
        "https" => {}
        "http" if is_localhost => {}
        "http" => return Err(LlmError::bad_request("HTTP is only allowed for localhost")),
        _ => return Err(LlmError::bad_request("Base URL must use http or https scheme")),
    }

    if !is_localhost {
        if let Ok(ip) = host.parse::<IpAddr>() {
            if is_private_ip(ip) {
                return Err(LlmError::bad_request("Private/link-local IP addresses are not allowed"));
            }
        }
    }

    Ok(())
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_broadcast() || v4.is_documentation()
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unicast_link_local() || is_ipv6_unique_local(v6),
    }
}

fn is_ipv6_unique_local(ip: Ipv6Addr) -> bool {
    (ip.octets()[0] & 0xfe) == 0xfc
}

fn normalized_base_url(url: &str) -> LlmResult<String> {
    let mut parsed = url::Url::parse(url).map_err(|_| LlmError::bad_request("Invalid base URL"))?;
    parsed.set_fragment(None);
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use futures::executor::block_on;

    use super::*;
    use crate::credentials::LlmCredentialStore;
    use crate::events::NoopLlmProviderEventSink;
    use crate::repository::{
        CreateLlmProviderRecord, LlmProviderRepository, StoredLlmProvider, UpdateLlmProviderRecord,
    };

    #[derive(Clone, Default)]
    struct FakeRepository {
        providers: Arc<Mutex<HashMap<String, StoredLlmProvider>>>,
        deleted: Arc<Mutex<Vec<String>>>,
    }

    impl FakeRepository {
        fn with_provider(provider: StoredLlmProvider) -> Self {
            let repository = Self::default();
            repository
                .providers
                .lock()
                .expect("providers lock")
                .insert(provider.id.clone(), provider);
            repository
        }
    }

    #[async_trait]
    impl LlmProviderRepository for FakeRepository {
        async fn list_providers(&self) -> LlmResult<Vec<StoredLlmProvider>> {
            Ok(self
                .providers
                .lock()
                .expect("providers lock")
                .values()
                .cloned()
                .collect())
        }

        async fn get_provider(
            &self,
            id: &str,
        ) -> LlmResult<Option<StoredLlmProvider>> {
            Ok(self.providers.lock().expect("providers lock").get(id).cloned())
        }

        async fn create_provider(
            &self,
            record: CreateLlmProviderRecord,
        ) -> LlmResult<StoredLlmProvider> {
            let provider = StoredLlmProvider {
                id: "provider-1".to_string(),
                name: record.name,
                provider_type: record.provider_type,
                base_url: record.base_url,
                model_id: record.model_id,
                secret_alias: record.secret_alias,
                default_params_json: record.default_params_json,
                is_default: false,
                created_at: None,
                updated_at: None,
            };
            self.providers
                .lock()
                .expect("providers lock")
                .insert(provider.id.clone(), provider.clone());
            Ok(provider)
        }

        async fn update_provider(
            &self,
            id: &str,
            record: UpdateLlmProviderRecord,
        ) -> LlmResult<Option<StoredLlmProvider>> {
            let mut providers = self.providers.lock().expect("providers lock");
            let Some(provider) = providers.get_mut(id) else {
                return Ok(None);
            };
            if let Some(name) = record.name {
                provider.name = name;
            }
            if let Some(provider_type) = record.provider_type {
                provider.provider_type = provider_type;
            }
            if let Some(base_url) = record.base_url {
                provider.base_url = base_url;
            }
            if let Some(model_id) = record.model_id {
                provider.model_id = model_id;
            }
            if let Some(secret_alias) = record.secret_alias {
                provider.secret_alias = secret_alias;
            }
            if let Some(default_params_json) = record.default_params_json {
                provider.default_params_json = default_params_json;
            }
            Ok(Some(provider.clone()))
        }

        async fn delete_provider(
            &self,
            id: &str,
        ) -> LlmResult<bool> {
            self.deleted.lock().expect("deleted lock").push(id.to_string());
            Ok(self.providers.lock().expect("providers lock").remove(id).is_some())
        }

        async fn set_default_provider(
            &self,
            _id: &str,
        ) -> LlmResult<()> {
            Ok(())
        }

        async fn get_default_provider(&self) -> LlmResult<Option<StoredLlmProvider>> {
            Ok(None)
        }
    }

    #[derive(Clone, Default)]
    struct FakeCredentialStore {
        replaced: Arc<Mutex<Vec<(String, Option<String>)>>>,
        fail_empty_replace: bool,
    }

    #[async_trait]
    impl LlmCredentialStore for FakeCredentialStore {
        async fn resolve_reference(
            &self,
            _alias: &str,
        ) -> LlmResult<String> {
            Ok("secret".to_string())
        }

        async fn verify_reference(
            &self,
            _alias: &str,
        ) -> LlmResult<()> {
            Ok(())
        }

        async fn create_owned_provider_key(
            &self,
            _provider_name: &str,
            _api_key_value: &str,
        ) -> LlmResult<String> {
            Ok("llm_provider_generated".to_string())
        }

        async fn replace_provider_usage(
            &self,
            provider_id: &str,
            secret_alias: Option<&str>,
        ) -> LlmResult<()> {
            if self.fail_empty_replace && secret_alias.is_none() {
                return Err(LlmError::service_unavailable("Secret store not available"));
            }
            self.replaced
                .lock()
                .expect("replaced lock")
                .push((provider_id.to_string(), secret_alias.map(str::to_string)));
            Ok(())
        }

        async fn delete_owned_reference_if_unused(
            &self,
            _alias: &str,
        ) -> LlmResult<()> {
            Ok(())
        }
    }

    fn stored_provider(
        id: &str,
        secret_alias: Option<&str>,
    ) -> StoredLlmProvider {
        StoredLlmProvider {
            id: id.to_string(),
            name: "Provider".to_string(),
            provider_type: "openai_chat".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model_id: "gpt-4o".to_string(),
            secret_alias: secret_alias.map(str::to_string),
            default_params_json: None,
            is_default: false,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn create_provider_without_key_does_not_require_usage_replacement() {
        let repository = FakeRepository::default();
        let credentials = FakeCredentialStore {
            fail_empty_replace: true,
            ..FakeCredentialStore::default()
        };
        let replaced = credentials.replaced.clone();
        let manager = LlmProviderManager::new(repository, credentials, NoopLlmProviderEventSink);

        let provider = block_on(manager.create_provider(CreateLlmProviderInput {
            name: "No Key".to_string(),
            provider_type: "openai_chat".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model_id: "gpt-4o".to_string(),
            api_key: None,
            default_params: None,
        }))
        .expect("create no-key provider");

        assert_eq!(provider.secret_alias, None);
        assert!(replaced.lock().expect("replaced lock").is_empty());
    }

    #[test]
    fn update_provider_without_key_change_does_not_replace_usage() {
        let repository = FakeRepository::with_provider(stored_provider("provider-1", None));
        let credentials = FakeCredentialStore {
            fail_empty_replace: true,
            ..FakeCredentialStore::default()
        };
        let replaced = credentials.replaced.clone();
        let manager = LlmProviderManager::new(repository, credentials, NoopLlmProviderEventSink);

        block_on(manager.update_provider(UpdateLlmProviderInput {
            id: "provider-1".to_string(),
            name: Some("Renamed".to_string()),
            provider_type: None,
            base_url: None,
            model_id: None,
            api_key: None,
            default_params: None,
        }))
        .expect("update no-key provider");

        assert!(replaced.lock().expect("replaced lock").is_empty());
    }
}
