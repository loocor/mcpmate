use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Sqlite, Transaction};

use super::{
    super::{Migration, MigrationStep},
    CLIENT_SCHEMA,
};

pub(super) fn migration() -> Migration {
    Migration::rust(
        6,
        "normalize legacy client configuration",
        &[include_str!("v0006_normalize_client_configuration.rs"), CLIENT_SCHEMA],
        NormalizeClientSchema,
    )
}

struct NormalizeClientSchema;

#[async_trait]
impl MigrationStep for NormalizeClientSchema {
    async fn apply(
        &self,
        transaction: &mut Transaction<'_, Sqlite>,
    ) -> Result<()> {
        let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('client')")
            .fetch_all(&mut **transaction)
            .await
            .context("inspect client schema")?;
        let create_sql: Option<String> =
            sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'client'")
                .fetch_optional(&mut **transaction)
                .await
                .context("read client schema SQL")?;
        let current = create_sql.as_deref().is_some_and(|sql| {
            sql.contains("streamable_http")
                && sql.contains("connection_mode IN ('local_config_detected', 'manual')")
                && columns.iter().any(|column| column == "attachment_state")
        });
        if !current {
            let direct_exposure_refs = snapshot_table_if_exists(
                transaction,
                "direct_exposure_refs",
                "direct_exposure_refs_migration_snapshot",
            )
            .await?;
            let direct_exposure_servers = snapshot_table_if_exists(
                transaction,
                "direct_exposure_servers",
                "direct_exposure_servers_migration_snapshot",
            )
            .await?;
            let client_table_sql = CLIENT_SCHEMA
                .split(";\n")
                .next()
                .expect("client schema starts with the client table")
                .replace(
                    "CREATE TABLE IF NOT EXISTS client",
                    "CREATE TABLE client_migration_current",
                );
            sqlx::query(&client_table_sql)
                .execute(&mut **transaction)
                .await
                .context("create current client replacement table")?;
            let expr = |column: &str, fallback: &str| {
                if columns.iter().any(|existing| existing == column) {
                    column.to_string()
                } else {
                    fallback.to_string()
                }
            };
            let display_name = if columns.iter().any(|column| column == "display_name") {
                "COALESCE(NULLIF(display_name, ''), name)".to_string()
            } else {
                "name".to_string()
            };
            let config_mode = if columns.iter().any(|column| column == "config_mode") {
                "NULLIF(config_mode, '')".to_string()
            } else {
                "NULL".to_string()
            };
            let transport = if columns.iter().any(|column| column == "transport") {
                "COALESCE(NULLIF(transport, ''), 'auto')".to_string()
            } else {
                "'auto'".to_string()
            };
            let source = if columns.iter().any(|column| column == "capability_source") {
                "COALESCE(NULLIF(capability_source, ''), 'activated')".to_string()
            } else {
                "'activated'".to_string()
            };
            let connection_mode = if columns.iter().any(|column| column == "config_path") {
                "CASE WHEN config_path IS NOT NULL AND TRIM(config_path) <> '' THEN 'local_config_detected' ELSE 'manual' END".to_string()
            } else {
                "'manual'".to_string()
            };
            let registration = if columns.iter().any(|column| column == "connection_mode") {
                if columns.iter().any(|column| column == "config_path") {
                    "CASE WHEN connection_mode = 'remote_http' THEN 'runtime_initialize' WHEN config_path IS NOT NULL AND TRIM(config_path) <> '' THEN 'config_detection' ELSE 'manual' END".to_string()
                } else {
                    "CASE WHEN connection_mode = 'remote_http' THEN 'runtime_initialize' ELSE 'manual' END".to_string()
                }
            } else {
                "'manual'".to_string()
            };
            let observed = if columns.iter().any(|column| column == "connection_mode") {
                "CASE WHEN connection_mode = 'remote_http' THEN 1 ELSE 0 END".to_string()
            } else {
                "0".to_string()
            };
            let transports = if columns.iter().any(|column| column == "transports") {
                "transports".to_string()
            } else if columns.iter().any(|column| column == "format_rules") {
                "format_rules".to_string()
            } else {
                "NULL".to_string()
            };
            let select = vec![
                expr("id", "NULL"),
                expr("name", "''"),
                display_name,
                expr("identifier", "''"),
                expr("config_path", "NULL"),
                config_mode,
                transport,
                expr("client_version", "NULL"),
                format!("COALESCE({}, 'keep_n')", expr("backup_policy", "NULL")),
                expr("backup_limit", "5"),
                source,
                expr("unify_route_mode", "'broker_only'"),
                expr("governance_kind", "'passive'"),
                connection_mode,
                registration,
                observed,
                expr("template_identifier", "identifier"),
                expr("selected_profile_ids", "NULL"),
                expr("custom_profile_id", "NULL"),
                expr("approval_status", "'approved'"),
                expr("template_id", "NULL"),
                expr("template_version", "NULL"),
                expr("approval_metadata", "NULL"),
                expr("config_format", "NULL"),
                expr("protocol_revision", "NULL"),
                expr("container_type", "NULL"),
                expr("container_keys", "NULL"),
                expr("storage_kind", "NULL"),
                expr("storage_adapter", "NULL"),
                expr("storage_path_strategy", "NULL"),
                expr("merge_strategy", "NULL"),
                expr("keep_original_config", "NULL"),
                expr("managed_source", "NULL"),
                transports,
                expr("config_file_parse", "NULL"),
                expr("attachment_state", "'not_applicable'"),
                expr("created_at", "CURRENT_TIMESTAMP"),
                expr("updated_at", "CURRENT_TIMESTAMP"),
            ]
            .join(", ");
            sqlx::query(&format!(
                "INSERT INTO client_migration_current SELECT {select} FROM client"
            ))
            .execute(&mut **transaction)
            .await
            .context("copy legacy client rows")?;
            sqlx::query(
                "CREATE TABLE client_writeback_policy_migration_current (client_identifier TEXT PRIMARY KEY, merge_strategy TEXT NOT NULL CHECK (merge_strategy IN ('replace', 'deep_merge')), created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, FOREIGN KEY (client_identifier) REFERENCES client_migration_current(identifier) ON DELETE CASCADE)",
            )
            .execute(&mut **transaction)
            .await
            .context("create current client writeback policy replacement table")?;
            sqlx::query(
                "INSERT INTO client_writeback_policy_migration_current SELECT client_identifier, merge_strategy, created_at, updated_at FROM client_writeback_policy",
            )
            .execute(&mut **transaction)
            .await
            .context("copy client writeback policies")?;
            sqlx::query("DROP TABLE client_writeback_policy")
                .execute(&mut **transaction)
                .await
                .context("drop legacy client writeback policy table")?;
            sqlx::query("DROP TABLE client")
                .execute(&mut **transaction)
                .await
                .context("drop legacy client table")?;
            sqlx::query("ALTER TABLE client_migration_current RENAME TO client")
                .execute(&mut **transaction)
                .await
                .context("rename rebuilt client table")?;
            sqlx::query("ALTER TABLE client_writeback_policy_migration_current RENAME TO client_writeback_policy")
                .execute(&mut **transaction)
                .await
                .context("rename rebuilt client writeback policy table")?;
            restore_table_snapshot(
                transaction,
                "direct_exposure_refs",
                "direct_exposure_refs_migration_snapshot",
                direct_exposure_refs,
            )
            .await?;
            restore_table_snapshot(
                transaction,
                "direct_exposure_servers",
                "direct_exposure_servers_migration_snapshot",
                direct_exposure_servers,
            )
            .await?;
        }
        sqlx::query(
            "INSERT OR IGNORE INTO client_runtime_settings (key, value) VALUES ('default_config_mode', 'unify')",
        )
        .execute(&mut **transaction)
        .await
        .context("seed default client config mode")?;
        Ok(())
    }
}

async fn snapshot_table_if_exists(
    transaction: &mut Transaction<'_, Sqlite>,
    table: &str,
    snapshot: &str,
) -> Result<bool> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?)")
            .bind(table)
            .fetch_one(&mut **transaction)
            .await
            .with_context(|| format!("inspect {table} before client migration"))?;
    if exists {
        sqlx::query(&format!("CREATE TEMP TABLE {snapshot} AS SELECT * FROM {table}"))
            .execute(&mut **transaction)
            .await
            .with_context(|| format!("snapshot {table} before client migration"))?;
    }
    Ok(exists)
}

async fn restore_table_snapshot(
    transaction: &mut Transaction<'_, Sqlite>,
    table: &str,
    snapshot: &str,
    snapshot_exists: bool,
) -> Result<()> {
    if !snapshot_exists {
        return Ok(());
    }
    sqlx::query(&format!("INSERT INTO {table} SELECT * FROM {snapshot}"))
        .execute(&mut **transaction)
        .await
        .with_context(|| format!("restore {table} after client migration"))?;
    sqlx::query(&format!("DROP TABLE {snapshot}"))
        .execute(&mut **transaction)
        .await
        .with_context(|| format!("drop {table} migration snapshot"))?;
    Ok(())
}
