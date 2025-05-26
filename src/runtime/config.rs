//! Runtime configuration management

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};
use thiserror::Error;

use super::types::RuntimeType;
use crate::generate_id;

/// Runtime configuration error
#[derive(Debug, Error)]
pub enum RuntimeConfigError {
    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    /// Invalid runtime type
    #[error("Invalid runtime type: {0}")]
    InvalidRuntimeType(String),

    /// Configuration not found
    #[error("Runtime configuration not found: {0}")]
    NotFound(String),

    /// Other error
    #[error("Runtime configuration error: {0}")]
    Other(String),
}

/// Runtime configuration stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RuntimeConfig {
    /// Unique identifier
    pub id: Option<String>,
    /// Runtime type (node, bun, uv)
    pub runtime_type: String,
    /// Version (v16, latest, etc.)
    pub version: String,
    /// Binary path relative to user directory
    pub relative_bin_path: String,
    /// Cache path relative to user directory (optional)
    pub relative_cache_path: Option<String>,
    /// Sub runtime type (e.g., 'python' for uv)
    pub sub_runtime_type: Option<String>,
    /// Sub runtime version (e.g., Python version for uv)
    pub sub_runtime_version: Option<String>,
    /// Whether this is the default version for this runtime type
    pub is_default: bool,
    /// Platform (windows, macos, linux)
    pub platform: Option<String>,
    /// Architecture (x86_64, aarch64)
    pub architecture: Option<String>,
    /// When the record was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the record was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

impl RuntimeConfig {
    /// Create a new runtime configuration
    pub fn new(
        runtime_type: RuntimeType,
        version: &str,
        relative_bin_path: &str,
    ) -> Self {
        let runtime_type_str = runtime_type.as_str().to_string();

        Self {
            id: None,
            runtime_type: runtime_type_str,
            version: version.to_string(),
            relative_bin_path: relative_bin_path.to_string(),
            relative_cache_path: None,
            sub_runtime_type: None,
            sub_runtime_version: None,
            is_default: false,
            platform: None,
            architecture: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Get the runtime type
    pub fn get_runtime_type(&self) -> Result<RuntimeType, RuntimeConfigError> {
        self.runtime_type
            .parse::<RuntimeType>()
            .map_err(|_e| RuntimeConfigError::InvalidRuntimeType(self.runtime_type.clone()))
    }
}

/// Save a runtime configuration to the database
pub async fn save_config(
    pool: &Pool<Sqlite>,
    config: &RuntimeConfig,
) -> Result<String, RuntimeConfigError> {
    // Generate ID if not provided
    let id = config.id.clone().unwrap_or_else(|| generate_id!("runt"));

    // Insert or update the configuration using runtime_type as unique constraint
    sqlx::query(
        r#"
        INSERT INTO runtime_config (
            id, runtime_type, version, relative_bin_path, relative_cache_path,
            sub_runtime_type, sub_runtime_version, is_default, platform, architecture
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(runtime_type) DO UPDATE SET
            version = excluded.version,
            relative_bin_path = excluded.relative_bin_path,
            relative_cache_path = excluded.relative_cache_path,
            sub_runtime_type = excluded.sub_runtime_type,
            sub_runtime_version = excluded.sub_runtime_version,
            is_default = excluded.is_default,
            platform = excluded.platform,
            architecture = excluded.architecture,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&id)
    .bind(&config.runtime_type)
    .bind(&config.version)
    .bind(&config.relative_bin_path)
    .bind(&config.relative_cache_path)
    .bind(&config.sub_runtime_type)
    .bind(&config.sub_runtime_version)
    .bind(config.is_default)
    .bind(&config.platform)
    .bind(&config.architecture)
    .execute(pool)
    .await
    .map_err(RuntimeConfigError::DatabaseError)?;

    // Return the runtime_type as the identifier
    Ok(config.runtime_type.clone())
}

/// Get a runtime configuration by runtime type
pub async fn get_config_by_type(
    pool: &Pool<Sqlite>,
    runtime_type: RuntimeType,
) -> Result<RuntimeConfig, RuntimeConfigError> {
    let runtime_type_str = runtime_type.as_str();

    sqlx::query_as::<_, RuntimeConfig>(
        r#"
        SELECT * FROM runtime_config WHERE runtime_type = ?
        "#,
    )
    .bind(runtime_type_str)
    .fetch_optional(pool)
    .await
    .map_err(RuntimeConfigError::DatabaseError)?
    .ok_or_else(|| RuntimeConfigError::NotFound(runtime_type_str.to_string()))
}

/// Get all runtime configurations
pub async fn get_all_configs(
    pool: &Pool<Sqlite>
) -> Result<Vec<RuntimeConfig>, RuntimeConfigError> {
    sqlx::query_as::<_, RuntimeConfig>(
        r#"
        SELECT * FROM runtime_config
        ORDER BY runtime_type
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(RuntimeConfigError::DatabaseError)
}
