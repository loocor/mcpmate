use anyhow::Result;
use sqlx::{Pool, Sqlite};

use crate::common::constants::database::tables;

/// Verify Profile authoring storage after the config migration stream has run.
pub async fn initialize_profile_tables(pool: &Pool<Sqlite>) -> Result<()> {
    mcpmate_migrations::migrate_config(pool).await?;
    verify_profile_tables(pool).await
}

async fn verify_profile_tables(pool: &Pool<Sqlite>) -> Result<()> {
    for table in [
        tables::PROFILE,
        "profile_server_relationships",
        tables::SERVER_TOOLS,
        tables::SERVER_PROMPTS,
        tables::SERVER_RESOURCES,
        tables::SERVER_RESOURCE_TEMPLATES,
        "server_issued_resources",
        "profile_capability_refs",
        "direct_exposure_refs",
        "direct_exposure_servers",
    ] {
        sqlx::query(&format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"
        ))
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("{table} table not found after migration"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn resource_registry_schema_contains_template_routes_and_issued_resources() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .unwrap();
        initialize_profile_tables(&pool).await.unwrap();
        let template_columns =
            sqlx::query_scalar::<_, String>("SELECT name FROM pragma_table_info('server_resource_templates')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(template_columns.iter().any(|column| column == "route_uri"));
        let issued_columns =
            sqlx::query_scalar::<_, String>("SELECT name FROM pragma_table_info('server_issued_resources')")
                .fetch_all(&pool)
                .await
                .unwrap();
        for expected in [
            "id",
            "server_id",
            "server_name",
            "resource_uri",
            "unique_uri",
            "created_at",
            "last_seen_at",
        ] {
            assert!(
                issued_columns.iter().any(|column| column == expected),
                "missing issued resource column {expected}"
            );
        }
        let issued_indexes =
            sqlx::query_scalar::<_, String>("SELECT name FROM pragma_index_list('server_issued_resources')")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert!(
            issued_indexes
                .iter()
                .any(|index| index == "idx_server_issued_resources_lookup")
        );
        assert!(
            issued_indexes
                .iter()
                .any(|index| index == "idx_server_issued_resources_unique_uri")
        );
    }

    #[tokio::test]
    async fn authoring_schema_uses_capability_refs_without_legacy_capability_tables() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::config::server::init::initialize_server_tables(&pool)
            .await
            .unwrap();
        crate::config::client::init::initialize_client_table(&pool)
            .await
            .unwrap();
        crate::config::database::initialize_capability_catalog(&pool)
            .await
            .unwrap();
        initialize_profile_tables(&pool).await.unwrap();
        for table in [
            "profile_capability_refs",
            "profile_server_relationships",
            "direct_exposure_refs",
            "direct_exposure_servers",
        ] {
            let exists: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
                    .bind(table)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(exists, 1, "missing authoring table {table}");
        }
        for legacy in [
            "profile_tool",
            "profile_prompt",
            "profile_resource",
            "profile_resource_template",
        ] {
            let exists: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?")
                    .bind(legacy)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(exists, 0, "legacy authoring table {legacy} must not exist");
        }
    }
}
