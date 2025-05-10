// Server operations for MCPMate
// Contains CRUD operations for server configuration

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Transaction};
use std::collections::{HashMap, HashSet};

use crate::conf::models::{Server, ServerArg, ServerEnv, ServerMeta};

/// Get all servers from the database
pub async fn get_all_servers(pool: &Pool<Sqlite>) -> Result<Vec<Server>> {
    tracing::debug!("Executing SQL query to get all servers");

    let servers = sqlx::query_as::<_, Server>(
        r#"
        SELECT * FROM server_config
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await
    .context("Failed to fetch servers")?;

    tracing::debug!(
        "Successfully fetched {} servers from database",
        servers.len()
    );
    Ok(servers)
}

/// Get a specific server from the database
pub async fn get_server(pool: &Pool<Sqlite>, name: &str) -> Result<Option<Server>> {
    tracing::debug!("Executing SQL query to get server '{}'", name);

    let server = sqlx::query_as::<_, Server>(
        r#"
        SELECT * FROM server_config
        WHERE name = ?
        "#,
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch server")?;

    if let Some(ref s) = server {
        tracing::debug!("Found server '{}', type: {}", name, s.server_type);
    } else {
        tracing::debug!("No server found with name '{}'", name);
    }

    Ok(server)
}

/// Create or update a server in the database
pub async fn upsert_server(pool: &Pool<Sqlite>, server: &Server) -> Result<String> {
    tracing::debug!(
        "Upserting server '{}', type: {}",
        server.name,
        server.server_type
    );

    let mut tx = pool.begin().await.context("Failed to begin transaction")?;
    let server_id = upsert_server_tx(&mut tx, server).await?;
    tx.commit().await.context("Failed to commit transaction")?;

    Ok(server_id)
}

/// Create or update a server in the database (transaction version)
pub async fn upsert_server_tx(tx: &mut Transaction<'_, Sqlite>, server: &Server) -> Result<String> {
    // Generate a UUID for the server if it doesn't have one
    let server_id = if let Some(id) = &server.id {
        id.clone()
    } else {
        uuid::Uuid::new_v4().to_string()
    };

    let result = sqlx::query(
        r#"
        INSERT INTO server_config (id, name, server_type, command, url, transport_type)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(name) DO UPDATE SET
            server_type = excluded.server_type,
            command = excluded.command,
            url = excluded.url,
            transport_type = excluded.transport_type,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&server_id)
    .bind(&server.name)
    .bind(&server.server_type)
    .bind(&server.command)
    .bind(&server.url)
    .bind(&server.transport_type)
    .execute(&mut **tx)
    .await
    .context("Failed to upsert server")?;

    if result.rows_affected() == 0 {
        // If no rows were affected, get the existing ID
        let existing_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM server_config
            WHERE name = ?
            "#,
        )
        .bind(&server.name)
        .fetch_one(&mut **tx)
        .await
        .context("Failed to get server ID")?;

        return Ok(existing_id);
    }

    Ok(server_id)
}

/// Delete a server from the database
pub async fn delete_server(pool: &Pool<Sqlite>, name: &str) -> Result<bool> {
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

/// Get all arguments for a server from the database
pub async fn get_server_args(pool: &Pool<Sqlite>, server_id: &str) -> Result<Vec<ServerArg>> {
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
    upsert_server_args_tx(&mut tx, server_id, args).await?;
    tx.commit().await.context("Failed to commit transaction")?;

    Ok(())
}

/// Create or update server arguments in the database (transaction version)
pub async fn upsert_server_args_tx(
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

    // Insert new arguments
    for (index, arg) in args.iter().enumerate() {
        // Generate a UUID for the argument
        let arg_id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO server_args (id, server_id, arg_index, arg_value)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&arg_id)
        .bind(server_id)
        .bind(index as i32)
        .bind(arg)
        .execute(&mut **tx)
        .await
        .context("Failed to insert server argument")?;
    }

    Ok(())
}

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
    upsert_server_env_tx(&mut tx, server_id, env).await?;
    tx.commit().await.context("Failed to commit transaction")?;

    Ok(())
}

