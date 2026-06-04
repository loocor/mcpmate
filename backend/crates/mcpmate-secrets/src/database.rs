use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};

use crate::{
    SecretReference,
    constants::AEAD_ALGORITHM,
    crypto::{EncryptedSecret, EncryptedSecretParts},
    types::{SecretMetadataView, SecretOriginInput, SecretUsageLocationInput, SecretUsageUpsertInput, SecretUsageView},
};

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
    sqlx::query("DELETE FROM secure_store_secrets WHERE alias = ?1")
        .bind(alias)
        .execute(pool)
        .await
        .with_context(|| format!("delete secret '{alias}'"))?;
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
