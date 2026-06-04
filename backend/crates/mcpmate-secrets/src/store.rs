use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{
    DevelopmentRootKeyProvider, RootKeyProviderMetadata, SecretError, SecretReference, SecretResolver,
    SecretRootKeyProvider, SecretValue,
    crypto::{EncryptedSecret, EncryptedSecretParts, EnvelopeCrypto},
    database, default_root_key_provider,
};

pub use crate::types::{
    SecretCreateInput, SecretKindInput, SecretMetadataView, SecretOriginInput, SecretUpdateInput,
    SecretUsageLocationInput, SecretUsageUpsertInput, SecretUsageView,
};

pub struct LocalSecretStore {
    pool: Pool<Sqlite>,
    crypto: EnvelopeCrypto,
    provider_metadata: RootKeyProviderMetadata,
    encrypted_cache: RwLock<HashMap<String, EncryptedSecret>>,
}

impl fmt::Debug for LocalSecretStore {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        formatter.debug_struct("LocalSecretStore").finish_non_exhaustive()
    }
}

impl LocalSecretStore {
    pub async fn initialize(pool: Pool<Sqlite>) -> Result<Self> {
        Self::initialize_with_root_key_provider(pool, default_root_key_provider()).await
    }

    pub async fn initialize_with_development_root_key(
        pool: Pool<Sqlite>,
        local_key_path: impl Into<std::path::PathBuf>,
    ) -> Result<Self> {
        Self::initialize_with_root_key_provider(pool, Arc::new(DevelopmentRootKeyProvider::new(local_key_path))).await
    }

    pub async fn initialize_with_root_key_provider(
        pool: Pool<Sqlite>,
        root_key_provider: Arc<dyn SecretRootKeyProvider>,
    ) -> Result<Self> {
        Self::ensure_schema(&pool).await?;
        let provider_metadata = root_key_provider.metadata();
        let key = root_key_provider
            .load_or_create_root_key()
            .with_context(|| format!("initialize root key provider '{}'", provider_metadata.provider_id()))?;
        let store = Self {
            pool,
            crypto: EnvelopeCrypto::new(key),
            provider_metadata,
            encrypted_cache: RwLock::new(HashMap::new()),
        };
        store.reload_cache().await?;
        Ok(store)
    }

    pub fn pool(&self) -> Pool<Sqlite> {
        self.pool.clone()
    }

    pub fn provider_metadata(&self) -> RootKeyProviderMetadata {
        self.provider_metadata.clone()
    }

    pub async fn ensure_schema(pool: &Pool<Sqlite>) -> Result<()> {
        database::ensure_schema(pool).await
    }

    pub async fn create_secret(
        &self,
        input: SecretCreateInput,
    ) -> Result<SecretMetadataView> {
        let reference = SecretReference::new(input.alias.clone()).context("invalid secret alias")?;
        let encrypted = self.crypto.encrypt(reference.alias(), &input.value)?;
        database::insert_secret(
            &self.pool,
            database::SecretInsert {
                alias: reference.alias(),
                kind: input.kind.as_str(),
                label: input.label.as_deref(),
                origin: input.origin.as_ref(),
                provider_id: self.provider_metadata.provider_id(),
                provider_kind: self.provider_metadata.provider_kind(),
                encrypted: &encrypted,
            },
        )
        .await?;

        self.cache_secret(reference.alias(), encrypted)?;
        self.get_secret_metadata(reference.alias()).await
    }

    pub async fn update_secret(
        &self,
        input: SecretUpdateInput,
    ) -> Result<SecretMetadataView> {
        let reference = SecretReference::new(input.alias.clone()).context("invalid secret alias")?;
        let existing = self.get_secret_metadata(reference.alias()).await?;
        let next_kind = input.kind.map(|kind| kind.to_string()).unwrap_or(existing.kind);
        let next_label = input.label.or(existing.label);
        let next_origin = input.origin.or(existing.origin);
        let update = database::SecretUpdate {
            alias: reference.alias(),
            kind: &next_kind,
            label: next_label.as_deref(),
            origin: next_origin.as_ref(),
        };

        if let Some(value) = input.value {
            let encrypted = self.crypto.encrypt(reference.alias(), &value)?;
            database::update_secret_with_value(&self.pool, update, &encrypted).await?;
            self.cache_secret(reference.alias(), encrypted)?;
        } else {
            database::update_secret_metadata(&self.pool, update).await?;
        }

        self.get_secret_metadata(reference.alias()).await
    }

