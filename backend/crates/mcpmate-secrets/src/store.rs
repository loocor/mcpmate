use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};

use anyhow::{Context, Result};
use sqlx::{Pool, Row, Sqlite, SqliteConnection};
use thiserror::Error;

use crate::{
    DevelopmentRootKeyProvider, RootKeyProviderMetadata, SecretError, SecretReference, SecretResolver, SecretRootKey,
    SecretRootKeyError, SecretRootKeyProvider, SecretValue,
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

#[derive(Debug, Error)]
pub enum SecretStoreRotationError {
    #[error("current root key provider failed: {0}")]
    CurrentProviderUnavailable(SecretRootKeyError),
    #[error("target root key provider failed: {0}")]
    TargetProviderUnavailable(SecretRootKeyError),
    #[error("secret '{alias}' cannot be decrypted before rotation: {message}")]
    CurrentRecordUnreadable { alias: String, message: String },
    #[error("secret '{alias}' cannot be decrypted after rotation: {message}")]
    PostRotationVerificationFailed { alias: String, message: String },
    #[error("secure store rotation persistence failed during {action}: {message}")]
    PersistenceFailed { action: &'static str, message: String },
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
        let secret_count = database::secure_store_secret_count(&pool).await?;
        let key = if secret_count > 0 {
            root_key_provider.load_existing_root_key()
        } else {
            root_key_provider.load_or_create_root_key()
        }
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
        let next_kind = match input.kind {
            Some(kind) if kind.as_str() != existing.kind => {
                anyhow::bail!("Secret kind cannot be changed after creation");
            }
            Some(kind) => kind.to_string(),
            None => existing.kind,
        };
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
        if !force {
            let usages = self.list_usages(alias).await?;
            if !usages.is_empty() {
                return Err(anyhow::anyhow!(
                    "Secret '{alias}' is in use by {} runtime location(s)",
                    usages.len()
                ));
            }

            let unsupported_count = self.count_unsupported_usages_for_alias(alias).await?;
            if unsupported_count > 0 {
                return Err(anyhow::anyhow!(
                    "Secret '{alias}' is in use by {unsupported_count} unsupported runtime location(s)"
                ));
            }
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
        SecretReference::new(input.alias.clone()).context("invalid secret alias")?;
        database::upsert_usage(&self.pool, &input).await?;
        Ok(input.into())
    }

    pub async fn replace_server_usages(
        &self,
        server_id: &str,
        mut usages: Vec<SecretUsageUpsertInput>,
    ) -> Result<()> {
        database::replace_server_usages(&self.pool, server_id).await?;

        // Deduplicate: when the same placeholder appears twice in one runtime
        // location (e.g. a URL containing [[secret:x]] twice), the caller will
        // push identical (alias, server_id, location) entries.  SQLite UNIQUE
        // constraints treat NULLs as distinct, so the second upsert would hit a
        // primary-key collision.  Sorting + dedup_by on the composite key
        // removes exact duplicates before the upsert loop.
        usages.sort_by(|a, b| {
            a.alias
                .cmp(&b.alias)
                .then_with(|| a.server_id.cmp(&b.server_id))
                .then_with(|| a.location.parts().0.cmp(b.location.parts().0))
                .then_with(|| a.location.parts().1.cmp(&b.location.parts().1))
                .then_with(|| a.location.parts().2.cmp(&b.location.parts().2))
        });
        usages.dedup_by(|a, b| a.alias == b.alias && a.server_id == b.server_id && a.location == b.location);

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

    pub async fn list_all_usages(&self) -> Result<Vec<SecretUsageView>> {
        database::list_all_usages(&self.pool).await
    }

    pub async fn list_all_usages_with_unsupported_counts(
        &self
    ) -> Result<(Vec<SecretUsageView>, HashMap<String, u64>)> {
        let rows = database::list_all_usage_rows(&self.pool).await?;
        Ok((rows.usages, rows.unsupported_count_by_alias))
    }

    pub async fn count_unsupported_usages_by_alias(&self) -> Result<HashMap<String, u64>> {
        database::count_unsupported_usages_by_alias(&self.pool).await
    }

    pub async fn count_unsupported_usages_for_alias(
        &self,
        alias: &str,
    ) -> Result<u64> {
        database::count_unsupported_usages_for_alias(&self.pool, alias).await
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

    pub async fn rotate_provider(
        pool: Pool<Sqlite>,
        current_provider: Arc<dyn SecretRootKeyProvider>,
        target_provider: Arc<dyn SecretRootKeyProvider>,
    ) -> std::result::Result<Self, SecretStoreRotationError> {
        Self::ensure_schema(&pool)
            .await
            .map_err(|err| SecretStoreRotationError::PersistenceFailed {
                action: "ensure schema",
                message: err.to_string(),
            })?;

        let mut conn = pool
            .acquire()
            .await
            .map_err(|err| SecretStoreRotationError::PersistenceFailed {
                action: "acquire rotation connection",
                message: err.to_string(),
            })?;
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *conn)
            .await
            .map_err(|err| SecretStoreRotationError::PersistenceFailed {
                action: "begin rotation transaction",
                message: err.to_string(),
            })?;

        let target_metadata = target_provider.metadata();
        let mut target_material = None;
        let rotation_result = async {
            let current_root_key = current_provider
                .load_existing_root_key()
                .map_err(SecretStoreRotationError::CurrentProviderUnavailable)?;
            let current_crypto = EnvelopeCrypto::new(current_root_key);
            let encrypted_secrets = load_encrypted_secrets_for_rotation(&mut conn).await?;
            let verified_data_keys = verify_current_records(&current_crypto, encrypted_secrets)?;

            let material = target_provider
                .generate_and_store_root_key_for_rotation()
                .map_err(SecretStoreRotationError::TargetProviderUnavailable)?;
            let target_root_key = material.root_key();
            let target_crypto = EnvelopeCrypto::new(target_root_key);
            target_material = Some(material);
            let rotated_records = rotate_secret_records(&target_crypto, &verified_data_keys)?;

            persist_rotated_records(&mut conn, &target_metadata, &rotated_records).await?;
            persist_provider_mode(&mut conn, &target_metadata).await?;

            sqlx::query("COMMIT").execute(&mut *conn).await.map_err(|err| {
                SecretStoreRotationError::PersistenceFailed {
                    action: "commit rotation transaction",
                    message: err.to_string(),
                }
            })?;

            Ok::<_, SecretStoreRotationError>((target_root_key, rotated_records))
        }
        .await;

        let (target_root_key, rotated_records) = match rotation_result {
            Ok(rotated_state) => {
                if let Some(material) = target_material.take() {
                    material.commit();
                }
                rotated_state
            }
            Err(err) => {
                let err = rollback_rotation_failure(&mut conn, target_material.take(), err).await;
                return Err(err);
            }
        };

        drop(conn);

        Ok(Self::from_rotated_records(
            pool,
            target_root_key,
            target_metadata,
            rotated_records,
        ))
    }

    fn from_rotated_records(
        pool: Pool<Sqlite>,
        root_key: SecretRootKey,
        provider_metadata: RootKeyProviderMetadata,
        rotated_records: Vec<EncryptedSecret>,
    ) -> Self {
        let encrypted_cache = rotated_records
            .into_iter()
            .map(|record| (record.alias.clone(), record))
            .collect();
        Self {
            pool,
            crypto: EnvelopeCrypto::new(root_key),
            provider_metadata,
            encrypted_cache: RwLock::new(encrypted_cache),
        }
    }
}

type VerifiedDataKey = (EncryptedSecret, SecretRootKey);

fn verify_current_records(
    current_crypto: &EnvelopeCrypto,
    encrypted_secrets: Vec<EncryptedSecret>,
) -> std::result::Result<Vec<VerifiedDataKey>, SecretStoreRotationError> {
    let mut verified_data_keys = Vec::with_capacity(encrypted_secrets.len());
    for encrypted in encrypted_secrets {
        let data_key = current_crypto.unwrap_data_key(&encrypted).map_err(|err| {
            SecretStoreRotationError::CurrentRecordUnreadable {
                alias: encrypted.alias.clone(),
                message: err.to_string(),
            }
        })?;
        current_crypto
            .decrypt_secret_with_data_key(&encrypted, &data_key)
            .map_err(|err| SecretStoreRotationError::CurrentRecordUnreadable {
                alias: encrypted.alias.clone(),
                message: err.to_string(),
            })?;
        verified_data_keys.push((encrypted, data_key));
    }
    Ok(verified_data_keys)
}

fn rotate_secret_records(
    target_crypto: &EnvelopeCrypto,
    verified_data_keys: &[VerifiedDataKey],
) -> std::result::Result<Vec<EncryptedSecret>, SecretStoreRotationError> {
    let mut rotated_records = Vec::with_capacity(verified_data_keys.len());
    for (encrypted, data_key) in verified_data_keys {
        let (key_nonce, encrypted_key) = target_crypto.wrap_data_key(&encrypted.alias, data_key).map_err(|err| {
            SecretStoreRotationError::PostRotationVerificationFailed {
                alias: encrypted.alias.clone(),
                message: err.to_string(),
            }
        })?;
        let rotated = EncryptedSecret {
            alias: encrypted.alias.clone(),
            key_nonce,
            encrypted_key,
            nonce: encrypted.nonce.clone(),
            encrypted_value: encrypted.encrypted_value.clone(),
        };
        target_crypto.decrypt_secret(&rotated).map_err(|err| {
            SecretStoreRotationError::PostRotationVerificationFailed {
                alias: rotated.alias.clone(),
                message: err.to_string(),
            }
        })?;
        rotated_records.push(rotated);
    }
    Ok(rotated_records)
}

async fn persist_rotated_records(
    conn: &mut SqliteConnection,
    target_metadata: &RootKeyProviderMetadata,
    rotated_records: &[EncryptedSecret],
) -> std::result::Result<(), SecretStoreRotationError> {
    for rotated in rotated_records {
        let result = sqlx::query(
            r#"
            UPDATE secure_store_secrets
            SET key_nonce = ?2,
                encrypted_key = ?3,
                provider_id = ?4,
                provider_kind = ?5,
                updated_at = CURRENT_TIMESTAMP
            WHERE alias = ?1
            "#,
        )
        .bind(&rotated.alias)
        .bind(&rotated.key_nonce)
        .bind(&rotated.encrypted_key)
        .bind(target_metadata.provider_id())
        .bind(target_metadata.provider_kind())
        .execute(&mut *conn)
        .await
        .map_err(|err| SecretStoreRotationError::PersistenceFailed {
            action: "update rotated secret records",
            message: err.to_string(),
        })?;
        if result.rows_affected() != 1 {
            return Err(SecretStoreRotationError::PersistenceFailed {
                action: "update rotated secret records",
                message: format!("secret '{}' changed during rotation", rotated.alias),
            });
        }
    }
    Ok(())
}

async fn persist_provider_mode(
    conn: &mut SqliteConnection,
    target_metadata: &RootKeyProviderMetadata,
) -> std::result::Result<(), SecretStoreRotationError> {
    sqlx::query(
        r#"
        INSERT INTO secure_store_provider_config (id, provider_mode, updated_at)
        VALUES (1, ?1, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
            provider_mode = ?1,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(target_metadata.mode().as_str())
    .execute(&mut *conn)
    .await
    .map_err(|err| SecretStoreRotationError::PersistenceFailed {
        action: "persist provider mode",
        message: err.to_string(),
    })?;
    Ok(())
}

async fn load_encrypted_secrets_for_rotation(
    conn: &mut SqliteConnection
) -> std::result::Result<Vec<EncryptedSecret>, SecretStoreRotationError> {
    let rows = sqlx::query("SELECT alias, key_nonce, encrypted_key, nonce, encrypted_value FROM secure_store_secrets")
        .fetch_all(&mut *conn)
        .await
        .map_err(load_encrypted_secrets_error)?;

    rows.into_iter()
        .map(|row| {
            let alias: String = row.try_get("alias").map_err(load_encrypted_secrets_error)?;
            Ok(EncryptedSecret {
                alias,
                key_nonce: row.try_get("key_nonce").map_err(load_encrypted_secrets_error)?,
                encrypted_key: row.try_get("encrypted_key").map_err(load_encrypted_secrets_error)?,
                nonce: row.try_get("nonce").map_err(load_encrypted_secrets_error)?,
                encrypted_value: row.try_get("encrypted_value").map_err(load_encrypted_secrets_error)?,
            })
        })
        .collect()
}

fn load_encrypted_secrets_error(err: sqlx::Error) -> SecretStoreRotationError {
    SecretStoreRotationError::PersistenceFailed {
        action: "load encrypted secrets",
        message: err.to_string(),
    }
}

async fn rollback_rotation_failure(
    conn: &mut SqliteConnection,
    target_material: Option<crate::StagedSecretRootKeyMaterial>,
    err: SecretStoreRotationError,
) -> SecretStoreRotationError {
    if let Some(material) = target_material {
        if let Err(rollback_err) = material.rollback() {
            let original = err.to_string();
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            return SecretStoreRotationError::PersistenceFailed {
                action: "rollback target root key material",
                message: format!("{rollback_err}; original rotation error: {original}"),
            };
        }
    }

    let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
    err
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

    use crate::{
        LocalFileRootKeyProvider, RootKeyProviderMode, RootKeySecurityLevel, SecretRootKey, SecretRootKeyError,
        SecretStoreRotationError,
    };
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

        fn load_existing_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
            Err(SecretRootKeyError::ProviderUnavailable(
                "provider unavailable".to_string(),
            ))
        }

        fn generate_and_store_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
            Err(SecretRootKeyError::ProviderUnavailable(
                "provider unavailable".to_string(),
            ))
        }
    }

    #[derive(Debug, Clone)]
    struct TestRootKeyProvider {
        fallback: LocalFileRootKeyProvider,
        metadata: RootKeyProviderMetadata,
    }

    impl TestRootKeyProvider {
        fn new(
            path: std::path::PathBuf,
            provider_id: &'static str,
            provider_kind: &'static str,
        ) -> Self {
            Self {
                fallback: LocalFileRootKeyProvider::new(path),
                metadata: RootKeyProviderMetadata::with_mode(
                    provider_id,
                    provider_kind,
                    RootKeyProviderMode::LocalFile,
                    RootKeySecurityLevel::BasicLocal,
                ),
            }
        }
    }

    impl SecretRootKeyProvider for TestRootKeyProvider {
        fn metadata(&self) -> RootKeyProviderMetadata {
            self.metadata.clone()
        }

        fn load_existing_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
            self.fallback.load_existing_root_key()
        }

        fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
            self.fallback.load_or_create_root_key()
        }

        fn generate_and_store_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
            self.fallback.generate_and_store_root_key()
        }

        fn generate_and_store_root_key_for_rotation(
            &self
        ) -> Result<crate::StagedSecretRootKeyMaterial, SecretRootKeyError> {
            self.fallback.generate_and_store_root_key_for_rotation()
        }
    }

    #[derive(Debug, Clone)]
    struct RotationOnlyRootKeyProvider {
        fallback: TestRootKeyProvider,
    }

    impl RotationOnlyRootKeyProvider {
        fn new(path: std::path::PathBuf) -> Self {
            Self {
                fallback: TestRootKeyProvider::new(path, "rotation-only-provider", "rotation_only_kind"),
            }
        }
    }

    impl SecretRootKeyProvider for RotationOnlyRootKeyProvider {
        fn metadata(&self) -> RootKeyProviderMetadata {
            self.fallback.metadata()
        }

        fn load_existing_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
            Err(SecretRootKeyError::ProviderUnavailable(
                "post-commit reload disabled".to_string(),
            ))
        }

        fn load_or_create_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
            self.fallback.load_or_create_root_key()
        }

        fn generate_and_store_root_key(&self) -> Result<SecretRootKey, SecretRootKeyError> {
            self.fallback.generate_and_store_root_key()
        }

        fn generate_and_store_root_key_for_rotation(
            &self
        ) -> Result<crate::StagedSecretRootKeyMaterial, SecretRootKeyError> {
            self.fallback.generate_and_store_root_key_for_rotation()
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
    async fn initialization_with_existing_secrets_does_not_create_missing_local_root_key() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        LocalSecretStore::ensure_schema(&db_pool).await.expect("ensure schema");
        sqlx::query(
            r#"
            INSERT INTO secure_store_secrets (
                alias,
                kind,
                provider_id,
                provider_kind,
                version,
                key_nonce,
                encrypted_key,
                nonce,
                encrypted_value
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind("server/test/token")
        .bind("token")
        .bind("local-file-root-key")
        .bind("local_encrypted_vault")
        .bind(1_i64)
        .bind("fake-key-nonce")
        .bind("fake-encrypted-key")
        .bind("fake-nonce")
        .bind("fake-encrypted-value")
        .execute(&db_pool)
        .await
        .expect("insert fake secret row");

        let local_key_path = temp_dir.path().join("secrets").join("missing-local-root.key");
        let err = LocalSecretStore::initialize_with_root_key_provider(
            db_pool,
            Arc::new(LocalFileRootKeyProvider::new(&local_key_path)),
        )
        .await
        .expect_err("existing secrets must not create a replacement local root key");

        assert!(
            err.chain()
                .any(|cause| cause.to_string().contains("root key material is missing"))
        );
        assert!(!local_key_path.exists());
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
    async fn list_usages_skips_unknown_location_rows() {
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

        store
            .create_secret(SecretCreateInput {
                alias: "provider/api-key".to_string(),
                kind: SecretKindInput::ApiKey,
                value: "secret".to_string(),
                label: None,
                origin: None,
            })
            .await
            .expect("create secret");
        store
            .upsert_usage(SecretUsageUpsertInput {
                alias: "provider/api-key".to_string(),
                server_id: "known-server".to_string(),
                location: SecretUsageLocationInput::StdioEnv {
                    name: "API_KEY".to_string(),
                },
            })
            .await
            .expect("record known usage");

        sqlx::query(
            r#"
            INSERT INTO secure_store_usages (
                id, alias, server_id, location_kind, location_name, location_index
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind("future-location-row")
        .bind("provider/api-key")
        .bind("LLMPROVFuture")
        .bind("llm_provider_api_key")
        .bind(Option::<String>::None)
        .bind(Option::<i64>::None)
        .execute(&store.pool())
        .await
        .expect("insert unknown usage row");

        let usages = store.list_usages("provider/api-key").await.expect("list usages");
        assert_eq!(usages.len(), 1);
        assert_eq!(usages[0].server_id, "known-server");
        assert_eq!(
            usages[0].location,
            SecretUsageLocationInput::StdioEnv {
                name: "API_KEY".to_string()
            }
        );

        let all_usages = store.list_all_usages().await.expect("list all usages");
        assert_eq!(all_usages.len(), 1);
        assert_eq!(all_usages[0].server_id, "known-server");

        let unknown_counts = store
            .count_unsupported_usages_by_alias()
            .await
            .expect("count unknown usages");
        assert_eq!(unknown_counts.get("provider/api-key"), Some(&1));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn delete_secret_without_force_blocks_unknown_location_rows() {
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

        store
            .create_secret(SecretCreateInput {
                alias: "provider/future-only".to_string(),
                kind: SecretKindInput::ApiKey,
                value: "secret".to_string(),
                label: None,
                origin: None,
            })
            .await
            .expect("create secret");

        sqlx::query(
            r#"
            INSERT INTO secure_store_usages (
                id, alias, server_id, location_kind, location_name, location_index
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind("future-only-location-row")
        .bind("provider/future-only")
        .bind("LLMPROVFuture")
        .bind("llm_provider_api_key")
        .bind(Option::<String>::None)
        .bind(Option::<i64>::None)
        .execute(&store.pool())
        .await
        .expect("insert unknown usage row");

        let usages = store.list_usages("provider/future-only").await.expect("list usages");
        assert!(usages.is_empty());

        let err = store
            .delete_secret("provider/future-only", false)
            .await
            .expect_err("unknown usage must block non-force delete");
        assert!(err.to_string().contains("unsupported runtime location"));

        store
            .delete_secret("provider/future-only", true)
            .await
            .expect("force delete should allow unknown usage cleanup");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn update_secret_rejects_kind_changes() {
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

        store
            .create_secret(SecretCreateInput {
                alias: "server/github/token".to_string(),
                kind: SecretKindInput::Token,
                value: "secret".to_string(),
                label: None,
                origin: None,
            })
            .await
            .expect("create secret");

        let error = store
            .update_secret(SecretUpdateInput {
                alias: "server/github/token".to_string(),
                kind: Some(SecretKindInput::OAuthAccessToken),
                value: None,
                label: Some("GitHub token".to_string()),
                origin: None,
            })
            .await
            .expect_err("kind changes should be rejected");

        assert!(error.to_string().contains("kind cannot be changed"));

        let metadata = store
            .update_secret(SecretUpdateInput {
                alias: "server/github/token".to_string(),
                kind: Some(SecretKindInput::Token),
                value: None,
                label: Some("GitHub token".to_string()),
                origin: None,
            })
            .await
            .expect("same-kind update should remain compatible");
        assert_eq!(metadata.kind, "token");
        assert_eq!(metadata.label.as_deref(), Some("GitHub token"));
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

    #[tokio::test]
    #[serial_test::serial]
    async fn rotate_provider_rewraps_records_updates_metadata_and_persists_mode() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let provider_a = Arc::new(TestRootKeyProvider::new(
            temp_dir.path().join("secrets-a").join("local-root.key"),
            "provider-a",
            "provider_a_kind",
        ));
        let provider_b = Arc::new(TestRootKeyProvider::new(
            temp_dir.path().join("secrets-b").join("local-root.key"),
            "provider-b",
            "provider_b_kind",
        ));
        let store = LocalSecretStore::initialize_with_root_key_provider(db_pool.clone(), provider_a.clone())
            .await
            .expect("initialize provider A");
        database::upsert_provider_config(&db_pool, provider_a.metadata().mode().as_str())
            .await
            .expect("persist provider A");

        create_test_secret(&store, "rotate/alpha", SecretKindInput::Token, "alpha-value").await;
        create_test_secret(&store, "rotate/beta", SecretKindInput::ApiKey, "beta-value").await;

        let before_rows = encrypted_rows(&db_pool).await;
        let rotated = LocalSecretStore::rotate_provider(db_pool.clone(), provider_a, provider_b.clone())
            .await
            .expect("rotate provider");
        let after_rows = encrypted_rows(&db_pool).await;

        assert_eq!(before_rows.len(), after_rows.len());
        for (before, after) in before_rows.iter().zip(after_rows.iter()) {
            assert_eq!(before.alias, after.alias);
            assert_ne!(before.key_nonce, after.key_nonce);
            assert_ne!(before.encrypted_key, after.encrypted_key);
            assert_eq!(before.encrypted_value, after.encrypted_value);
            assert_eq!(after.provider_id, "provider-b");
            assert_eq!(after.provider_kind, "provider_b_kind");
        }
        assert_eq!(rotated.provider_metadata(), provider_b.metadata());
        assert_eq!(
            database::get_provider_config(&db_pool)
                .await
                .expect("provider config")
                .expect("provider config row")
                .provider_mode,
            "local_file"
        );
        assert_secret_value(&rotated, "rotate/alpha", "alpha-value");
        assert_secret_value(&rotated, "rotate/beta", "beta-value");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn rotate_provider_rejects_corrupted_current_record_without_mutation() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let provider_a = Arc::new(TestRootKeyProvider::new(
            temp_dir.path().join("secrets-a").join("local-root.key"),
            "provider-a",
            "provider_a_kind",
        ));
        let provider_b = Arc::new(TestRootKeyProvider::new(
            temp_dir.path().join("secrets-b").join("local-root.key"),
            "provider-b",
            "provider_b_kind",
        ));
        let store = LocalSecretStore::initialize_with_root_key_provider(db_pool.clone(), provider_a.clone())
            .await
            .expect("initialize provider A");
        database::upsert_provider_config(&db_pool, "local_file")
            .await
            .expect("persist provider A");
        create_test_secret(&store, "rotate/alpha", SecretKindInput::Token, "alpha-value").await;
        create_test_secret(&store, "rotate/beta", SecretKindInput::ApiKey, "beta-value").await;
        sqlx::query("UPDATE secure_store_secrets SET encrypted_key = ?2 WHERE alias = ?1")
            .bind("rotate/alpha")
            .bind("not-valid-base64")
            .execute(&db_pool)
            .await
            .expect("corrupt alpha");
        let before_rows = encrypted_rows(&db_pool).await;

        let error = LocalSecretStore::rotate_provider(db_pool.clone(), provider_a, provider_b)
            .await
            .expect_err("corrupted current record should block rotation");

        match error {
            SecretStoreRotationError::CurrentRecordUnreadable { alias, .. } => assert_eq!(alias, "rotate/alpha"),
            other => panic!("expected CurrentRecordUnreadable, got {other:?}"),
        }
        assert_eq!(encrypted_rows(&db_pool).await, before_rows);
        assert_eq!(
            database::get_provider_config(&db_pool)
                .await
                .expect("provider config")
                .expect("provider config row")
                .provider_mode,
            "local_file"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn rotate_provider_overwrites_stale_target_root_material() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let provider_a = Arc::new(TestRootKeyProvider::new(
            temp_dir.path().join("secrets-a").join("local-root.key"),
            "provider-a",
            "provider_a_kind",
        ));
        let provider_b = Arc::new(TestRootKeyProvider::new(
            temp_dir.path().join("secrets-b").join("local-root.key"),
            "provider-b",
            "provider_b_kind",
        ));
        let stale_target_key = provider_b
            .load_or_create_root_key()
            .expect("create stale target material");
        let store = LocalSecretStore::initialize_with_root_key_provider(db_pool.clone(), provider_a.clone())
            .await
            .expect("initialize provider A");
        database::upsert_provider_config(&db_pool, "local_file")
            .await
            .expect("persist provider A");
        create_test_secret(&store, "rotate/alpha", SecretKindInput::Token, "alpha-value").await;

        let rotated = LocalSecretStore::rotate_provider(db_pool.clone(), provider_a, provider_b.clone())
            .await
            .expect("rotate provider");

        let target_key = provider_b
            .load_existing_root_key()
            .expect("load target material after rotation");
        assert_ne!(target_key, stale_target_key);
        assert_secret_value(&rotated, "rotate/alpha", "alpha-value");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn rotate_provider_returns_committed_store_without_target_reload() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let provider_a = Arc::new(TestRootKeyProvider::new(
            temp_dir.path().join("secrets-a").join("local-root.key"),
            "provider-a",
            "provider_a_kind",
        ));
        let provider_b = Arc::new(RotationOnlyRootKeyProvider::new(
            temp_dir.path().join("secrets-b").join("local-root.key"),
        ));
        let store = LocalSecretStore::initialize_with_root_key_provider(db_pool.clone(), provider_a.clone())
            .await
            .expect("initialize provider A");
        database::upsert_provider_config(&db_pool, "local_file")
            .await
            .expect("persist provider A");
        create_test_secret(&store, "rotate/alpha", SecretKindInput::Token, "alpha-value").await;

        let rotated = LocalSecretStore::rotate_provider(db_pool.clone(), provider_a, provider_b.clone())
            .await
            .expect("rotate provider without target reload");

        assert_eq!(rotated.provider_metadata(), provider_b.metadata());
        assert_secret_value(&rotated, "rotate/alpha", "alpha-value");
        assert_eq!(
            database::get_provider_config(&db_pool)
                .await
                .expect("provider config")
                .expect("provider config row")
                .provider_mode,
            "local_file"
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn rotate_provider_target_failure_keeps_current_state() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let provider_a = Arc::new(TestRootKeyProvider::new(
            temp_dir.path().join("secrets-a").join("local-root.key"),
            "provider-a",
            "provider_a_kind",
        ));
        let store = LocalSecretStore::initialize_with_root_key_provider(db_pool.clone(), provider_a.clone())
            .await
            .expect("initialize provider A");
        database::upsert_provider_config(&db_pool, "local_file")
            .await
            .expect("persist provider A");
        create_test_secret(&store, "rotate/alpha", SecretKindInput::Token, "alpha-value").await;
        let before_rows = encrypted_rows(&db_pool).await;

        let error = LocalSecretStore::rotate_provider(db_pool.clone(), provider_a, Arc::new(FailingRootKeyProvider))
            .await
            .expect_err("target provider failure should block rotation");

        match error {
            SecretStoreRotationError::TargetProviderUnavailable { .. } => {}
            other => panic!("expected TargetProviderUnavailable, got {other:?}"),
        }
        assert_eq!(encrypted_rows(&db_pool).await, before_rows);
        assert_secret_value(&store, "rotate/alpha", "alpha-value");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn passphrase_rotation_rewraps_records() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let passphrase_path = temp_dir.path().join("secrets").join("passphrase-wrapped-key.json");
        let old_provider = Arc::new(crate::PassphraseRootKeyProvider::new(
            &passphrase_path,
            "old passphrase",
        ));
        let new_provider = Arc::new(crate::PassphraseRootKeyProvider::new(
            &passphrase_path,
            "new passphrase",
        ));
        let store = LocalSecretStore::initialize_with_root_key_provider(db_pool.clone(), old_provider.clone())
            .await
            .expect("initialize old passphrase store");
        database::upsert_provider_config(&db_pool, "passphrase")
            .await
            .expect("persist passphrase mode");
        create_test_secret(&store, "rotate/alpha", SecretKindInput::Token, "alpha-value").await;

        let rotated = LocalSecretStore::rotate_provider(db_pool.clone(), old_provider.clone(), new_provider.clone())
            .await
            .expect("rotate passphrase provider");

        assert!(old_provider.load_existing_root_key().is_err());
        let reopened = LocalSecretStore::initialize_with_root_key_provider(db_pool, new_provider)
            .await
            .expect("reopen with new passphrase");
        assert_secret_value(&rotated, "rotate/alpha", "alpha-value");
        assert_secret_value(&reopened, "rotate/alpha", "alpha-value");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn passphrase_rotation_persistence_failure_restores_root_material() {
        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let passphrase_path = temp_dir.path().join("secrets").join("passphrase-wrapped-key.json");
        let old_provider = Arc::new(crate::PassphraseRootKeyProvider::new(
            &passphrase_path,
            "old passphrase",
        ));
        let new_provider = Arc::new(crate::PassphraseRootKeyProvider::new(
            &passphrase_path,
            "new passphrase",
        ));
        let store = LocalSecretStore::initialize_with_root_key_provider(db_pool.clone(), old_provider.clone())
            .await
            .expect("initialize old passphrase store");
        database::upsert_provider_config(&db_pool, "passphrase")
            .await
            .expect("persist passphrase mode");
        create_test_secret(&store, "rotate/alpha", SecretKindInput::Token, "alpha-value").await;
        let before_rows = encrypted_rows(&db_pool).await;
        let before_material = std::fs::read(&passphrase_path).expect("read original passphrase material");

        sqlx::query(
            r#"
            CREATE TRIGGER fail_rotation_secret_update
            BEFORE UPDATE OF key_nonce ON secure_store_secrets
            BEGIN
                SELECT RAISE(FAIL, 'forced rotation update failure');
            END
            "#,
        )
        .execute(&db_pool)
        .await
        .expect("create failing update trigger");

        let error = LocalSecretStore::rotate_provider(db_pool.clone(), old_provider.clone(), new_provider.clone())
            .await
            .expect_err("rotation update failure should roll back root material");

        match error {
            SecretStoreRotationError::PersistenceFailed {
                action: "update rotated secret records",
                ..
            } => {}
            other => panic!("expected update persistence failure, got {other:?}"),
        }
        assert_eq!(
            std::fs::read(&passphrase_path).expect("read restored material"),
            before_material
        );
        assert_eq!(encrypted_rows(&db_pool).await, before_rows);
        assert_eq!(
            database::get_provider_config(&db_pool)
                .await
                .expect("provider config")
                .expect("provider config row")
                .provider_mode,
            "passphrase"
        );
        assert!(old_provider.load_existing_root_key().is_ok());
        assert!(new_provider.load_existing_root_key().is_err());
        assert_secret_value(&store, "rotate/alpha", "alpha-value");
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial]
    async fn passphrase_rotation_root_material_write_failure_keeps_current_material() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().expect("temp dir");
        let db_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let secrets_dir = temp_dir.path().join("secrets");
        let passphrase_path = secrets_dir.join("passphrase-wrapped-key.json");
        let old_provider = Arc::new(crate::PassphraseRootKeyProvider::new(
            &passphrase_path,
            "old passphrase",
        ));
        let new_provider = Arc::new(crate::PassphraseRootKeyProvider::new(
            &passphrase_path,
            "new passphrase",
        ));
        let store = LocalSecretStore::initialize_with_root_key_provider(db_pool.clone(), old_provider.clone())
            .await
            .expect("initialize old passphrase store");
        database::upsert_provider_config(&db_pool, "passphrase")
            .await
            .expect("persist passphrase mode");
        create_test_secret(&store, "rotate/alpha", SecretKindInput::Token, "alpha-value").await;
        let before_rows = encrypted_rows(&db_pool).await;
        let before_material = std::fs::read(&passphrase_path).expect("read original passphrase material");

        std::fs::set_permissions(&secrets_dir, std::fs::Permissions::from_mode(0o500))
            .expect("make passphrase directory read-only");
        let result =
            LocalSecretStore::rotate_provider(db_pool.clone(), old_provider.clone(), new_provider.clone()).await;
        std::fs::set_permissions(&secrets_dir, std::fs::Permissions::from_mode(0o700))
            .expect("restore passphrase directory permissions");

        let error = result.expect_err("root material write failure should block rotation");
        match error {
            SecretStoreRotationError::TargetProviderUnavailable(SecretRootKeyError::LocalStorage(_)) => {}
            other => panic!("expected target local storage failure, got {other:?}"),
        }
        assert_eq!(
            std::fs::read(&passphrase_path).expect("read preserved material"),
            before_material
        );
        assert_eq!(encrypted_rows(&db_pool).await, before_rows);
        assert!(old_provider.load_existing_root_key().is_ok());
        assert!(new_provider.load_existing_root_key().is_err());
        assert_secret_value(&store, "rotate/alpha", "alpha-value");
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct EncryptedRowSnapshot {
        alias: String,
        key_nonce: String,
        encrypted_key: String,
        encrypted_value: String,
        provider_id: String,
        provider_kind: String,
    }

    async fn encrypted_rows(pool: &sqlx::Pool<sqlx::Sqlite>) -> Vec<EncryptedRowSnapshot> {
        sqlx::query(
            "SELECT alias, key_nonce, encrypted_key, encrypted_value, provider_id, provider_kind FROM secure_store_secrets ORDER BY alias",
        )
        .fetch_all(pool)
        .await
        .expect("load encrypted rows")
        .into_iter()
        .map(|row| EncryptedRowSnapshot {
            alias: row.try_get("alias").expect("alias"),
            key_nonce: row.try_get("key_nonce").expect("key nonce"),
            encrypted_key: row.try_get("encrypted_key").expect("encrypted key"),
            encrypted_value: row.try_get("encrypted_value").expect("encrypted value"),
            provider_id: row.try_get("provider_id").expect("provider id"),
            provider_kind: row.try_get("provider_kind").expect("provider kind"),
        })
        .collect()
    }

    async fn create_test_secret(
        store: &LocalSecretStore,
        alias: &str,
        kind: SecretKindInput,
        value: &str,
    ) {
        store
            .create_secret(SecretCreateInput {
                alias: alias.to_string(),
                kind,
                value: value.to_string(),
                label: None,
                origin: None,
            })
            .await
            .expect("create test secret");
    }

    fn assert_secret_value(
        store: &LocalSecretStore,
        alias: &str,
        expected: &str,
    ) {
        let reference = SecretReference::new(alias.to_string()).expect("secret reference");
        let resolved = store.resolve_secret(&reference).expect("resolve secret");
        assert_eq!(resolved.expose(), expected);
    }
}
