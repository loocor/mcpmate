use std::{path::Path, path::PathBuf, sync::Arc};

use anyhow::Result;
use mcpmate_secrets::{RootKeyProviderMetadata, SecretRootKeyError};
use sqlx::{Pool, Sqlite};

pub use mcpmate_secrets::store::{
    LocalSecretStore, SecretCreateInput, SecretKindInput, SecretMetadataView, SecretOriginInput, SecretUpdateInput,
    SecretUsageLocationInput, SecretUsageUpsertInput, SecretUsageView,
};
pub use mcpmate_secrets::{
    LocalFileRootKeyProvider, OperatingSystemRootKeyProvider, PassphraseRootKeyProvider, RootKeyProviderMode,
    SecretRootKeyProvider, default_root_key_provider,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretStoreProviderSnapshot {
    pub provider_id: String,
    pub provider_kind: String,
    pub provider_mode: String,
    pub security_level: String,
}

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
        provider: Option<SecretStoreProviderSnapshot>,
    },
}

#[derive(Debug)]
pub struct SecretStoreBootstrap {
    pub store: Option<LocalSecretStore>,
    pub readiness: SecretStoreReadiness,
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
            provider: None,
        }
    }

    pub fn unavailable_with_provider(
        reason_code: impl Into<String>,
        message: impl Into<String>,
        metadata: RootKeyProviderMetadata,
    ) -> Self {
        Self::Unavailable {
            reason_code: reason_code.into(),
            message: message.into(),
            provider: Some(provider_snapshot(metadata)),
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
                    SecretRootKeyError::MissingMaterial(_) => "missing_root_key",
                    SecretRootKeyError::InvalidMaterial(_) => "invalid_root_key",
                    SecretRootKeyError::LocalStorage(_) => "local_storage_error",
                    SecretRootKeyError::DevelopmentStorage(_) => "development_storage_error",
                };
                return Self::unavailable(reason_code, message);
            }
        }

        Self::unavailable("initialization_failed", message)
    }

    pub fn from_initialization_error_with_provider(
        error: &anyhow::Error,
        metadata: RootKeyProviderMetadata,
    ) -> Self {
        match Self::from_initialization_error(error) {
            Self::Unavailable {
                reason_code,
                message,
                provider,
            } => Self::Unavailable {
                reason_code,
                message,
                provider: provider.or_else(|| Some(provider_snapshot(metadata))),
            },
            ready => ready,
        }
    }
}

pub fn provider_snapshot(metadata: RootKeyProviderMetadata) -> SecretStoreProviderSnapshot {
    SecretStoreProviderSnapshot {
        provider_id: metadata.provider_id().to_string(),
        provider_kind: metadata.provider_kind().to_string(),
        provider_mode: metadata.mode().as_str().to_string(),
        security_level: metadata.security_level().as_str().to_string(),
    }
}

pub fn secret_store_paths(data_dir: &Path) -> (PathBuf, PathBuf) {
    let secrets_dir = data_dir.join("secrets");
    (
        secrets_dir.join("passphrase-wrapped-key.json"),
        secrets_dir.join("local-root.key"),
    )
}

pub fn parse_persisted_provider_mode(mode: &str) -> Result<RootKeyProviderMode, String> {
    match mode {
        "passphrase" => Ok(RootKeyProviderMode::Passphrase),
        "local_file" => Ok(RootKeyProviderMode::LocalFile),
        "operating_system" => Ok(RootKeyProviderMode::OperatingSystem),
        other if other == RootKeyProviderMode::Development.as_str() => Ok(RootKeyProviderMode::Development),
        other => Err(format!("Unknown provider mode '{other}'")),
    }
}

pub fn provider_mode_to_persisted(mode: RootKeyProviderMode) -> &'static str {
    mode.as_str()
}

