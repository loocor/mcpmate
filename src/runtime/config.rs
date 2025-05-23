//! Runtime configuration management

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Sqlite};
use std::path::PathBuf;
use thiserror::Error;

use super::constants::*;
use super::types::{RuntimeType, RuntimeVersion};

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

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RuntimeConfig {
    /// Unique ID
    pub id: Option<String>,
    /// Runtime type (node, bun, uv)
    pub runtime_type: String,
    /// Version (v16, latest, etc.)
    pub version: String,
    /// Binary path relative to user directory
    pub relative_bin_path: String,
    /// Cache path relative to user directory
    pub relative_cache_path: Option<String>,
    /// Sub-runtime type (e.g., python for uv)
    pub sub_runtime_type: Option<String>,
    /// Sub-runtime version (e.g., 3.10 for python)
    pub sub_runtime_version: Option<String>,
    /// Whether this is the default version for this runtime type
    pub is_default: bool,
    /// Platform (windows, macos, linux)
    pub platform: Option<String>,
    /// Architecture (x86_64, aarch64)
    pub architecture: Option<String>,
    /// When the configuration was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the configuration was last updated
    pub updated_at: Option<DateTime<Utc>>,
}

impl RuntimeConfig {
    /// Create a new runtime configuration
    pub fn new(
        runtime_type: RuntimeType,
        version: &str,
        is_default: bool,
    ) -> Self {
        let runtime_type_str = runtime_type.as_str().to_string();
        let relative_bin_path = format!("runtimes/{}/{}/bin", runtime_type_str, version);
        let relative_cache_path = format!("cache/{}", runtime_type_str);

        Self {
            id: None,
            runtime_type: runtime_type_str,
            version: version.to_string(),
            relative_bin_path,
            relative_cache_path: Some(relative_cache_path),
            sub_runtime_type: None,
            sub_runtime_version: None,
            is_default,
            platform: None,
            architecture: None,
            created_at: None,
            updated_at: None,
        }
    }

    /// Create a new runtime configuration with sub-runtime
    pub fn new_with_sub_runtime(
        runtime_type: RuntimeType,
        version: &str,
        sub_runtime_type: &str,
        sub_runtime_version: &str,
        is_default: bool,
    ) -> Self {
        let mut config = Self::new(runtime_type, version, is_default);
        config.sub_runtime_type = Some(sub_runtime_type.to_string());
        config.sub_runtime_version = Some(sub_runtime_version.to_string());
        config
    }

    /// Get the runtime type
    pub fn get_runtime_type(&self) -> Result<RuntimeType, RuntimeConfigError> {
        self.runtime_type
            .parse::<RuntimeType>()
            .map_err(|e| RuntimeConfigError::InvalidRuntimeType(self.runtime_type.clone()))
    }
}

/// Create runtime_config table if it doesn't exist
pub async fn create_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating runtime_config table if it doesn't exist");
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS runtime_config (
            id TEXT PRIMARY KEY,
            runtime_type TEXT NOT NULL,
            version TEXT NOT NULL,
            relative_bin_path TEXT NOT NULL,
            relative_cache_path TEXT,
            sub_runtime_type TEXT,
            sub_runtime_version TEXT,
            is_default BOOLEAN NOT NULL DEFAULT 0,
            platform TEXT,
            architecture TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create runtime_config table")?;

    // Create index on runtime_type and version
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_runtime_config_type_version
        ON runtime_config(runtime_type, version)
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create index on runtime_config")?;

    // Create index on is_default
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_runtime_config_default
        ON runtime_config(runtime_type, is_default)
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create index on runtime_config")?;

    tracing::debug!("runtime_config table created or already exists");
    Ok(())
}

/// Save a runtime configuration to the database
pub async fn save_config(
    pool: &Pool<Sqlite>,
    config: &RuntimeConfig,
) -> Result<String, RuntimeConfigError> {
    let id = if let Some(id) = &config.id {
        id.clone()
    } else {
        // Generate a new ID with 'runt' prefix
        format!("runt{}", nanoid::nanoid!(12))
    };

    // If this is set as default, unset any other default for this runtime type
    if config.is_default {
        sqlx::query(
            r#"
            UPDATE runtime_config
            SET is_default = 0
            WHERE runtime_type = ? AND is_default = 1
            "#,
        )
        .bind(&config.runtime_type)
        .execute(pool)
        .await
        .map_err(RuntimeConfigError::DatabaseError)?;
    }

    // Insert or update the configuration
    sqlx::query(
        r#"
        INSERT INTO runtime_config (
            id, runtime_type, version, relative_bin_path, relative_cache_path,
            sub_runtime_type, sub_runtime_version, is_default, platform, architecture
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            runtime_type = excluded.runtime_type,
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
    .bind(&config.is_default)
    .bind(&config.platform)
    .bind(&config.architecture)
    .execute(pool)
    .await
    .map_err(RuntimeConfigError::DatabaseError)?;

    Ok(id)
}

/// Get a runtime configuration by ID
pub async fn get_config_by_id(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<RuntimeConfig, RuntimeConfigError> {
    sqlx::query_as::<_, RuntimeConfig>(
        r#"
        SELECT * FROM runtime_config WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(RuntimeConfigError::DatabaseError)?
    .ok_or_else(|| RuntimeConfigError::NotFound(id.to_string()))
}

/// Get the default runtime configuration for a runtime type
pub async fn get_default_config(
    pool: &Pool<Sqlite>,
    runtime_type: RuntimeType,
) -> Result<RuntimeConfig, RuntimeConfigError> {
    let runtime_type_str = runtime_type.as_str();

    sqlx::query_as::<_, RuntimeConfig>(
        r#"
        SELECT * FROM runtime_config
        WHERE runtime_type = ? AND is_default = 1
        "#,
    )
    .bind(runtime_type_str)
    .fetch_optional(pool)
    .await
    .map_err(RuntimeConfigError::DatabaseError)?
    .ok_or_else(|| {
        RuntimeConfigError::NotFound(format!("Default configuration for {}", runtime_type_str))
    })
}

/// Get a runtime configuration by type and version
pub async fn get_config(
    pool: &Pool<Sqlite>,
    runtime_type: RuntimeType,
    version: &str,
) -> Result<RuntimeConfig, RuntimeConfigError> {
    let runtime_type_str = runtime_type.as_str();

    sqlx::query_as::<_, RuntimeConfig>(
        r#"
        SELECT * FROM runtime_config
        WHERE runtime_type = ? AND version = ?
        "#,
    )
    .bind(runtime_type_str)
    .bind(version)
    .fetch_optional(pool)
    .await
    .map_err(RuntimeConfigError::DatabaseError)?
    .ok_or_else(|| {
        RuntimeConfigError::NotFound(format!(
            "Configuration for {} {}",
            runtime_type_str, version
        ))
    })
}
