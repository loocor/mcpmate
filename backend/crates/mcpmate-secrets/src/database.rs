use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite, sqlite::SqliteRow};
use tracing::warn;

use crate::{
    SecretReference,
    constants::AEAD_ALGORITHM,
    crypto::{EncryptedSecret, EncryptedSecretParts},
    types::{SecretMetadataView, SecretOriginInput, SecretUsageLocationInput, SecretUsageUpsertInput, SecretUsageView},
};

const SECURE_STORE_SECRETS_TABLE: &str = "secure_store_secrets";
const REQUIRED_SECRET_COLUMNS: &[&str] = &[
    "alias",
    "kind",
    "label",
    "origin_server_id",
    "origin_server_name",
    "origin_server_kind",
    "origin_source",
    "origin_field_group",
    "origin_field_key",
    "origin_field_index",
    "origin_field_path",
    "provider_id",
    "provider_kind",
    "version",
    "key_nonce",
    "encrypted_key",
    "nonce",
    "encrypted_value",
    "key_wrap_alg",
    "encryption_alg",
    "created_at",
    "updated_at",
];

pub(crate) struct SecretInsert<'a> {
    pub alias: &'a str,
    pub kind: &'a str,
    pub label: Option<&'a str>,
    pub origin: Option<&'a SecretOriginInput>,
    pub provider_id: &'a str,
    pub provider_kind: &'a str,
    pub encrypted: &'a EncryptedSecretParts,
}

pub(crate) struct SecretUpdate<'a> {
    pub alias: &'a str,
    pub kind: &'a str,
    pub label: Option<&'a str>,
    pub origin: Option<&'a SecretOriginInput>,
}

pub(crate) async fn ensure_schema(pool: &Pool<Sqlite>) -> Result<()> {
    ensure_secure_store_secrets_schema(pool).await?;

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

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS secure_store_password_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            password_hash TEXT NOT NULL,
            hash_salt TEXT NOT NULL,
            hash_iterations INTEGER NOT NULL DEFAULT 600000,
            protection_scope TEXT NOT NULL DEFAULT '["startup"]',
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .context("create secure_store_password_config table")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS secure_store_provider_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            provider_mode TEXT NOT NULL DEFAULT 'operating_system',
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .context("create secure_store_provider_config table")?;

    Ok(())
}

pub(crate) async fn secure_store_secret_count(pool: &Pool<Sqlite>) -> Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM secure_store_secrets")
        .fetch_one(pool)
        .await
        .context("count secure store secrets")
}

async fn ensure_secure_store_secrets_schema(pool: &Pool<Sqlite>) -> Result<()> {
    if !table_exists(pool, SECURE_STORE_SECRETS_TABLE).await? {
        create_secure_store_secrets_table(pool).await?;
        return Ok(());
    }

    let columns = table_columns(pool, SECURE_STORE_SECRETS_TABLE).await?;
    let missing_columns = REQUIRED_SECRET_COLUMNS
        .iter()
        .filter(|column| !columns.iter().any(|existing| existing == **column))
        .copied()
        .collect::<Vec<_>>();
    if missing_columns.is_empty() {
        return Ok(());
    }

    let legacy_record_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM secure_store_secrets")
        .fetch_one(pool)
        .await
        .context("count legacy secure store secrets")?;

    if legacy_record_count == 0 {
        sqlx::query("DROP TABLE secure_store_secrets")
            .execute(pool)
            .await
            .context("drop empty legacy secure_store_secrets table")?;
        create_secure_store_secrets_table(pool).await?;
        return Ok(());
    }

    bail!(
        "outdated secure_store_secrets schema contains {legacy_record_count} legacy secret record(s); missing column(s): {}; reset the secure store data or run an explicit migration tool before using encrypted secrets",
        missing_columns.join(", ")
    );
}

async fn table_exists(
    pool: &Pool<Sqlite>,
    table_name: &str,
) -> Result<bool> {
    let exists: Option<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1")
            .bind(table_name)
            .fetch_optional(pool)
            .await
            .with_context(|| format!("inspect sqlite table '{table_name}'"))?;
    Ok(exists.is_some())
}

async fn table_columns(
    pool: &Pool<Sqlite>,
    table_name: &str,
) -> Result<Vec<String>> {
    let rows = sqlx::query(&format!("PRAGMA table_info({table_name})"))
        .fetch_all(pool)
        .await
        .with_context(|| format!("inspect sqlite table columns for '{table_name}'"))?;
    rows.into_iter()
        .map(|row| row.try_get("name").context("read sqlite table column name"))
        .collect()
}

