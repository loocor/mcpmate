use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::common::constants::database::{columns, tables};
use crate::config::llm::models::LlmProviderConfig;
use crate::generate_id;

pub async fn get_all_providers(pool: &Pool<Sqlite>) -> Result<Vec<LlmProviderConfig>> {
    let providers = sqlx::query_as::<_, LlmProviderConfig>(&format!(
        "SELECT * FROM {} ORDER BY {} ASC",
        tables::LLM_PROVIDER,
        columns::NAME
    ))
    .fetch_all(pool)
    .await
    .context("Failed to fetch LLM providers")?;

    Ok(providers)
}

pub async fn get_provider_by_id(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<Option<LlmProviderConfig>> {
    let provider = sqlx::query_as::<_, LlmProviderConfig>(&format!(
        "SELECT * FROM {} WHERE {} = ?",
        tables::LLM_PROVIDER,
        columns::ID
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch LLM provider by ID")?;

    Ok(provider)
}

pub async fn create_provider(
    pool: &Pool<Sqlite>,
    name: &str,
    provider_type: &str,
    base_url: &str,
    model_id: &str,
    secret_alias: Option<&str>,
    default_params_json: Option<&str>,
) -> Result<LlmProviderConfig> {
    let id = generate_id!("llmprov");

    tracing::debug!("Creating LLM provider '{}', type: {}", name, provider_type);

    sqlx::query(&format!(
        "INSERT INTO {} ({}, {}, {}, {}, {}, {}, {}, {}, {}) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        tables::LLM_PROVIDER,
        columns::ID,
        columns::NAME,
        columns::PROVIDER_TYPE,
        columns::BASE_URL,
        columns::MODEL_ID,
        columns::SECRET_ALIAS,
        columns::DEFAULT_PARAMS_JSON,
        columns::CREATED_AT,
        columns::UPDATED_AT
    ))
    .bind(&id)
    .bind(name)
    .bind(provider_type)
    .bind(base_url)
    .bind(model_id)
    .bind(secret_alias)
    .bind(default_params_json)
    .execute(pool)
    .await
    .context("Failed to create LLM provider")?;

    get_provider_by_id(pool, &id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created LLM provider"))
}

pub async fn update_provider(
    pool: &Pool<Sqlite>,
    id: &str,
    name: Option<&str>,
    provider_type: Option<&str>,
    base_url: Option<&str>,
    model_id: Option<&str>,
    secret_alias: Option<Option<&str>>,
    default_params_json: Option<Option<&str>>,
) -> Result<Option<LlmProviderConfig>> {
    let existing = get_provider_by_id(pool, id).await?;
    if existing.is_none() {
        return Ok(None);
    }

    let current = existing.unwrap();

    sqlx::query(&format!(
        "UPDATE {} SET {} = ?, {} = ?, {} = ?, {} = ?, {} = ?, {} = ?, {} = CURRENT_TIMESTAMP WHERE {} = ?",
        tables::LLM_PROVIDER,
        columns::NAME,
        columns::PROVIDER_TYPE,
        columns::BASE_URL,
        columns::MODEL_ID,
        columns::SECRET_ALIAS,
        columns::DEFAULT_PARAMS_JSON,
        columns::UPDATED_AT,
        columns::ID
    ))
    .bind(name.unwrap_or(&current.name))
    .bind(provider_type.unwrap_or(&current.provider_type))
    .bind(base_url.unwrap_or(&current.base_url))
    .bind(model_id.unwrap_or(&current.model_id))
    .bind(secret_alias.unwrap_or(current.secret_alias.as_deref()))
    .bind(default_params_json.unwrap_or(current.default_params_json.as_deref()))
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to update LLM provider")?;

    get_provider_by_id(pool, id).await
}

pub async fn delete_provider(pool: &Pool<Sqlite>, id: &str) -> Result<bool> {
    let result = sqlx::query(&format!(
        "DELETE FROM {} WHERE {} = ?",
        tables::LLM_PROVIDER,
        columns::ID
    ))
    .bind(id)
    .execute(pool)
    .await
    .context("Failed to delete LLM provider")?;

    Ok(result.rows_affected() > 0)
}

pub async fn set_default_provider(pool: &Pool<Sqlite>, id: &str) -> Result<()> {
    // Clear all defaults first
    sqlx::query("UPDATE llm_provider SET is_default = 0")
        .execute(pool)
        .await
        .context("Failed to clear default providers")?;

    // Set the new default
    sqlx::query("UPDATE llm_provider SET is_default = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("Failed to set default provider")?;

    Ok(())
}

pub async fn get_default_provider(pool: &Pool<Sqlite>) -> Result<Option<LlmProviderConfig>> {
    let provider = sqlx::query_as::<_, LlmProviderConfig>("SELECT * FROM llm_provider WHERE is_default = 1 LIMIT 1")
        .fetch_optional(pool)
        .await
        .context("Failed to fetch default provider")?;

    Ok(provider)
}