pub async fn bootstrap_secret_store(
    pool: Pool<Sqlite>,
    data_dir: &Path,
) -> SecretStoreBootstrap {
    if let Err(err) = LocalSecretStore::ensure_schema(&pool).await {
        return SecretStoreBootstrap {
            store: None,
            readiness: SecretStoreReadiness::from_initialization_error(&err),
        };
    }

    let (passphrase_path, local_file_path) = secret_store_paths(data_dir);
    let persisted_mode = match mcpmate_secrets::database::get_provider_config(&pool).await {
        Ok(Some(config)) => match parse_persisted_provider_mode(&config.provider_mode) {
            Ok(mode) => mode,
            Err(err) => {
                return SecretStoreBootstrap {
                    store: None,
                    readiness: SecretStoreReadiness::unavailable("provider_config_invalid", err),
                };
            }
        },
        Ok(None) => RootKeyProviderMode::OperatingSystem,
        Err(err) => {
            return SecretStoreBootstrap {
                store: None,
                readiness: SecretStoreReadiness::unavailable("provider_config_error", err.to_string()),
            };
        }
    };

    match persisted_mode {
        RootKeyProviderMode::Passphrase => {
            let metadata =
                PassphraseRootKeyProvider::new(passphrase_path.clone(), "bootstrap-metadata-only").metadata();
            // Passphrase mode always requires unlock — the wrapped key file is
            // never loaded automatically at bootstrap time.
            SecretStoreBootstrap {
                store: None,
                readiness: SecretStoreReadiness::unavailable_with_provider(
                    "passphrase_unlock_required",
                    "Enter your encryption password to unlock the secure store.",
                    metadata,
                ),
            }
        }
        RootKeyProviderMode::LocalFile => {
            initialize_with_root_key_provider(pool, Arc::new(LocalFileRootKeyProvider::new(local_file_path))).await
        }
        RootKeyProviderMode::OperatingSystem => {
            initialize_with_root_key_provider(pool, Arc::new(OperatingSystemRootKeyProvider::new())).await
        }
        RootKeyProviderMode::Development | RootKeyProviderMode::Custom => {
            initialize_with_root_key_provider(pool, default_root_key_provider()).await
        }
    }
}

pub async fn initialize_secret_store_with_passphrase(
    pool: Pool<Sqlite>,
    data_dir: &Path,
    passphrase: &str,
) -> Result<SecretStoreBootstrap> {
    let (passphrase_path, _) = secret_store_paths(data_dir);
    if !passphrase_path.exists() {
        return Ok(SecretStoreBootstrap {
            store: None,
            readiness: SecretStoreReadiness::unavailable_with_provider(
                "passphrase_setup_required",
                "Passphrase encryption is configured but no wrapped root key file was found.",
                PassphraseRootKeyProvider::new(passphrase_path, passphrase).metadata(),
            ),
        });
    }

    let provider = Arc::new(PassphraseRootKeyProvider::new(passphrase_path, passphrase));
    Ok(initialize_with_root_key_provider(pool, provider).await)
}

async fn initialize_with_root_key_provider(
    pool: Pool<Sqlite>,
    provider: Arc<dyn SecretRootKeyProvider>,
) -> SecretStoreBootstrap {
    let metadata = provider.metadata();
    match LocalSecretStore::initialize_with_root_key_provider(pool, provider).await {
        Ok(store) => {
            let readiness = SecretStoreReadiness::ready(store.provider_metadata());
            SecretStoreBootstrap {
                store: Some(store),
                readiness,
            }
        }
        Err(err) => SecretStoreBootstrap {
            store: None,
            readiness: SecretStoreReadiness::from_initialization_error_with_provider(&err, metadata),
        },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialization_error_with_provider_preserves_provider_metadata() {
        let error = anyhow::Error::new(SecretRootKeyError::ProviderUnavailable("access denied".to_string()));
        let metadata = OperatingSystemRootKeyProvider::new().metadata();

        let readiness = SecretStoreReadiness::from_initialization_error_with_provider(&error, metadata);

        match readiness {
            SecretStoreReadiness::Unavailable {
                reason_code,
                provider: Some(provider),
                ..
            } => {
                assert_eq!(reason_code, "provider_unavailable");
                assert_eq!(provider.provider_mode, "operating_system");
            }
            other => panic!("expected unavailable readiness with provider metadata, got {other:?}"),
        }
    }
}
