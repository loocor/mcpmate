// Server arguments management
// Contains operations for managing server command-line arguments

use anyhow::{Context, Result};
use nanoid::nanoid;
use sqlx::{Pool, Sqlite, Transaction};

use crate::conf::{
    models::ServerArg,
    operations::utils::get_server_name_with_tx,
};

/// Get all arguments for a server from the database
pub async fn get_server_args(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<Vec<ServerArg>> {
    tracing::debug!(
        "Executing SQL query to get arguments for server ID {}",
        server_id
    );

    let args = sqlx::query_as::<_, ServerArg>(
        r#"
        SELECT * FROM server_args
        WHERE server_id = ?
        ORDER BY arg_index
        "#,
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
    .context("Failed to fetch server arguments")?;

    tracing::debug!(
        "Successfully fetched {} arguments for server ID {}",
        args.len(),
        server_id
    );
    Ok(args)
}

/// Create or update server arguments in the database
pub async fn upsert_server_args(
    pool: &Pool<Sqlite>,
    server_id: &str,
    args: &[String],
) -> Result<()> {
    tracing::debug!(
        "Upserting {} arguments for server ID {}",
        args.len(),
        server_id
    );

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;
    upsert_server_args_inner(&mut tx, server_id, args).await?;
    tx.commit().await.context("Failed to commit transaction")?;

    Ok(())
}

/// Core logic for upserting server arguments, used internally with a transaction
async fn upsert_server_args_inner(
    tx: &mut Transaction<'_, Sqlite>,
    server_id: &str,
    args: &[String],
) -> Result<()> {
    // Delete existing arguments
    sqlx::query(
        r#"
        DELETE FROM server_args
        WHERE server_id = ?
        "#,
    )
    .bind(server_id)
    .execute(&mut **tx)
    .await
    .context("Failed to delete existing server arguments")?;

    // Get the server name using transaction
    let server_name = get_server_name_with_tx(tx, server_id).await?;

    // Insert new arguments
    for (index, arg) in args.iter().enumerate() {
        // Generate an ID for the argument
        let arg_id = format!("sarg{}", nanoid!(12));

        sqlx::query(
            r#"
            INSERT INTO server_args (id, server_id, server_name, arg_index, arg_value)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&arg_id)
        .bind(server_id)
        .bind(&server_name)
        .bind(index as i32)
        .bind(arg)
        .execute(&mut **tx)
        .await
        .context("Failed to insert server argument")?;
    }

    Ok(())
}
