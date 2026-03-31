// Registry cache database initialization

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

const REGISTRY_CACHE_TABLE: &str = "registry_cache";

/// Initialize registry cache table
pub async fn initialize_registry_cache_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing registry_cache table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS registry_cache (
            server_name TEXT PRIMARY KEY,
            version TEXT NOT NULL,
            schema_url TEXT,
            title TEXT,
            description TEXT,
            packages_json TEXT,
            remotes_json TEXT,
            icons_json TEXT,
            meta_json TEXT,
            website_url TEXT,
            repository_json TEXT,
            status TEXT DEFAULT 'active',
            published_at TIMESTAMP,
            updated_at TIMESTAMP,
            synced_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create registry_cache table: {}", e);
        anyhow::anyhow!("Failed to create registry_cache table: {}", e)
    })?;

    // Create indexes for common queries
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_registry_cache_status ON registry_cache(status)")
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create registry_cache status index: {}", e);
            anyhow::anyhow!("Failed to create registry_cache status index: {}", e)
        })?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_registry_cache_synced_at ON registry_cache(synced_at)")
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create registry_cache synced_at index: {}", e);
            anyhow::anyhow!("Failed to create registry_cache synced_at index: {}", e)
        })?;

    ensure_column(pool, "registry_cache", "website_url", "TEXT").await?;
    ensure_column(pool, "registry_cache", "repository_json", "TEXT").await?;
    ensure_column(pool, "registry_cache", "schema_url", "TEXT").await?;

    tracing::debug!("registry_cache table initialized successfully");
    Ok(())
}

async fn ensure_column(
    pool: &Pool<Sqlite>,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    let stmt = format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition}",
        table = table,
        column = column,
        definition = definition
    );

    match sqlx::query(&stmt).execute(pool).await {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column name") => Ok(()),
        Err(e) => Err(anyhow::anyhow!("Failed to add column {}.{}: {}", table, column, e)),
    }
}

/// Verify registry_cache table exists
pub async fn verify_registry_cache_table(pool: &Pool<Sqlite>) -> Result<()> {
    let exists: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?",
    )
    .bind(REGISTRY_CACHE_TABLE)
    .fetch_optional(pool)
    .await?;

    if exists.is_none() {
        return Err(anyhow::anyhow!(
            "registry_cache table not found after creation"
        ));
    }

    tracing::debug!("Verified registry_cache table exists");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory SQLite pool")
    }

    #[tokio::test]
    async fn test_initialize_registry_cache_table() {
        let pool = setup_test_db().await;
        let result = initialize_registry_cache_table(&pool).await;
        assert!(result.is_ok());

        // Verify table exists
        let verify_result = verify_registry_cache_table(&pool).await;
        assert!(verify_result.is_ok());
    }

    #[tokio::test]
    async fn test_table_has_correct_schema() {
        let pool = setup_test_db().await;
        initialize_registry_cache_table(&pool).await.unwrap();

        // Check that we can insert a record with all fields
        let result = sqlx::query(
            r#"
            INSERT INTO registry_cache (
                server_name, version, schema_url, title, description,
                packages_json, remotes_json, icons_json, meta_json,
                website_url, repository_json,
                status, published_at, updated_at, synced_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind("test-server")
        .bind("1.0.0")
        .bind("https://modelcontextprotocol.io/schema/server.schema.json")
        .bind("Test Server")
        .bind("A test server")
        .bind("[]")
        .bind("[]")
        .bind("[]")
        .bind("{}")
        .bind("https://example.com")
        .bind(r#"{"url":"https://github.com/example/test-server","source":"github"}"#)
        .bind("active")
        .bind(None::<String>)
        .bind(None::<String>)
        .bind("2025-01-01T00:00:00Z")
        .execute(&pool)
        .await;

        assert!(result.is_ok());
    }
}
