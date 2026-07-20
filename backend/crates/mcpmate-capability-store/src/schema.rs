use sqlx::{Pool, Sqlite};

use crate::Result;

pub(crate) async fn ensure_schema(pool: &Pool<Sqlite>) -> Result<()> {
    let mut transaction = pool.begin().await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS capability_server_snapshots (
            server_id TEXT PRIMARY KEY,
            server_name TEXT NOT NULL,
            config_fingerprint TEXT NOT NULL,
            record_format_version INTEGER NOT NULL,
            catalog_revision INTEGER NOT NULL,
            snapshot_state TEXT NOT NULL,
            initialize_payload TEXT NOT NULL,
            observed_at TEXT NOT NULL,
            committed_at TEXT NOT NULL,
            last_error TEXT
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS capability_kind_states (
            server_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            kind TEXT NOT NULL,
            declaration_state TEXT NOT NULL,
            inventory_state TEXT NOT NULL,
            error TEXT,
            catalog_revision INTEGER NOT NULL,
            observed_at TEXT NOT NULL,
            PRIMARY KEY (server_id, kind),
            FOREIGN KEY (server_id) REFERENCES capability_server_snapshots(server_id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS capability_records (
            stable_id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            kind TEXT NOT NULL,
            upstream_key TEXT NOT NULL,
            external_key TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            record_format_version INTEGER NOT NULL,
            catalog_revision INTEGER NOT NULL,
            FOREIGN KEY (server_id) REFERENCES capability_server_snapshots(server_id) ON DELETE CASCADE,
            UNIQUE (server_id, kind, upstream_key),
            UNIQUE (kind, external_key)
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_capability_records_server_kind ON capability_records(server_id, kind)")
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;
    Ok(())
}