async fn create_secure_store_secrets_table(pool: &Pool<Sqlite>) -> Result<()> {
    let secrets_schema = format!(
        r#"
        CREATE TABLE IF NOT EXISTS secure_store_secrets (
            alias TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            label TEXT,
            origin_server_id TEXT,
            origin_server_name TEXT,
            origin_server_kind TEXT,
            origin_source TEXT,
            origin_field_group TEXT,
            origin_field_key TEXT,
            origin_field_index INTEGER,
            origin_field_path TEXT,
            provider_id TEXT NOT NULL,
            provider_kind TEXT NOT NULL,
            version INTEGER NOT NULL,
            key_nonce TEXT NOT NULL,
            encrypted_key TEXT NOT NULL,
            nonce TEXT NOT NULL,
            encrypted_value TEXT NOT NULL,
            key_wrap_alg TEXT NOT NULL DEFAULT '{AEAD_ALGORITHM}',
            encryption_alg TEXT NOT NULL DEFAULT '{AEAD_ALGORITHM}',
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    );
    sqlx::query(&secrets_schema)
        .execute(pool)
        .await
        .context("create secure_store_secrets table")?;
    Ok(())
}

pub(crate) async fn insert_secret(
    pool: &Pool<Sqlite>,
    input: SecretInsert<'_>,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO secure_store_secrets (
            alias,
            kind,
            label,
            origin_server_id,
            origin_server_name,
            origin_server_kind,
            origin_source,
            origin_field_group,
            origin_field_key,
            origin_field_index,
            origin_field_path,
            provider_id,
            provider_kind,
            version,
            key_nonce,
            encrypted_key,
            nonce,
            encrypted_value,
            key_wrap_alg,
            encryption_alg
        )
        VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
            ?11, ?12, ?13, 1, ?14, ?15, ?16, ?17, ?18, ?19
        )
        "#,
    )
    .bind(input.alias)
    .bind(input.kind)
    .bind(input.label)
    .bind(input.origin.and_then(|origin| origin.server_id.as_deref()))
    .bind(input.origin.and_then(|origin| origin.server_name.as_deref()))
    .bind(input.origin.and_then(|origin| origin.server_kind.as_deref()))
    .bind(input.origin.and_then(|origin| origin.source.as_deref()))
    .bind(input.origin.and_then(|origin| origin.field_group.as_deref()))
    .bind(input.origin.and_then(|origin| origin.field_key.as_deref()))
    .bind(input.origin.and_then(|origin| origin.field_index))
    .bind(input.origin.and_then(|origin| origin.field_path.as_deref()))
    .bind(input.provider_id)
    .bind(input.provider_kind)
    .bind(&input.encrypted.key_nonce)
    .bind(&input.encrypted.encrypted_key)
    .bind(&input.encrypted.nonce)
    .bind(&input.encrypted.encrypted_value)
    .bind(AEAD_ALGORITHM)
    .bind(AEAD_ALGORITHM)
    .execute(pool)
    .await
    .with_context(|| format!("create secret '{}'", input.alias))?;

    Ok(())
}

pub(crate) async fn update_secret_with_value(
    pool: &Pool<Sqlite>,
    input: SecretUpdate<'_>,
    encrypted: &EncryptedSecretParts,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE secure_store_secrets
        SET kind = ?2,
            label = ?3,
            origin_server_id = ?4,
            origin_server_name = ?5,
            origin_server_kind = ?6,
            origin_source = ?7,
            origin_field_group = ?8,
            origin_field_key = ?9,
            origin_field_index = ?10,
            origin_field_path = ?11,
            version = version + 1,
            key_nonce = ?12,
            encrypted_key = ?13,
            nonce = ?14,
            encrypted_value = ?15,
            key_wrap_alg = ?16,
            encryption_alg = ?17,
            updated_at = CURRENT_TIMESTAMP
        WHERE alias = ?1
        "#,
    )
    .bind(input.alias)
    .bind(input.kind)
    .bind(input.label)
    .bind(input.origin.and_then(|origin| origin.server_id.as_deref()))
    .bind(input.origin.and_then(|origin| origin.server_name.as_deref()))
    .bind(input.origin.and_then(|origin| origin.server_kind.as_deref()))
    .bind(input.origin.and_then(|origin| origin.source.as_deref()))
    .bind(input.origin.and_then(|origin| origin.field_group.as_deref()))
    .bind(input.origin.and_then(|origin| origin.field_key.as_deref()))
    .bind(input.origin.and_then(|origin| origin.field_index))
    .bind(input.origin.and_then(|origin| origin.field_path.as_deref()))
    .bind(&encrypted.key_nonce)
    .bind(&encrypted.encrypted_key)
    .bind(&encrypted.nonce)
    .bind(&encrypted.encrypted_value)
    .bind(AEAD_ALGORITHM)
    .bind(AEAD_ALGORITHM)
    .execute(pool)
    .await
    .with_context(|| format!("update secret '{}'", input.alias))?;

    Ok(())
}

