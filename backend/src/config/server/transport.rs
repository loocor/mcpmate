use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Transaction};

use crate::config::models::ServerTransportDraft;

pub async fn get_server_transport_draft(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<Option<ServerTransportDraft>> {
    let draft_json: Option<String> = sqlx::query_scalar("SELECT draft_json FROM server_transport WHERE server_id = ?")
        .bind(server_id)
        .fetch_optional(pool)
        .await
        .context("load server transport draft")?;
    draft_json
        .map(|draft_json| serde_json::from_str(&draft_json).context("decode server transport draft"))
        .transpose()
}

pub async fn upsert_server_transport_draft_tx(
    transaction: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    draft: &ServerTransportDraft,
) -> Result<()> {
    let draft_json = serde_json::to_string(draft).context("encode server transport draft")?;
    sqlx::query(
        "INSERT INTO server_transport (server_id, draft_json)
         VALUES (?, ?)
         ON CONFLICT(server_id) DO UPDATE SET
           draft_json = excluded.draft_json,
           updated_at = CURRENT_TIMESTAMP",
    )
    .bind(server_id)
    .bind(draft_json)
    .execute(&mut **transaction)
    .await
    .context("upsert server transport draft")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use sqlx::sqlite::SqlitePoolOptions;

    use super::{get_server_transport_draft, upsert_server_transport_draft_tx};
    use crate::config::models::ServerTransportDraft;

    #[tokio::test]
    async fn stores_and_reloads_a_single_typed_transport_draft() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("open test database");
        crate::test_helpers::prepare_config_database(&pool).await;
        sqlx::query("INSERT INTO server_config (id, name, server_type) VALUES ('server-a', 'A', 'stdio')")
            .execute(&pool)
            .await
            .expect("insert server identity");
        let draft = ServerTransportDraft::Stdio {
            command: Some("echo".into()),
            args: vec!["hello".into()],
            env: BTreeMap::new(),
        };

        let mut transaction = pool.begin().await.expect("begin transaction");
        upsert_server_transport_draft_tx(&mut transaction, "server-a", &draft)
            .await
            .expect("store draft");
        transaction.commit().await.expect("commit draft");

        assert_eq!(
            get_server_transport_draft(&pool, "server-a").await.expect("load draft"),
            Some(draft),
        );
    }
}
