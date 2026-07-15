use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Transaction};

/// Get the persisted MCPMate namespace by server ID.
pub async fn get_server_name(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<String> {
    let name = sqlx::query_scalar::<_, String>(r#"SELECT name FROM server_config WHERE id = ?"#)
        .bind(server_id)
        .fetch_optional(pool)
        .await
        .context("Failed to get server namespace")?
        .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_id))?;
    Ok(name)
}

/// Get the persisted MCPMate namespace by server ID using a transaction.
pub async fn get_server_name_with_tx(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
) -> Result<String> {
    let name = sqlx::query_scalar::<_, String>(r#"SELECT name FROM server_config WHERE id = ?"#)
        .bind(server_id)
        .fetch_optional(&mut **tx)
        .await
        .context("Failed to get server namespace (tx)")?
        .ok_or_else(|| anyhow::anyhow!("Server '{}' not found", server_id))?;
    Ok(name)
}
