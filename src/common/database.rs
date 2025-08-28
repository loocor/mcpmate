//! Unified database query helpers
//!
//! Provides standardized database query patterns to eliminate code duplication
//! and ensure consistent error handling across all database operations.

use anyhow::{Context, Result};
use sqlx::{FromRow, Pool, Sqlite};
use tracing;

/// Generic query helper for fetching all records with optional ordering
///
/// Standardizes the pattern of fetching all records from a table with consistent
/// logging and error handling.
pub async fn fetch_all_ordered<T>(
    pool: &Pool<Sqlite>,
    table_name: &str,
    order_by: Option<&str>,
) -> Result<Vec<T>>
where
    T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
{
    let order_clause = order_by.unwrap_or("id");
    let query = format!("SELECT * FROM {} ORDER BY {}", table_name, order_clause);

    tracing::debug!("Executing query to fetch all records from {}", table_name);

    let records = sqlx::query_as::<_, T>(&query)
        .fetch_all(pool)
        .await
        .with_context(|| format!("Failed to fetch records from {}", table_name))?;

    tracing::debug!("Successfully fetched {} records from {}", records.len(), table_name);

    Ok(records)
}

/// Generic query helper for fetching records with WHERE condition
///
/// Standardizes the pattern of fetching records with a single WHERE condition.
pub async fn fetch_where<T>(
    pool: &Pool<Sqlite>,
    table_name: &str,
    where_column: &str,
    where_value: &str,
    order_by: Option<&str>,
) -> Result<Vec<T>>
where
    T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
{
    let order_clause = match order_by {
        Some(order) => format!(" ORDER BY {}", order),
        None => String::new(),
    };

    let query = format!(
        "SELECT * FROM {} WHERE {} = ?{}",
        table_name, where_column, order_clause
    );

    tracing::debug!(
        "Executing query to fetch records from {} where {} = '{}'",
        table_name,
        where_column,
        where_value
    );

    let records = sqlx::query_as::<_, T>(&query)
        .bind(where_value)
        .fetch_all(pool)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch records from {} where {} = '{}'",
                table_name, where_column, where_value
            )
        })?;

    tracing::debug!(
        "Successfully fetched {} records from {} where {} = '{}'",
        records.len(),
        table_name,
        where_column,
        where_value
    );

    Ok(records)
}

/// Generic query helper for fetching a single optional record
///
/// Standardizes the pattern of fetching a single record that may or may not exist.
pub async fn fetch_optional<T>(
    pool: &Pool<Sqlite>,
    table_name: &str,
    where_column: &str,
    where_value: &str,
) -> Result<Option<T>>
where
    T: for<'r> FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
{
    let query = format!("SELECT * FROM {} WHERE {} = ?", table_name, where_column);

    tracing::debug!(
        "Executing query to fetch single record from {} where {} = '{}'",
        table_name,
        where_column,
        where_value
    );

    let record = sqlx::query_as::<_, T>(&query)
        .bind(where_value)
        .fetch_optional(pool)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch record from {} where {} = '{}'",
                table_name, where_column, where_value
            )
        })?;

    match &record {
        Some(_) => tracing::debug!(
            "Found record in {} where {} = '{}'",
            table_name,
            where_column,
            where_value
        ),
        None => tracing::debug!(
            "No record found in {} where {} = '{}'",
            table_name,
            where_column,
            where_value
        ),
    }

    Ok(record)
}

/// Generic query helper for fetching a single scalar value
///
/// Standardizes the pattern of fetching a single column value.
pub async fn fetch_scalar<T>(
    pool: &Pool<Sqlite>,
    table_name: &str,
    column_name: &str,
    where_column: &str,
    where_value: &str,
) -> Result<Option<T>>
where
    T: for<'r> sqlx::Decode<'r, sqlx::Sqlite> + sqlx::Type<sqlx::Sqlite> + Send + Unpin,
{
    let query = format!("SELECT {} FROM {} WHERE {} = ?", column_name, table_name, where_column);

    tracing::debug!(
        "Executing scalar query: {} from {} where {} = '{}'",
        column_name,
        table_name,
        where_column,
        where_value
    );

    let value = sqlx::query_scalar::<_, T>(&query)
        .bind(where_value)
        .fetch_optional(pool)
        .await
        .with_context(|| {
            format!(
                "Failed to fetch {} from {} where {} = '{}'",
                column_name, table_name, where_column, where_value
            )
        })?;

    match &value {
        Some(_) => tracing::debug!(
            "Found {} value in {} where {} = '{}'",
            column_name,
            table_name,
            where_column,
            where_value
        ),
        None => tracing::debug!(
            "No {} value found in {} where {} = '{}'",
            column_name,
            table_name,
            where_column,
            where_value
        ),
    }

    Ok(value)
}

