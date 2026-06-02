use std::{
    collections::HashMap,
    fmt,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::RwLock,
};

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use mcpmate_secrets::{SecretError, SecretReference, SecretResolver, SecretValue};
use ring::{aead, rand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};

const LOCAL_PROVIDER_ID: &str = "local-encrypted-vault";
const LOCAL_PROVIDER_KIND: &str = "local_encrypted_vault";
const ROOT_KEY_ENV: &str = "MCPMATE_SECRETS_LOCAL_KEY";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretKindInput {
    Generic,
    Token,
    ApiKey,
    Password,
    OAuthAccessToken,
    OAuthRefreshToken,
    UrlCredential,
    HeaderValue,
}

impl SecretKindInput {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Generic => "generic",
            Self::Token => "token",
            Self::ApiKey => "api_key",
            Self::Password => "password",
            Self::OAuthAccessToken => "oauth_access_token",
            Self::OAuthRefreshToken => "oauth_refresh_token",
            Self::UrlCredential => "url_credential",
            Self::HeaderValue => "header_value",
        }
    }
}

impl fmt::Display for SecretKindInput {
    fn fmt(
        &self,
        formatter: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for SecretKindInput {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "generic" => Ok(Self::Generic),
            "token" => Ok(Self::Token),
            "api_key" => Ok(Self::ApiKey),
            "password" => Ok(Self::Password),
            "oauth_access_token" => Ok(Self::OAuthAccessToken),
            "oauth_refresh_token" => Ok(Self::OAuthRefreshToken),
            "url_credential" => Ok(Self::UrlCredential),
            "header_value" => Ok(Self::HeaderValue),
            other => Err(anyhow::anyhow!("Unsupported secret kind '{other}'")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecretCreateInput {
    pub alias: String,
    pub kind: SecretKindInput,
    pub value: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SecretUpdateInput {
    pub alias: String,
    pub kind: Option<SecretKindInput>,
    pub value: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretUsageLocationInput {
    StdioCommand,
    StdioArgument { index: usize },
    StdioEnv { name: String },
    StreamableHttpUrl,
    StreamableHttpHeader { name: String },
    OAuthToken,
}

impl SecretUsageLocationInput {
    fn parts(&self) -> (&'static str, Option<&str>, Option<i64>) {
        match self {
            Self::StdioCommand => ("stdio_command", None, None),
            Self::StdioArgument { index } => ("stdio_argument", None, Some(*index as i64)),
            Self::StdioEnv { name } => ("stdio_env", Some(name.as_str()), None),
            Self::StreamableHttpUrl => ("streamable_http_url", None, None),
            Self::StreamableHttpHeader { name } => ("streamable_http_header", Some(name.as_str()), None),
            Self::OAuthToken => ("oauth_token", None, None),
        }
    }

    fn from_parts(
        kind: &str,
        name: Option<String>,
        index: Option<i64>,
    ) -> Result<Self> {
        match kind {
            "stdio_command" => Ok(Self::StdioCommand),
            "stdio_argument" => Ok(Self::StdioArgument {
                index: index.unwrap_or_default() as usize,
            }),
            "stdio_env" => Ok(Self::StdioEnv {
                name: name.unwrap_or_default(),
            }),
            "streamable_http_url" => Ok(Self::StreamableHttpUrl),
            "streamable_http_header" => Ok(Self::StreamableHttpHeader {
                name: name.unwrap_or_default(),
            }),
            "oauth_token" => Ok(Self::OAuthToken),
            other => Err(anyhow::anyhow!("Unsupported secret usage location '{other}'")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecretUsageUpsertInput {
    pub alias: String,
    pub server_id: String,
    pub location: SecretUsageLocationInput,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecretMetadataView {
    pub alias: String,
    pub placeholder: String,
    pub kind: String,
    pub label: Option<String>,
    pub provider_id: String,
    pub provider_kind: String,
    pub version: u64,
    pub used_by_count: u64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SecretUsageView {
    pub alias: String,
    pub server_id: String,
    pub location: SecretUsageLocationInput,
}

#[derive(Debug, Clone)]
struct EncryptedSecret {
    alias: String,
    nonce: String,
    encrypted_value: String,
}

pub struct LocalSecretStore {
    pool: Pool<Sqlite>,
    key: [u8; 32],
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
        Self::ensure_schema(&pool).await?;
        let key = load_local_key_material()?;
        let store = Self {
            pool,
            key,
            encrypted_cache: RwLock::new(HashMap::new()),
        };
        store.reload_cache().await?;
        Ok(store)
    }

    pub fn pool(&self) -> Pool<Sqlite> {
        self.pool.clone()
    }

    pub async fn ensure_schema(pool: &Pool<Sqlite>) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS secure_store_secrets (
                alias TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                label TEXT,
                provider_id TEXT NOT NULL,
                provider_kind TEXT NOT NULL,
                version INTEGER NOT NULL,
                nonce TEXT NOT NULL,
                encrypted_value TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(pool)
        .await
        .context("create secure_store_secrets table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS secure_store_usages (
                id TEXT PRIMARY KEY,
                alias TEXT NOT NULL,
                server_id TEXT NOT NULL,
                location_kind TEXT NOT NULL,
                location_name TEXT,
                location_index INTEGER,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (alias) REFERENCES secure_store_secrets (alias) ON DELETE CASCADE,
                UNIQUE(alias, server_id, location_kind, location_name, location_index)
            )
            "#,
        )
        .execute(pool)
        .await
        .context("create secure_store_usages table")?;

        Ok(())
    }

    pub async fn create_secret(
        &self,
        input: SecretCreateInput,
    ) -> Result<SecretMetadataView> {
        let reference = SecretReference::new(input.alias.clone()).context("invalid secret alias")?;
        let (nonce, encrypted_value) = self.encrypt(reference.alias(), &input.value)?;

        sqlx::query(
            r#"
            INSERT INTO secure_store_secrets (
                alias, kind, label, provider_id, provider_kind, version, nonce, encrypted_value
            )
            VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)
            "#,
        )
        .bind(reference.alias())
        .bind(input.kind.as_str())
        .bind(input.label.as_deref())
        .bind(LOCAL_PROVIDER_ID)
        .bind(LOCAL_PROVIDER_KIND)
        .bind(&nonce)
        .bind(&encrypted_value)
        .execute(&self.pool)
        .await
        .with_context(|| format!("create secret '{}'", reference.alias()))?;

        self.cache_secret(reference.alias(), nonce, encrypted_value)?;
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

        if let Some(value) = input.value {
            let (nonce, encrypted_value) = self.encrypt(reference.alias(), &value)?;
            sqlx::query(
                r#"
                UPDATE secure_store_secrets
                SET kind = ?2,
                    label = ?3,
                    version = version + 1,
                    nonce = ?4,
                    encrypted_value = ?5,
                    updated_at = CURRENT_TIMESTAMP
                WHERE alias = ?1
                "#,
            )
            .bind(reference.alias())
            .bind(&next_kind)
            .bind(next_label.as_deref())
            .bind(&nonce)
            .bind(&encrypted_value)
            .execute(&self.pool)
            .await
            .with_context(|| format!("update secret '{}'", reference.alias()))?;
            self.cache_secret(reference.alias(), nonce, encrypted_value)?;
        } else {
            sqlx::query(
                r#"
                UPDATE secure_store_secrets
                SET kind = ?2,
                    label = ?3,
                    updated_at = CURRENT_TIMESTAMP
                WHERE alias = ?1
                "#,
            )
            .bind(reference.alias())
            .bind(&next_kind)
            .bind(next_label.as_deref())
            .execute(&self.pool)
            .await
            .with_context(|| format!("update secret metadata '{}'", reference.alias()))?;
        }

        self.get_secret_metadata(reference.alias()).await
    }

    pub async fn get_secret_metadata(
        &self,
        alias: &str,
    ) -> Result<SecretMetadataView> {
        let row = sqlx::query(
            r#"
            SELECT
                s.alias,
                s.kind,
                s.label,
                s.provider_id,
                s.provider_kind,
                s.version,
                s.created_at,
                s.updated_at,
                COUNT(u.id) AS used_by_count
            FROM secure_store_secrets s
            LEFT JOIN secure_store_usages u ON u.alias = s.alias
            WHERE s.alias = ?1
            GROUP BY s.alias
            "#,
        )
        .bind(alias)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("load secret metadata '{alias}'"))?
        .ok_or_else(|| anyhow::anyhow!("Secret '{alias}' was not found"))?;

        secret_metadata_from_row(&row)
    }

    pub async fn list_secret_metadata(&self) -> Result<Vec<SecretMetadataView>> {
        let rows = sqlx::query(
            r#"
            SELECT
                s.alias,
                s.kind,
                s.label,
                s.provider_id,
                s.provider_kind,
                s.version,
                s.created_at,
                s.updated_at,
                COUNT(u.id) AS used_by_count
            FROM secure_store_secrets s
            LEFT JOIN secure_store_usages u ON u.alias = s.alias
            GROUP BY s.alias
            ORDER BY s.alias ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("list secret metadata")?;

        rows.iter().map(secret_metadata_from_row).collect()
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

        sqlx::query("DELETE FROM secure_store_secrets WHERE alias = ?1")
            .bind(alias)
            .execute(&self.pool)
            .await
            .with_context(|| format!("delete secret '{alias}'"))?;
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
        let (location_kind, location_name, location_index) = input.location.parts();
        let id_material = format!(
            "{}|{}|{}|{}|{}",
            reference.alias(),
            input.server_id,
            location_kind,
            location_name.unwrap_or_default(),
            location_index.unwrap_or_default()
        );
        let id = format!("{:x}", Sha256::digest(id_material.as_bytes()));

        sqlx::query(
            r#"
            INSERT INTO secure_store_usages (
                id, alias, server_id, location_kind, location_name, location_index
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(alias, server_id, location_kind, location_name, location_index)
            DO UPDATE SET updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(id)
        .bind(reference.alias())
        .bind(&input.server_id)
        .bind(location_kind)
        .bind(location_name)
        .bind(location_index)
        .execute(&self.pool)
        .await
        .with_context(|| format!("upsert usage for secret '{}'", reference.alias()))?;

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
        let mut tx = self.pool.begin().await.context("begin secret usage replacement")?;
        sqlx::query("DELETE FROM secure_store_usages WHERE server_id = ?1")
            .bind(server_id)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("delete secret usages for server '{server_id}'"))?;
        tx.commit().await.context("commit secret usage replacement")?;

        for usage in usages {
            self.upsert_usage(usage).await?;
        }

        Ok(())
    }

    pub async fn list_usages(
        &self,
        alias: &str,
    ) -> Result<Vec<SecretUsageView>> {
        let rows = sqlx::query(
            r#"
            SELECT alias, server_id, location_kind, location_name, location_index
            FROM secure_store_usages
            WHERE alias = ?1
            ORDER BY server_id ASC, location_kind ASC
            "#,
        )
        .bind(alias)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("list usages for secret '{alias}'"))?;

        rows.iter()
            .map(|row| {
                let alias: String = row.try_get("alias")?;
                let server_id: String = row.try_get("server_id")?;
                let location_kind: String = row.try_get("location_kind")?;
                let location_name: Option<String> = row.try_get("location_name")?;
                let location_index: Option<i64> = row.try_get("location_index")?;
                Ok(SecretUsageView {
                    alias,
                    server_id,
                    location: SecretUsageLocationInput::from_parts(&location_kind, location_name, location_index)?,
                })
            })
            .collect()
    }

    async fn reload_cache(&self) -> Result<()> {
        let rows = sqlx::query("SELECT alias, nonce, encrypted_value FROM secure_store_secrets")
            .fetch_all(&self.pool)
            .await
            .context("load encrypted secret cache")?;
        let mut cache = self
            .encrypted_cache
            .write()
            .map_err(|_| anyhow::anyhow!("secret cache lock poisoned"))?;
        cache.clear();
        for row in rows {
            let alias: String = row.try_get("alias")?;
            cache.insert(
                alias.clone(),
                EncryptedSecret {
                    alias,
                    nonce: row.try_get("nonce")?,
                    encrypted_value: row.try_get("encrypted_value")?,
                },
            );
        }
        Ok(())
    }

    fn cache_secret(
        &self,
        alias: &str,
        nonce: String,
        encrypted_value: String,
    ) -> Result<()> {
        self.encrypted_cache
            .write()
            .map_err(|_| anyhow::anyhow!("secret cache lock poisoned"))?
            .insert(
                alias.to_string(),
                EncryptedSecret {
                    alias: alias.to_string(),
                    nonce,
                    encrypted_value,
                },
            );
        Ok(())
    }

    fn encrypt(
        &self,
        alias: &str,
        plaintext: &str,
    ) -> Result<(String, String)> {
        let rng = rand::SystemRandom::new();
        let mut nonce_bytes = [0_u8; 12];
        rand::SecureRandom::fill(&rng, &mut nonce_bytes).map_err(|_| anyhow::anyhow!("generate secret nonce"))?;

        let key = aead_key(&self.key)?;
        let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
        let mut in_out = plaintext.as_bytes().to_vec();
        key.seal_in_place_append_tag(nonce, aead::Aad::from(alias.as_bytes()), &mut in_out)
            .map_err(|_| anyhow::anyhow!("encrypt secret value"))?;

        Ok((STANDARD.encode(nonce_bytes), STANDARD.encode(in_out)))
    }

    fn decrypt_secret(
        &self,
        encrypted: &EncryptedSecret,
    ) -> Result<SecretValue, SecretError> {
        let nonce_bytes = STANDARD
            .decode(&encrypted.nonce)
            .map_err(|err| SecretError::InvalidMetadata(format!("invalid secret nonce: {err}")))?;
        let nonce_array: [u8; 12] = nonce_bytes
            .try_into()
            .map_err(|_| SecretError::InvalidMetadata("invalid secret nonce length".to_string()))?;
        let mut in_out = STANDARD
            .decode(&encrypted.encrypted_value)
            .map_err(|err| SecretError::InvalidMetadata(format!("invalid encrypted secret value: {err}")))?;
        let key = aead_key(&self.key).map_err(|err| SecretError::InvalidMetadata(err.to_string()))?;
        let plaintext = key
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce_array),
                aead::Aad::from(encrypted.alias.as_bytes()),
                &mut in_out,
            )
            .map_err(|_| SecretError::ProviderUnavailable)?;
        let value = std::str::from_utf8(plaintext)
            .map_err(|err| SecretError::InvalidMetadata(format!("secret value is not utf-8: {err}")))?;
        Ok(SecretValue::new(value.to_string()))
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
        self.decrypt_secret(&encrypted)
    }
}

fn secret_metadata_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<SecretMetadataView> {
    let alias: String = row.try_get("alias")?;
    let version: i64 = row.try_get("version")?;
    let used_by_count: i64 = row.try_get("used_by_count")?;
    Ok(SecretMetadataView {
        placeholder: SecretReference::new(alias.clone())?.placeholder(),
        alias,
        kind: row.try_get("kind")?,
        label: row.try_get("label")?,
        provider_id: row.try_get("provider_id")?,
        provider_kind: row.try_get("provider_kind")?,
        version: version.max(0) as u64,
        used_by_count: used_by_count.max(0) as u64,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn aead_key(raw_key: &[u8; 32]) -> Result<aead::LessSafeKey> {
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, raw_key).map_err(|_| anyhow::anyhow!("build AEAD key"))?;
    Ok(aead::LessSafeKey::new(unbound))
}

fn load_local_key_material() -> Result<[u8; 32]> {
    if let Ok(value) = std::env::var(ROOT_KEY_ENV) {
        if !value.trim().is_empty() {
            return Ok(derive_key(value.as_bytes()));
        }
    }

    let key_path = local_key_path()?;
    if key_path.exists() {
        let mut file = OpenOptions::new()
            .read(true)
            .open(&key_path)
            .with_context(|| format!("open local secret root key {}", key_path.display()))?;
        let mut encoded = String::new();
        file.read_to_string(&mut encoded)
            .with_context(|| format!("read local secret root key {}", key_path.display()))?;
        let decoded = STANDARD
            .decode(encoded.trim())
            .context("decode local secret root key")?;
        return Ok(derive_key(&decoded));
    }

    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create secret key directory {}", parent.display()))?;
    }
    let rng = rand::SystemRandom::new();
    let mut root = [0_u8; 32];
    rand::SecureRandom::fill(&rng, &mut root).map_err(|_| anyhow::anyhow!("generate local secret root key"))?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&key_path)
        .with_context(|| format!("create local secret root key {}", key_path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| format!("restrict local secret root key {}", key_path.display()))?;
    }
    file.write_all(STANDARD.encode(root).as_bytes())
        .with_context(|| format!("write local secret root key {}", key_path.display()))?;
    Ok(derive_key(&root))
}

fn local_key_path() -> Result<PathBuf> {
    Ok(crate::common::paths::global_paths()
        .base_dir()
        .join("secrets")
        .join("local-root.key"))
}

fn derive_key(material: &[u8]) -> [u8; 32] {
    Sha256::digest(material).into()
}
