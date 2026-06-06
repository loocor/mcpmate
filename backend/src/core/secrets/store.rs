use std::path::PathBuf;

use anyhow::Result;
use mcpmate_secrets::{RootKeyProviderMetadata, SecretRootKeyError};
use sqlx::{Pool, Sqlite};

pub use mcpmate_secrets::store::{
    LocalSecretStore, SecretCreateInput, SecretKindInput, SecretMetadataView, SecretOriginInput, SecretUpdateInput,
    SecretUsageLocationInput, SecretUsageUpsertInput, SecretUsageView,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretStoreReadiness {
    Ready {
        provider_id: String,
        provider_kind: String,
        provider_mode: String,
        security_level: String,
    },
    Unavailable {
        reason_code: String,
        message: String,
    },
}

impl SecretStoreReadiness {
    pub fn ready(metadata: RootKeyProviderMetadata) -> Self {
        Self::Ready {
            provider_id: metadata.provider_id().to_string(),
            provider_kind: metadata.provider_kind().to_string(),
            provider_mode: metadata.mode().as_str().to_string(),
            security_level: metadata.security_level().as_str().to_string(),
        }
    }

    pub fn unavailable(
        reason_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Unavailable {
            reason_code: reason_code.into(),
            message: message.into(),
        }
    }

    pub fn from_initialization_error(error: &anyhow::Error) -> Self {
        let message = error.to_string();

        if message.contains("outdated secure_store_secrets schema") {
            return Self::unavailable("schema_migration_required", message);
        }
        if message.contains("load encrypted secrets") {
            return Self::unavailable("cache_error", message);
        }

        for cause in error.chain() {
            if let Some(root_key_error) = cause.downcast_ref::<SecretRootKeyError>() {
                let reason_code = match root_key_error {
                    SecretRootKeyError::ProviderUnavailable(_) => "provider_unavailable",
                    SecretRootKeyError::InvalidMaterial(_) => "invalid_root_key",
                    SecretRootKeyError::LocalStorage(_) => "local_storage_error",
                    SecretRootKeyError::DevelopmentStorage(_) => "development_storage_error",
                };
                return Self::unavailable(reason_code, message);
            }
        }

        Self::unavailable("initialization_failed", message)
    }
}

pub async fn initialize_development_secret_store(pool: Pool<Sqlite>) -> Result<LocalSecretStore> {
    LocalSecretStore::initialize_with_development_root_key(pool, development_root_key_path()?).await
}

pub fn development_root_key_path() -> Result<PathBuf> {
    Ok(crate::common::paths::global_paths()
        .base_dir()
        .join("secrets")
        .join("local-root.key"))
}
