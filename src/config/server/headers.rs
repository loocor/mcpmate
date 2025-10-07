// Server HTTP headers persistence helpers
// Provides CRUD utilities for default headers used by HTTP-based transports

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

const TABLE: &str = "server_headers";

/// Create or replace all headers for a server (idempotent upsert per key)
pub async fn upsert_server_headers(
    pool: &Pool<Sqlite>,
    server_id: &str,
    headers: &HashMap<String, String>,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Upsert each header (normalized key to lowercase)
    for (k, v) in headers.iter() {
        let key = k.trim().to_ascii_lowercase();
        if key.is_empty() { continue; }
        sqlx::query(&format!(
            r#"
            INSERT INTO {TABLE} (server_id, header_key, header_value)
            VALUES (?, ?, ?)
            ON CONFLICT(server_id, header_key) DO UPDATE SET header_value = excluded.header_value
            "#,
        ))
        .bind(server_id)
        .bind(&key)
        .bind(v)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Replace headers set: remove keys not present in provided map, upsert provided ones
pub async fn replace_server_headers(
    pool: &Pool<Sqlite>,
    server_id: &str,
    headers: &HashMap<String, String>,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    // Fetch existing keys
    let rows: Vec<(String,)> = sqlx::query_as(&format!(
        "SELECT header_key FROM {TABLE} WHERE server_id = ?"
    ))
    .bind(server_id)
    .fetch_all(&mut *tx)
    .await?;
    let existing: std::collections::HashSet<String> = rows.into_iter().map(|(k,)| k).collect();

    // Upsert provided
    for (k, v) in headers.iter() {
        let key = k.trim().to_ascii_lowercase();
        if key.is_empty() { continue; }
        sqlx::query(&format!(
            r#"INSERT INTO {TABLE} (server_id, header_key, header_value)
                VALUES (?, ?, ?)
                ON CONFLICT(server_id, header_key) DO UPDATE SET header_value = excluded.header_value"#
        ))
        .bind(server_id)
        .bind(&key)
        .bind(v)
        .execute(&mut *tx)
        .await?;
    }

    // Delete removed keys
    for key in existing {
        if !headers.contains_key(&key) && !headers.contains_key(&key.to_ascii_uppercase()) {
            sqlx::query(&format!("DELETE FROM {TABLE} WHERE server_id = ? AND header_key = ?"))
                .bind(server_id)
                .bind(&key)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

/// Get all headers for a server as a map (keys normalized to lowercase)
pub async fn get_server_headers(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<HashMap<String, String>> {
    let rows = sqlx::query_as::<_, (String, String)>(&format!(
        "SELECT header_key, header_value FROM {TABLE} WHERE server_id = ? ORDER BY header_key"
    ))
    .bind(server_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