    pub async fn get_secret_metadata(
        &self,
        alias: &str,
    ) -> Result<SecretMetadataView> {
        database::get_secret_metadata(&self.pool, alias).await
    }

    pub async fn list_secret_metadata(&self) -> Result<Vec<SecretMetadataView>> {
        database::list_secret_metadata(&self.pool).await
    }

    pub async fn delete_secret(
        &self,
        alias: &str,
        force: bool,
    ) -> Result<()> {
        let usages = self.list_usages(alias).await?;
        if !force && !usages.is_empty() {
            return Err(anyhow::anyhow!(
                "Secret '{alias}' is in use by {} runtime location(s)",
                usages.len()
            ));
        }

        database::delete_secret(&self.pool, alias).await?;
        self.encrypted_cache
            .write()
            .map_err(|_| anyhow::anyhow!("secret cache lock poisoned"))?
            .remove(alias);
        Ok(())
    }

    pub async fn upsert_usage(
        &self,
        input: SecretUsageUpsertInput,
    ) -> Result<SecretUsageView> {
        let reference = SecretReference::new(input.alias.clone()).context("invalid secret alias")?;
        database::upsert_usage(&self.pool, &input).await?;
        Ok(SecretUsageView {
            alias: reference.alias().to_string(),
            server_id: input.server_id,
            location: input.location,
        })
    }

    pub async fn replace_server_usages(
        &self,
        server_id: &str,
        usages: Vec<SecretUsageUpsertInput>,
    ) -> Result<()> {
        database::replace_server_usages(&self.pool, server_id).await?;

        for usage in usages {
            self.upsert_usage(usage).await?;
        }

        Ok(())
    }

    pub async fn list_usages(
        &self,
        alias: &str,
    ) -> Result<Vec<SecretUsageView>> {
        database::list_usages(&self.pool, alias).await
    }

    async fn reload_cache(&self) -> Result<()> {
        let rows = database::load_encrypted_secrets(&self.pool).await?;
        let mut cache = self
            .encrypted_cache
            .write()
            .map_err(|_| anyhow::anyhow!("secret cache lock poisoned"))?;
        cache.clear();
        for encrypted in rows {
            cache.insert(encrypted.alias.clone(), encrypted);
        }
        Ok(())
    }

    fn cache_secret(
        &self,
        alias: &str,
        encrypted: EncryptedSecretParts,
    ) -> Result<()> {
        self.encrypted_cache
            .write()
            .map_err(|_| anyhow::anyhow!("secret cache lock poisoned"))?
            .insert(
                alias.to_string(),
                EncryptedSecret {
                    alias: alias.to_string(),
                    key_nonce: encrypted.key_nonce,
                    encrypted_key: encrypted.encrypted_key,
                    nonce: encrypted.nonce,
                    encrypted_value: encrypted.encrypted_value,
                },
            );
        Ok(())
    }
}