pub(crate) async fn update_secret_metadata(
    pool: &Pool<Sqlite>,
    input: SecretUpdate<'_>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE secure_store_secrets
        SET kind = ?2,
            label = ?3,
            origin_server_id = ?4,
            origin_server_name = ?5,
            origin_server_kind = ?6,
            origin_source = ?7,
            origin_field_group = ?8,
            origin_field_key = ?9,
            origin_field_index = ?10,
            origin_field_path = ?11,
            updated_at = CURRENT_TIMESTAMP
        WHERE alias = ?1
        "#,
    )
    .bind(input.alias)
    .bind(input.kind)
    .bind(input.label)
    .bind(input.origin.and_then(|origin| origin.server_id.as_deref()))
    .bind(input.origin.and_then(|origin| origin.server_name.as_deref()))
    .bind(input.origin.and_then(|origin| origin.server_kind.as_deref()))
    .bind(input.origin.and_then(|origin| origin.source.as_deref()))
    .bind(input.origin.and_then(|origin| origin.field_group.as_deref()))
    .bind(input.origin.and_then(|origin| origin.field_key.as_deref()))
    .bind(input.origin.and_then(|origin| origin.field_index))
    .bind(input.origin.and_then(|origin| origin.field_path.as_deref()))
    .execute(pool)
    .await
    .with_context(|| format!("update secret metadata '{}'", input.alias))?;

    Ok(())
}