/// Generic query helper for checking record existence
///
/// Standardizes the pattern of checking if a record exists without fetching it.
pub async fn record_exists(
    pool: &Pool<Sqlite>,
    table_name: &str,
    where_column: &str,
    where_value: &str,
) -> Result<bool> {
    let query = format!("SELECT 1 FROM {} WHERE {} = ? LIMIT 1", table_name, where_column);

    tracing::debug!(
        "Checking if record exists in {} where {} = '{}'",
        table_name,
        where_column,
        where_value
    );

    let exists = sqlx::query_scalar::<_, i32>(&query)
        .bind(where_value)
        .fetch_optional(pool)
        .await
        .with_context(|| {
            format!(
                "Failed to check existence in {} where {} = '{}'",
                table_name, where_column, where_value
            )
        })?
        .is_some();

    tracing::debug!(
        "Record {} in {} where {} = '{}'",
        if exists { "exists" } else { "does not exist" },
        table_name,
        where_column,
        where_value
    );

    Ok(exists)
}

/// Generic query helper for counting records
///
/// Standardizes the pattern of counting records with optional WHERE condition.
pub async fn count_records(
    pool: &Pool<Sqlite>,
    table_name: &str,
    where_condition: Option<(&str, &str)>,
) -> Result<i64> {
    let (query, log_msg) = match where_condition {
        Some((column, value)) => (
            format!("SELECT COUNT(*) FROM {} WHERE {} = ?", table_name, column),
            format!("Counting records in {} where {} = '{}'", table_name, column, value),
        ),
        None => (
            format!("SELECT COUNT(*) FROM {}", table_name),
            format!("Counting all records in {}", table_name),
        ),
    };

    tracing::debug!("{}", log_msg);

    let mut query_builder = sqlx::query_scalar::<_, i64>(&query);

    if let Some((_, value)) = where_condition {
        query_builder = query_builder.bind(value);
    }

    let count = query_builder
        .fetch_one(pool)
        .await
        .with_context(|| format!("Failed to count records in {}", table_name))?;

    tracing::debug!("Found {} records in {}", count, table_name);

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use tempfile::tempdir;

    async fn setup_test_db() -> SqlitePool {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite:{}", db_path.display()))
            .await
            .unwrap();

        // Create test table
        sqlx::query(
            r#"
            CREATE TABLE test_table (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                value INTEGER
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert test data
        sqlx::query("INSERT INTO test_table (id, name, value) VALUES (?, ?, ?)")
            .bind("1")
            .bind("test1")
            .bind(100)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO test_table (id, name, value) VALUES (?, ?, ?)")
            .bind("2")
            .bind("test2")
            .bind(200)
            .execute(&pool)
            .await
            .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_record_exists() {
        let pool = setup_test_db().await;

        let exists = record_exists(&pool, "test_table", "name", "test1").await.unwrap();
        assert!(exists);

        let not_exists = record_exists(&pool, "test_table", "name", "nonexistent").await.unwrap();
        assert!(!not_exists);
    }

    #[tokio::test]
    async fn test_count_records() {
        let pool = setup_test_db().await;

        let total_count = count_records(&pool, "test_table", None).await.unwrap();
        assert_eq!(total_count, 2);

        let filtered_count = count_records(&pool, "test_table", Some(("name", "test1")))
            .await
            .unwrap();
        assert_eq!(filtered_count, 1);
    }

    #[tokio::test]
    async fn test_fetch_scalar() {
        let pool = setup_test_db().await;

        let value: Option<i32> = fetch_scalar(&pool, "test_table", "value", "name", "test1")
            .await
            .unwrap();
        assert_eq!(value, Some(100));

        let no_value: Option<i32> = fetch_scalar(&pool, "test_table", "value", "name", "nonexistent")
            .await
            .unwrap();
        assert_eq!(no_value, None);
    }
}
