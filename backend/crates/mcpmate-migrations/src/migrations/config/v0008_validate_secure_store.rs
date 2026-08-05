use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use sqlx::{Sqlite, Transaction};

use super::{
    super::{Migration, MigrationStep},
    SECURE_STORE_SCHEMA,
};

pub(super) fn migration() -> Migration {
    Migration::rust(
        8,
        "validate legacy secure store storage",
        &[include_str!("v0008_validate_secure_store.rs"), SECURE_STORE_SCHEMA],
        ValidateSecureStoreSchema,
    )
}

struct ValidateSecureStoreSchema;

#[async_trait]
impl MigrationStep for ValidateSecureStoreSchema {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        if secure_store_schema_is_current(transaction).await? {
            return Ok(());
        }
        let count = secure_store_record_count(transaction).await?;
        if count != 0 {
            bail!(
                "outdated secure store schema contains {count} legacy record(s); it cannot be safely upgraded without preserving its security constraints"
            );
        }
        rebuild_empty_secure_store(transaction).await?;
        Ok(())
    }
}

const SECURE_STORE_TABLES: &[(&str, &[&str])] = &[
    (
        "secure_store_secrets",
        &[
            "alias|TEXT|0||1",
            "kind|TEXT|1||0",
            "label|TEXT|0||0",
            "origin_server_id|TEXT|0||0",
            "origin_server_name|TEXT|0||0",
            "origin_server_kind|TEXT|0||0",
            "origin_source|TEXT|0||0",
            "origin_field_group|TEXT|0||0",
            "origin_field_key|TEXT|0||0",
            "origin_field_index|INTEGER|0||0",
            "origin_field_path|TEXT|0||0",
            "provider_id|TEXT|1||0",
            "provider_kind|TEXT|1||0",
            "version|INTEGER|1||0",
            "key_nonce|TEXT|1||0",
            "encrypted_key|TEXT|1||0",
            "nonce|TEXT|1||0",
            "encrypted_value|TEXT|1||0",
            "key_wrap_alg|TEXT|1|'AES-256-GCM'|0",
            "encryption_alg|TEXT|1|'AES-256-GCM'|0",
            "created_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0",
            "updated_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0",
        ],
    ),
    (
        "secure_store_usages",
        &[
            "id|TEXT|0||1",
            "alias|TEXT|1||0",
            "server_id|TEXT|1||0",
            "location_kind|TEXT|1||0",
            "location_name|TEXT|0||0",
            "location_index|INTEGER|0||0",
            "created_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0",
            "updated_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0",
        ],
    ),
    (
        "secure_store_password_config",
        &[
            "id|INTEGER|0||1",
            "password_hash|TEXT|1||0",
            "hash_salt|TEXT|1||0",
            "hash_iterations|INTEGER|1|600000|0",
            "protection_scope|TEXT|1|'[\"startup\"]'|0",
            "created_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0",
            "updated_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0",
        ],
    ),
    (
        "secure_store_provider_config",
        &[
            "id|INTEGER|0||1",
            "provider_mode|TEXT|1|'operating_system'|0",
            "created_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0",
            "updated_at|TIMESTAMP|1|CURRENT_TIMESTAMP|0",
        ],
    ),
];

async fn secure_store_schema_is_current(transaction: &mut Transaction<'_, Sqlite>) -> Result<bool> {
    for (table, expected) in SECURE_STORE_TABLES {
        let actual: Vec<String> = sqlx::query_scalar(&format!(
            "SELECT name || '|' || upper(type) || '|' || \"notnull\" || '|' || COALESCE(dflt_value, '') || '|' || pk FROM pragma_table_info('{table}') ORDER BY cid"
        ))
        .fetch_all(&mut **transaction)
        .await
        .with_context(|| format!("inspect {table} schema"))?;
        if !actual.iter().map(String::as_str).eq(expected.iter().copied()) {
            return Ok(false);
        }
    }

    let usage_foreign_key: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM pragma_foreign_key_list('secure_store_usages')
            WHERE \"table\" = 'secure_store_secrets'
              AND \"from\" = 'alias'
              AND \"to\" = 'alias'
              AND on_delete = 'CASCADE'
        )",
    )
    .fetch_one(&mut **transaction)
    .await
    .context("inspect secure store usage foreign key")?;
    let usage_identity_unique: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM pragma_index_list('secure_store_usages') AS indexes
            WHERE indexes.\"unique\" = 1
              AND (SELECT group_concat(name, ',') FROM pragma_index_info(indexes.name))
                  = 'alias,server_id,location_kind,location_name,location_index'
        )",
    )
    .fetch_one(&mut **transaction)
    .await
    .context("inspect secure store usage identity constraint")?;
    for table in ["secure_store_password_config", "secure_store_provider_config"] {
        let create_sql: String = sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?")
            .bind(table)
            .fetch_one(&mut **transaction)
            .await
            .with_context(|| format!("inspect {table} constraints"))?;
        if !create_sql.to_ascii_lowercase().contains("check (id = 1)") {
            return Ok(false);
        }
    }
    Ok(usage_foreign_key && usage_identity_unique)
}

async fn secure_store_record_count(transaction: &mut Transaction<'_, Sqlite>) -> Result<i64> {
    let mut count = 0;
    for (table, _) in SECURE_STORE_TABLES {
        count += sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&mut **transaction)
            .await
            .with_context(|| format!("count legacy {table} records"))?;
    }
    Ok(count)
}

async fn rebuild_empty_secure_store(transaction: &mut Transaction<'_, Sqlite>) -> Result<()> {
    for table in [
        "secure_store_usages",
        "secure_store_password_config",
        "secure_store_provider_config",
        "secure_store_secrets",
    ] {
        sqlx::query(&format!("DROP TABLE {table}"))
            .execute(&mut **transaction)
            .await
            .with_context(|| format!("drop empty legacy {table}"))?;
    }
    for statement in SECURE_STORE_SCHEMA.split(";\n").filter(|sql| !sql.trim().is_empty()) {
        sqlx::query(statement)
            .execute(&mut **transaction)
            .await
            .context("recreate current secure store schema")?;
    }
    Ok(())
}
