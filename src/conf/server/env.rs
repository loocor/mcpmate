// Server environment variables management
// Contains operations for managing server environment variables

use std::collections::HashMap;

use anyhow::{Context, Result};
use nanoid::nanoid;
use sqlx::{Pool, Sqlite, Transaction};

use crate::conf::{
    models::ServerEnv,
    operations::utils::get_server_name_with_tx,
};

/// Get all environment variables for a server from the database
pub async fn get_server_env(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<HashMap<String, String>> {
    tracing::debug!(
        "Executing SQL query to get environment variables for server ID {}",
        server_id
    );

    let env_vars = sqlx::query_as::<_, ServerEnv>(
        r#"
        SELECT * FROM server_env
        WHERE server_id = ?
        "#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
    .context("Failed to fetch server environment variables")?;

    let mut env_map = HashMap::new();
    for env_var in env_vars {
        env_map.insert(env_var.env_key, env_var.env_value);
    }

    tracing::debug!(
        "Successfully fetched {} environment variables for server ID {}",
        env_map.len(),
        server_id
    );
    Ok(env_map)
}

/// Create or update server environment variables in the database
pub async fn upsert_server_env(
    pool: &Pool<Sqlite>,
    server_id: &str,
    env: &HashMap<String, String>,
) -> Result<()> {
    tracing::debug!(
        "Upserting {} environment variables for server ID {}",
        env.len(),
        server_id
    );

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;
    upsert_server_env_inner(&mut tx, server_id, env).await?;
    tx.commit().await.context("Failed to commit transaction")?;

    Ok(())
}

/// Core logic for upserting server environment variables, used internally with a transaction
async fn upsert_server_env_inner(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    env: &HashMap<String, String>,
) -> Result<()> {
    // Delete existing environment variables
    sqlx::query(
        r#"
        DELETE FROM server_env
        WHERE server_id = ?
        "#,
    )
    .bind(server_id)
    .execute(&mut **tx)
    .await
    .context("Failed to delete existing server environment variables")?;

    // Get the server name using transaction
    let server_name = get_server_name_with_tx(tx, server_id).await?;

    // Insert new environment variables
    for (key, value) in env {
        // Generate an ID for the environment variable
        let env_id = format!("senv{}", nanoid!(12));

        sqlx::query(
            r#"
            INSERT INTO server_env (id, server_id, server_name, env_key, env_value)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&env_id)
        .bind(server_id)
        .bind(&server_name)
        .bind(key)
        .bind(value)
        .execute(&mut **tx)
        .await
        .context("Failed to insert server environment variable")?;
    }

    Ok(())
}
