// Basic CRUD operations for server configuration
// Contains create, read, update, delete operations for servers

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Transaction};

use crate::common::{
    constants::database::{columns, tables},
    database::{fetch_all_ordered, fetch_optional},
};
use crate::config::models::Server;
use crate::generate_id;

/// Get all servers from the database
pub async fn get_all_servers(pool: &Pool<Sqlite>) -> Result<Vec<Server>> {
    fetch_all_ordered(pool, tables::SERVER_CONFIG, Some(columns::NAME)).await
}

/// Get a specific server from the database by name
pub async fn get_server(
    pool: &Pool<Sqlite>,
    name: &str,
) -> Result<Option<Server>> {
    let server: Option<Server> = fetch_optional(pool, tables::SERVER_CONFIG, columns::NAME, name).await?;

    if let Some(ref s) = server {
        tracing::debug!("Found server '{}', type: {}", name, s.server_type);
    } else {
        tracing::debug!("No server found with name '{}'", name);
    }

    Ok(server)
}

/// Get a specific server from the database by ID
pub async fn get_server_by_id(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<Option<Server>> {
    tracing::debug!("Executing SQL query to get server with ID '{}'", id);

    let server = sqlx::query_as::<_, Server>(&format!(
        r#"
        SELECT * FROM {}
        WHERE {} = ?
        "#,
        tables::SERVER_CONFIG,
        columns::ID
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch server by ID")?;

    if let Some(ref s) = server {
        tracing::debug!("Found server with ID '{}', name: {}", id, s.name);
    } else {
        tracing::debug!("No server found with ID '{}'", id);
    }

    Ok(server)
}

/// Create or update a server in the database
pub async fn upsert_server(
    pool: &Pool<Sqlite>,
    server: &Server,
) -> Result<String> {
    tracing::debug!("Upserting server '{}', type: {}", server.name, server.server_type);

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;
    let server_id = upsert_server_tx(&mut tx, server).await?;
    tx.commit().await.context("Failed to commit transaction")?;

    Ok(server_id)
}

/// Create or update a server in the database (transaction version)
pub async fn upsert_server_tx(
    tx: &mut Transaction<'_, Sqlite>,
    server: &Server,
) -> Result<String> {
    // Generate an ID for the server if it doesn't have one
    let server_id = if let Some(id) = &server.id {
        id.clone()
    } else {
        generate_id!("serv")
    };

    let result = sqlx::query(&format!(
        r#"
        INSERT INTO {} ({}, {}, {}, {}, {}, {}, {}, {})
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT({}) DO UPDATE SET
            {} = excluded.{},
            {} = excluded.{},
            {} = excluded.{},
            {} = excluded.{},
            {} = excluded.{},
            {} = excluded.{},
            {} = CURRENT_TIMESTAMP
        "#,
        tables::SERVER_CONFIG,
        columns::ID,
        columns::NAME,
        columns::SERVER_TYPE,
        columns::COMMAND,
        columns::URL,
        columns::TRANSPORT_TYPE,
        columns::REGISTRY_SERVER_ID,
        columns::CAPABILITIES,
        columns::NAME,
        columns::SERVER_TYPE,
        columns::SERVER_TYPE,
        columns::COMMAND,
        columns::COMMAND,
        columns::URL,
        columns::URL,
        columns::TRANSPORT_TYPE,
        columns::TRANSPORT_TYPE,
        columns::REGISTRY_SERVER_ID,
        columns::REGISTRY_SERVER_ID,
        columns::CAPABILITIES,
        columns::CAPABILITIES,
        columns::UPDATED_AT
    ))
    .bind(&server_id)
    .bind(&server.name)
    .bind(server.server_type)
    .bind(&server.command)
    .bind(&server.url)
    .bind(server.transport_type)
    .bind(&server.registry_server_id)
    .bind(&server.capabilities)
    .execute(&mut **tx)
    .await
    .context("Failed to upsert server")?;

    if result.rows_affected() == 0 {
        // If no rows were affected, get the existing ID
        let existing_id = sqlx::query_scalar::<_, String>(&format!(
            r#"
            SELECT {} FROM {}
            WHERE {} = ?
            "#,
            columns::ID,
            tables::SERVER_CONFIG,
            columns::NAME
        ))
        .bind(&server.name)
        .fetch_one(&mut **tx)
        .await
        .context("Failed to get server ID")?;

        return Ok(existing_id);
    }

    Ok(server_id)
}

/// Delete a server from the database
pub async fn delete_server(
    pool: &Pool<Sqlite>,
    name: &str,
) -> Result<bool> {
    tracing::debug!("Deleting server '{}'", name);

    let result = sqlx::query(
        r#"
        DELETE FROM server_config
        WHERE name = ?
        "#,
    )
    .bind(name)
    .execute(pool)
    .await
    .context("Failed to delete server")?;

    Ok(result.rows_affected() > 0)
}