/// Create or update server environment variables in the database (transaction version)
pub async fn upsert_server_env_tx(
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

    // Insert new environment variables
    for (key, value) in env {
        // Generate a UUID for the environment variable
        let env_id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO server_env (id, server_id, env_key, env_value)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&env_id)
        .bind(server_id)
        .bind(key)
        .bind(value)
        .execute(&mut **tx)
        .await
        .context("Failed to insert server environment variable")?;
    }

    Ok(())
}

/// Get server metadata from the database
pub async fn get_server_meta(pool: &Pool<Sqlite>, server_id: &str) -> Result<Option<ServerMeta>> {
    tracing::debug!(
        "Executing SQL query to get metadata for server ID {}",
        server_id
    );

    let meta = sqlx::query_as::<_, ServerMeta>(
        r#"
        SELECT * FROM server_meta
        WHERE server_id = ?
        "#,
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch server metadata")?;

    if meta.is_some() {
        tracing::debug!("Found metadata for server ID {}", server_id);
    } else {
        tracing::debug!("No metadata found for server ID {}", server_id);
    }

    Ok(meta)
}

/// Create or update server metadata in the database
pub async fn upsert_server_meta(pool: &Pool<Sqlite>, meta: &ServerMeta) -> Result<String> {
    tracing::debug!("Upserting metadata for server ID {}", meta.server_id);

    // Generate a UUID for the metadata if it doesn't have one
    let meta_id = if let Some(id) = &meta.id {
        id.clone()
    } else {
        uuid::Uuid::new_v4().to_string()
    };

    let result = sqlx::query(
        r#"
        INSERT INTO server_meta (
            id, server_id, description, author, website, repository,
            category, recommended_scenario, rating
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(server_id) DO UPDATE SET
            description = excluded.description,
            author = excluded.author,
            website = excluded.website,
            repository = excluded.repository,
            category = excluded.category,
            recommended_scenario = excluded.recommended_scenario,
            rating = excluded.rating,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(&meta_id)
    .bind(&meta.server_id)
    .bind(&meta.description)
    .bind(&meta.author)
    .bind(&meta.website)
    .bind(&meta.repository)
    .bind(&meta.category)
    .bind(&meta.recommended_scenario)
    .bind(meta.rating)
    .execute(pool)
    .await
    .context("Failed to upsert server metadata")?;

    if result.rows_affected() == 0 {
        // If no rows were affected, get the existing ID
        let existing_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM server_meta
            WHERE server_id = ?
            "#,
        )
        .bind(&meta.server_id)
        .fetch_one(pool)
        .await
        .context("Failed to get server metadata ID")?;

        return Ok(existing_id);
    }

    Ok(meta_id)
}

/// Get all enabled servers from the database based on config suits
pub async fn get_enabled_servers(pool: &Pool<Sqlite>) -> Result<Vec<Server>> {
    tracing::debug!("Getting all enabled servers from config suits");

    // Get all servers first
    let all_servers = get_all_servers(pool).await?;

    // If there are no servers, return empty list
    if all_servers.is_empty() {
        return Ok(Vec::new());
    }

    // Get the default config suit
    let default_suit = crate::conf::operations::get_config_suit_by_name(pool, "default").await?;

    // If there's no default suit, return all servers (backward compatibility)
    if default_suit.is_none() {
        tracing::info!("No default config suit found, returning all servers");
        return Ok(all_servers);
    }

    let suit_id = default_suit.unwrap().id.unwrap();

    // Get all enabled servers in the default suit
    let enabled_server_configs =
        crate::conf::operations::get_config_suit_servers(pool, &suit_id).await?;

    // If there are no server configs in the suit, return all servers (backward compatibility)
    if enabled_server_configs.is_empty() {
        tracing::info!("No server configurations in default suit, returning all servers");
        return Ok(all_servers);
    }

    // Create a set of enabled server IDs
    let mut enabled_server_ids = HashSet::new();
    for server_config in enabled_server_configs {
        if server_config.enabled {
            enabled_server_ids.insert(server_config.server_id);
        }
    }

    // Filter servers by enabled status
    let enabled_servers: Vec<Server> = all_servers
        .into_iter()
        .filter(|server| {
            if let Some(id) = &server.id {
                enabled_server_ids.contains(id)
            } else {
                false // Server without ID is not enabled
            }
        })
        .collect();

    tracing::info!(
        "Found {} enabled servers in default config suit",
        enabled_servers.len()
    );

    Ok(enabled_servers)
}
