use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Transaction};

/// Get server_name by server_id. If not found, returns "unknown". Spaces are replaced with underscores.
pub async fn get_server_name(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<String> {
    let name = sqlx::query_scalar::<_, String>(r#"SELECT name FROM server_config WHERE id = ?"#)
        .bind(server_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get server name")?
        .unwrap_or_else(|| "unknown".to_string());
    Ok(name.replace(' ', "_"))
}

/// Get server_name by server_id using a transaction. If not found, returns "unknown". Spaces are replaced with underscores.
pub async fn get_server_name_with_tx(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
) -> Result<String> {
    let name = sqlx::query_scalar::<_, String>(r#"SELECT name FROM server_config WHERE id = ?"#)
        .bind(server_id)
        .fetch_optional(&mut **tx)
        .await
        .context("Failed to get server name (tx)")?
        .unwrap_or_else(|| "unknown".to_string());
    Ok(name.replace(' ', "_"))
}
