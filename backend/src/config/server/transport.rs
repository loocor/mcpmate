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
    use crate::{
        common::{server::ServerType, status::EnabledStatus},
        config::{
            models::{ConfigValue, Server, ServerTransportDraft},
            server::{get_server_args, get_server_env, get_server_headers, upsert_server_definition},
        },
    };

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

    #[tokio::test]
    async fn persists_a_validated_definition_and_legacy_projection_atomically() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("open test database");
        crate::test_helpers::prepare_config_database(&pool).await;
        let server = Server {
            id: Some("server-b".into()),
            name: "B".into(),
            server_type: ServerType::Stdio,
            command: None,
            url: None,
            source: None,
            enabled: EnabledStatus::Enabled,
            unify_direct_exposure_eligible: false,
            pending_import: false,
            created_at: None,
            updated_at: None,
        };
        let draft = ServerTransportDraft::Http {
            protocol: crate::config::models::HttpTransportKind::StreamableHttp,
            endpoint: Some("https://example.com/mcp".into()),
            headers: BTreeMap::from([("Authorization".into(), ConfigValue::SecretRef { alias: "token".into() })]),
        };

        let server_id = upsert_server_definition(&pool, &server, &draft)
            .await
            .expect("persist definition");

        assert_eq!(server_id, "server-b");
        assert_eq!(
            get_server_transport_draft(&pool, &server_id).await.expect("load draft"),
            Some(draft),
        );
        let stored = crate::config::server::get_server_by_id(&pool, &server_id)
            .await
            .expect("load server")
            .expect("server exists");
        assert_eq!(stored.server_type, ServerType::StreamableHttp);
        assert_eq!(stored.command, None);
        assert_eq!(stored.url.as_deref(), Some("https://example.com/mcp"));
        assert!(get_server_args(&pool, &server_id).await.expect("load args").is_empty());
        assert!(get_server_env(&pool, &server_id).await.expect("load env").is_empty());
        assert_eq!(
            get_server_headers(&pool, &server_id).await.expect("load headers"),
            std::collections::HashMap::from([("authorization".into(), "[[secret:token]]".into(),)]),
        );

        let replacement = ServerTransportDraft::Http {
            protocol: crate::config::models::HttpTransportKind::StreamableHttp,
            endpoint: Some("https://example.com/mcp".into()),
            headers: BTreeMap::from([
                (
                    "AUTHORIZATION".into(),
                    ConfigValue::Literal {
                        value: "Bearer replacement".into(),
                    },
                ),
                (
                    "X-Request-Id".into(),
                    ConfigValue::Literal {
                        value: "request-id".into(),
                    },
                ),
            ]),
        };
        upsert_server_definition(&pool, &server, &replacement)
            .await
            .expect("replace definition");

        assert_eq!(
            get_server_headers(&pool, &server_id)
                .await
                .expect("load replaced headers"),
            std::collections::HashMap::from([
                ("authorization".into(), "Bearer replacement".into()),
                ("x-request-id".into(), "request-id".into()),
            ]),
        );
    }

    #[tokio::test]
    async fn rejects_invalid_definition_before_persisting_any_projection() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("open test database");
        crate::test_helpers::prepare_config_database(&pool).await;
        let server = Server {
            id: Some("server-c".into()),
            name: "C".into(),
            server_type: ServerType::Stdio,
            command: None,
            url: None,
            source: None,
            enabled: EnabledStatus::Enabled,
            unify_direct_exposure_eligible: false,
            pending_import: false,
            created_at: None,
            updated_at: None,
        };
        let draft = ServerTransportDraft::Stdio {
            command: None,
            args: Vec::new(),
            env: BTreeMap::new(),
        };

        assert!(upsert_server_definition(&pool, &server, &draft).await.is_err());
        assert!(
            crate::config::server::get_server_by_id(&pool, "server-c")
                .await
                .expect("load server")
                .is_none()
        );
        assert!(
            get_server_transport_draft(&pool, "server-c")
                .await
                .expect("load draft")
                .is_none()
        );
    }
}