pub(crate) async fn get_secret_metadata(
    pool: &Pool<Sqlite>,
    alias: &str,
) -> Result<SecretMetadataView> {
    let row = sqlx::query(
        r#"
        SELECT
            s.alias,
            s.kind,
            s.label,
            s.origin_server_id,
            s.origin_server_name,
            s.origin_server_kind,
            s.origin_source,
            s.origin_field_group,
            s.origin_field_key,
            s.origin_field_index,
            s.origin_field_path,
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
    .fetch_optional(pool)
    .await
    .with_context(|| format!("load secret metadata '{alias}'"))?
    .ok_or_else(|| anyhow::anyhow!("Secret '{alias}' was not found"))?;

    secret_metadata_from_row(&row)
}

pub(crate) async fn list_secret_metadata(pool: &Pool<Sqlite>) -> Result<Vec<SecretMetadataView>> {
    let rows = sqlx::query(
        r#"
        SELECT
            s.alias,
            s.kind,
            s.label,
            s.origin_server_id,
            s.origin_server_name,
            s.origin_server_kind,
            s.origin_source,
            s.origin_field_group,
            s.origin_field_key,
            s.origin_field_index,
            s.origin_field_path,
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
    .fetch_all(pool)
    .await
    .context("list secret metadata")?;

    rows.iter().map(secret_metadata_from_row).collect()
}

pub(crate) async fn delete_secret(
    pool: &Pool<Sqlite>,
    alias: &str,
) -> Result<()> {
    let result = sqlx::query("DELETE FROM secure_store_secrets WHERE alias = ?1")
        .bind(alias)
        .execute(pool)
        .await
        .with_context(|| format!("delete secret '{alias}'"))?;

    if result.rows_affected() == 0 {
        bail!("Secret '{alias}' was not found");
    }

    Ok(())
}

pub(crate) async fn upsert_usage(
    pool: &Pool<Sqlite>,
    input: &SecretUsageUpsertInput,
) -> Result<()> {
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
        ON CONFLICT(id)
        DO UPDATE SET updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(id)
    .bind(reference.alias())
    .bind(&input.server_id)
    .bind(location_kind)
    .bind(location_name)
    .bind(location_index)
    .execute(pool)
    .await
    .with_context(|| format!("upsert usage for secret '{}'", reference.alias()))?;

    Ok(())
}

pub(crate) async fn replace_server_usages(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<()> {
    let mut tx = pool.begin().await.context("begin secret usage replacement")?;
    sqlx::query("DELETE FROM secure_store_usages WHERE server_id = ?1")
        .bind(server_id)
        .execute(&mut *tx)
        .await
        .with_context(|| format!("delete secret usages for server '{server_id}'"))?;
    tx.commit().await.context("commit secret usage replacement")?;
    Ok(())
}

pub(crate) async fn list_usages(
    pool: &Pool<Sqlite>,
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
    .fetch_all(pool)
    .await
    .with_context(|| format!("list usages for secret '{alias}'"))?;

    parse_usage_rows(rows)
}

pub(crate) async fn list_all_usages(pool: &Pool<Sqlite>) -> Result<Vec<SecretUsageView>> {
    let rows = sqlx::query(
        r#"
        SELECT alias, server_id, location_kind, location_name, location_index
        FROM secure_store_usages
        ORDER BY alias ASC, server_id ASC, location_kind ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("list all secret usages")?;

    parse_usage_rows(rows)
}

pub(crate) async fn count_unsupported_usages_by_alias(pool: &Pool<Sqlite>) -> Result<HashMap<String, u64>> {
    let rows = sqlx::query(
        r#"
        SELECT alias, location_kind, location_name, location_index
        FROM secure_store_usages
        ORDER BY alias ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .context("list secret usage locations")?;

    let mut counts = HashMap::new();
    for row in rows {
        if let Some(alias) = unsupported_usage_alias(&row)? {
            *counts.entry(alias).or_insert(0) += 1;
        }
    }
    Ok(counts)
}

pub(crate) async fn count_unsupported_usages_for_alias(
    pool: &Pool<Sqlite>,
    alias: &str,
) -> Result<u64> {
    let rows = sqlx::query(
        r#"
        SELECT alias, location_kind, location_name, location_index
        FROM secure_store_usages
        WHERE alias = ?1
        ORDER BY alias ASC
        "#,
    )
    .bind(alias)
    .fetch_all(pool)
    .await
    .with_context(|| format!("list secret usage locations for secret '{alias}'"))?;

    let mut count = 0;
    for row in rows {
        if unsupported_usage_alias(&row)?.is_some() {
            count += 1;
        }
    }
    Ok(count)
}

fn parse_usage_rows(rows: Vec<SqliteRow>) -> Result<Vec<SecretUsageView>> {
    let mut usages = Vec::with_capacity(rows.len());
    for row in rows {
        let alias: String = row.try_get("alias")?;
        let server_id: String = row.try_get("server_id")?;
        let location_kind: String = row.try_get("location_kind")?;
        let location_name: Option<String> = row.try_get("location_name")?;
        let location_index: Option<i64> = row.try_get("location_index")?;
        let location = match SecretUsageLocationInput::from_parts(&location_kind, location_name, location_index) {
            Ok(location) => location,
            Err(error) => {
                warn!(
                    alias = %alias,
                    server_id = %server_id,
                    location_kind = %location_kind,
                    error = %error,
                    "Skipping unsupported secret usage location"
                );
                continue;
            }
        };
        usages.push(SecretUsageView {
            alias,
            server_id,
            location,
        });
    }
    Ok(usages)
}

fn unsupported_usage_alias(row: &SqliteRow) -> Result<Option<String>> {
    let alias: String = row.try_get("alias")?;
    let location_kind: String = row.try_get("location_kind")?;
    let location_name: Option<String> = row.try_get("location_name")?;
    let location_index: Option<i64> = row.try_get("location_index")?;

    if SecretUsageLocationInput::from_parts(&location_kind, location_name, location_index).is_err() {
        return Ok(Some(alias));
    }

    Ok(None)
}

pub(crate) async fn load_encrypted_secrets(pool: &Pool<Sqlite>) -> Result<Vec<EncryptedSecret>> {
    let rows = sqlx::query("SELECT alias, key_nonce, encrypted_key, nonce, encrypted_value FROM secure_store_secrets")
        .fetch_all(pool)
        .await
        .context("load encrypted secret cache")?;

    rows.into_iter()
        .map(|row| {
            let alias: String = row.try_get("alias")?;
            Ok(EncryptedSecret {
                alias,
                key_nonce: row.try_get("key_nonce")?,
                encrypted_key: row.try_get("encrypted_key")?,
                nonce: row.try_get("nonce")?,
                encrypted_value: row.try_get("encrypted_value")?,
            })
        })
        .collect()
}

// ── Password Config ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PasswordConfigRow {
    pub password_hash: String,
    pub hash_salt: String,
    pub hash_iterations: i64,
    pub protection_scope: String,
}

pub async fn get_password_config(pool: &Pool<Sqlite>) -> Result<Option<PasswordConfigRow>> {
    let row = sqlx::query(
        "SELECT password_hash, hash_salt, hash_iterations, protection_scope FROM secure_store_password_config WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .context("load password config")?;

    Ok(row.map(|r| PasswordConfigRow {
        password_hash: r.try_get("password_hash").unwrap_or_default(),
        hash_salt: r.try_get("hash_salt").unwrap_or_default(),
        hash_iterations: r.try_get("hash_iterations").unwrap_or(600_000),
        protection_scope: r.try_get("protection_scope").unwrap_or_else(|_| "[]".to_string()),
    }))
}

pub async fn upsert_password_config(
    pool: &Pool<Sqlite>,
    password_hash: &str,
    hash_salt: &str,
    hash_iterations: i64,
    protection_scope: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO secure_store_password_config (id, password_hash, hash_salt, hash_iterations, protection_scope, updated_at)
        VALUES (1, ?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
            password_hash = ?1,
            hash_salt = ?2,
            hash_iterations = ?3,
            protection_scope = ?4,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(password_hash)
    .bind(hash_salt)
    .bind(hash_iterations)
    .bind(protection_scope)
    .execute(pool)
    .await
    .context("upsert password config")?;

    Ok(())
}

pub async fn delete_password_config(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query("DELETE FROM secure_store_password_config WHERE id = 1")
        .execute(pool)
        .await
        .context("delete password config")?;
    Ok(())
}

// ── Provider Config ────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfigRow {
    pub provider_mode: String,
}

pub async fn get_provider_config(pool: &Pool<Sqlite>) -> Result<Option<ProviderConfigRow>> {
    let row = sqlx::query("SELECT provider_mode FROM secure_store_provider_config WHERE id = 1")
        .fetch_optional(pool)
        .await
        .context("load provider config")?;

    Ok(row.map(|r| ProviderConfigRow {
        provider_mode: r
            .try_get("provider_mode")
            .unwrap_or_else(|_| "operating_system".to_string()),
    }))
}

pub async fn upsert_provider_config(
    pool: &Pool<Sqlite>,
    provider_mode: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO secure_store_provider_config (id, provider_mode, updated_at)
        VALUES (1, ?1, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
            provider_mode = ?1,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(provider_mode)
    .execute(pool)
    .await
    .context("upsert provider config")?;

    Ok(())
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
        origin: secret_origin_from_row(row)?,
        provider_id: row.try_get("provider_id")?,
        provider_kind: row.try_get("provider_kind")?,
        version: version.max(0) as u64,
        used_by_count: used_by_count.max(0) as u64,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn secret_origin_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Option<SecretOriginInput>> {
    let origin = SecretOriginInput {
        server_id: row.try_get("origin_server_id")?,
        server_name: row.try_get("origin_server_name")?,
        server_kind: row.try_get("origin_server_kind")?,
        source: row.try_get("origin_source")?,
        field_group: row.try_get("origin_field_group")?,
        field_key: row.try_get("origin_field_key")?,
        field_index: row.try_get("origin_field_index")?,
        field_path: row.try_get("origin_field_path")?,
    };
    Ok((!origin.is_empty()).then_some(origin))
}

#[cfg(test)]
mod tests {
    use super::*;

    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn ensure_schema_rebuilds_empty_legacy_secret_table() {
        let pool = sqlite_pool().await;
        create_legacy_secret_table(&pool).await;

        ensure_schema(&pool).await.expect("ensure schema");

        let columns = table_columns(&pool, "secure_store_secrets")
            .await
            .expect("table columns");
        assert!(columns.iter().any(|column| column == "key_nonce"));
        assert!(columns.iter().any(|column| column == "encrypted_key"));
        assert!(columns.iter().any(|column| column == "origin_server_id"));
    }

    #[tokio::test]
    async fn ensure_schema_rejects_nonempty_legacy_secret_table() {
        let pool = sqlite_pool().await;
        create_legacy_secret_table(&pool).await;
        sqlx::query(
            r#"
            INSERT INTO secure_store_secrets (
                alias,
                kind,
                label,
                provider_id,
                provider_kind,
                version,
                nonce,
                encrypted_value
            )
            VALUES ('context7-token', 'token', NULL, 'local-encrypted-vault', 'local_encrypted_vault', 1, 'nonce', 'value')
            "#,
        )
        .execute(&pool)
        .await
        .expect("insert legacy secret");

        let err = ensure_schema(&pool)
            .await
            .expect_err("non-empty legacy schema should fail closed");
        let message = err.to_string();

        assert!(message.contains("outdated secure_store_secrets schema"));
        assert!(message.contains("1 legacy secret record"));
    }

    async fn sqlite_pool() -> Pool<Sqlite> {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool")
    }

    async fn create_legacy_secret_table(pool: &Pool<Sqlite>) {
        sqlx::query(
            r#"
            CREATE TABLE secure_store_secrets (
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
        .expect("create legacy secrets table");
    }
}
