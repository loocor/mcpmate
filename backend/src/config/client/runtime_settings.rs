use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;

use crate::clients::error::{ConfigError, ConfigResult};
use crate::clients::models::MergeStrategy;

use super::init::CLIENT_RUNTIME_SETTINGS_TABLE;

const CLIENT_DEFAULTS_SETTING_KEY: &str = "client_defaults";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRuntimeDefaults {
    #[serde(default)]
    pub default_merge_strategy_override: Option<MergeStrategy>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

pub async fn get_client_runtime_defaults(pool: &SqlitePool) -> ConfigResult<ClientRuntimeDefaults> {
    let value = sqlx::query_scalar::<_, String>(&format!(
        "SELECT value FROM {CLIENT_RUNTIME_SETTINGS_TABLE} WHERE key = ?"
    ))
    .bind(CLIENT_DEFAULTS_SETTING_KEY)
    .fetch_optional(pool)
    .await
    .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;

    parse_client_runtime_defaults(value)
}

pub async fn set_default_merge_strategy_override(
    pool: &SqlitePool,
    strategy: Option<MergeStrategy>,
) -> ConfigResult<ClientRuntimeDefaults> {
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    let value = sqlx::query_scalar::<_, String>(&format!(
        "SELECT value FROM {CLIENT_RUNTIME_SETTINGS_TABLE} WHERE key = ?"
    ))
    .bind(CLIENT_DEFAULTS_SETTING_KEY)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    let mut defaults = parse_client_runtime_defaults(value)?;
    defaults.default_merge_strategy_override = strategy;
    let serialized = serde_json::to_string(&defaults)?;

    sqlx::query(&format!(
        "INSERT INTO {CLIENT_RUNTIME_SETTINGS_TABLE} (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value"
    ))
    .bind(CLIENT_DEFAULTS_SETTING_KEY)
    .bind(serialized)
    .execute(&mut *transaction)
    .await
    .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;
    transaction
        .commit()
        .await
        .map_err(|error| ConfigError::DataAccessError(error.to_string()))?;

    Ok(defaults)
}

fn parse_client_runtime_defaults(value: Option<String>) -> ConfigResult<ClientRuntimeDefaults> {
    value
        .map(|value| serde_json::from_str(&value).map_err(ConfigError::from))
        .unwrap_or_else(|| Ok(ClientRuntimeDefaults::default()))
}