impl SecretResolver for LocalSecretStore {
    fn resolve_secret(
        &self,
        reference: &SecretReference,
    ) -> Result<SecretValue, SecretError> {
        let encrypted = self
            .encrypted_cache
            .read()
            .map_err(|_| SecretError::ProviderUnavailable)?
            .get(reference.alias())
            .cloned()
            .ok_or_else(|| SecretError::NotFound(reference.alias().to_string()))?;
        self.crypto.decrypt_secret(&encrypted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{SecretRootKey, SecretRootKeyError};
    use sqlx::{Row, sqlite::SqlitePoolOptions};
    use tempfile::TempDir;

    #[derive(Debug)]
    struct FailingRootKeyProvider;

    impl SecretRootKeyProvider for FailingRootKeyProvider {
        fn metadata(&self) -> RootKeyProviderMetadata {
            RootKeyProviderMetadata::new("test-failing-provider", "test")
        }

        fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
            Err(SecretRootKeyError::ProviderUnavailable(
                "provider unavailable".to_string(),
            ))
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn initialization_with_failing_root_key_provider_does_not_fallback() {
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");

        let err = LocalSecretStore::initialize_with_root_key_provider(db_pool, Arc::new(FailingRootKeyProvider))
            .await
            .expect_err("failing provider must fail store initialization");

        assert!(err.to_string().contains("test-failing-provider"));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn development_root_key_provider_metadata_is_stored_with_secret() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let store = LocalSecretStore::initialize_with_development_root_key(
            db_pool,
            temp_dir.path().join("secrets").join("local-root.key"),
        )
        .await
        .expect("initialize store");

        let metadata = store
            .create_secret(SecretCreateInput {
                alias: "server/test/token".to_string(),
                kind: SecretKindInput::Token,
                value: "secret".to_string(),
                label: None,
                origin: None,
            })
            .await
            .expect("create secret");

        assert_eq!(metadata.provider_id, "local-encrypted-vault");
        assert_eq!(metadata.provider_kind, "local_encrypted_vault");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn create_secret_stores_origin_metadata() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let store = LocalSecretStore::initialize_with_development_root_key(
            db_pool,
            temp_dir.path().join("secrets").join("local-root.key"),
        )
        .await
        .expect("initialize store");

        let metadata = store
            .create_secret(SecretCreateInput {
                alias: "server/github/header-token".to_string(),
                kind: SecretKindInput::HeaderValue,
                value: "secret".to_string(),
                label: Some("GitHub Authorization".to_string()),
                origin: Some(SecretOriginInput {
                    server_id: Some("github".to_string()),
                    server_name: Some("GitHub".to_string()),
                    server_kind: Some("streamable_http".to_string()),
                    source: Some("server_install".to_string()),
                    field_group: Some("headers".to_string()),
                    field_key: Some("Authorization".to_string()),
                    field_index: Some(0),
                    field_path: Some("headers[0].value".to_string()),
                }),
            })
            .await
            .expect("create secret");

        assert_eq!(
            metadata.origin,
            Some(SecretOriginInput {
                server_id: Some("github".to_string()),
                server_name: Some("GitHub".to_string()),
                server_kind: Some("streamable_http".to_string()),
                source: Some("server_install".to_string()),
                field_group: Some("headers".to_string()),
                field_key: Some("Authorization".to_string()),
                field_index: Some(0),
                field_path: Some("headers[0].value".to_string()),
            })
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn create_secret_uses_per_record_envelope_keys() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let store = LocalSecretStore::initialize_with_development_root_key(
            db_pool.clone(),
            temp_dir.path().join("secrets").join("local-root.key"),
        )
        .await
        .expect("initialize store");

        for alias in ["server/one/token", "server/two/token"] {
            store
                .create_secret(SecretCreateInput {
                    alias: alias.to_string(),
                    kind: SecretKindInput::Token,
                    value: "same-secret".to_string(),
                    label: None,
                    origin: None,
                })
                .await
                .expect("create secret");
        }

        let rows = sqlx::query(
            "SELECT encrypted_key, key_nonce, encrypted_value FROM secure_store_secrets ORDER BY alias ASC",
        )
        .fetch_all(&db_pool)
        .await
        .expect("load encrypted rows");

        assert_eq!(rows.len(), 2);
        let first_key: String = rows[0].try_get("encrypted_key").expect("first encrypted key");
        let second_key: String = rows[1].try_get("encrypted_key").expect("second encrypted key");
        let first_key_nonce: String = rows[0].try_get("key_nonce").expect("first key nonce");
        let second_key_nonce: String = rows[1].try_get("key_nonce").expect("second key nonce");
        let first_value: String = rows[0].try_get("encrypted_value").expect("first encrypted value");
        let second_value: String = rows[1].try_get("encrypted_value").expect("second encrypted value");

        assert_ne!(first_key, second_key);
        assert_ne!(first_key_nonce, second_key_nonce);
        assert_ne!(first_value, second_value);
    }
}
