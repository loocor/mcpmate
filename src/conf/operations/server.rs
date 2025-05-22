// Server operations for MCPMate
// Contains CRUD operations for server configuration

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use nanoid::nanoid;
use sqlx::{Pool, Sqlite, Transaction};

use crate::conf::{
    models::{Server, ServerArg, ServerEnv, ServerMeta},
    operations::utils::{get_server_name, get_server_name_with_tx},
};

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

/// Get a specific server from the database by name
pub async fn get_server(
    pool: &Pool<Sqlite>,
    name: &str,
) -> Result<Option<Server>> {
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

/// Get a specific server from the database by ID
pub async fn get_server_by_id(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<Option<Server>> {
    tracing::debug!("Executing SQL query to get server with ID '{}'", id);

    let server = sqlx::query_as::<_, Server>(
        r#"
        SELECT * FROM server_config
        WHERE id = ?
        "#,
    )
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
pub async fn upsert_server_tx(
    tx: &mut Transaction<'_, Sqlite>,
    server: &Server,
) -> Result<String> {
    // Generate an ID for the server if it doesn't have one
    let server_id = if let Some(id) = &server.id {
        id.clone()
    } else {
        format!("serv{}", nanoid!(12))
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
    .bind(server.server_type)
    .bind(&server.command)
    .bind(&server.url)
    .bind(server.transport_type)
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

/// Get server metadata from the database
pub async fn get_server_meta(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<Option<ServerMeta>> {
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
pub async fn upsert_server_meta(
    pool: &Pool<Sqlite>,
    meta: &ServerMeta,
) -> Result<String> {
    tracing::debug!("Upserting metadata for server ID {}", meta.server_id);

    // Generate an ID for the metadata if it doesn't have one
    let meta_id = if let Some(id) = &meta.id {
        id.clone()
    } else {
        format!("smet{}", nanoid!(12))
    };

    // Get the server name
    let server_name = get_server_name(pool, &meta.server_id).await?;

    let result = sqlx::query(
        r#"
        INSERT INTO server_meta (
            id, server_id, server_name, description, author, website, repository,
            category, recommended_scenario, rating
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(server_id) DO UPDATE SET
            server_name = excluded.server_name,
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
    .bind(&server_name)
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

    // Get all active config suits
    let active_suits = crate::conf::operations::suit::get_active_config_suits(pool).await?;

    // If there are no active suits, try to get the default suit
    if active_suits.is_empty() {
        let default_suit = crate::conf::operations::suit::get_default_config_suit(pool).await?;

        // If there's no default suit, try the legacy "default" named suit
        if default_suit.is_none() {
            let legacy_default =
                crate::conf::operations::get_config_suit_by_name(pool, "default").await?;

            // If there's no legacy default suit either, return no servers (whitelist mode)
            if legacy_default.is_none() {
                tracing::info!(
                    "No active or default config suits found, returning no servers (whitelist mode)"
                );
                return Ok(Vec::new());
            }

            // Use the legacy default suit
            let suit_id = legacy_default.unwrap().id.unwrap();
            return get_enabled_servers_from_suit(pool, &suit_id, &all_servers).await;
        }

        // Use the default suit
        let suit_id = default_suit.unwrap().id.unwrap();
        return get_enabled_servers_from_suit(pool, &suit_id, &all_servers).await;
    }

    // Create a map to track enabled server IDs with their priority
    // Higher priority value means higher precedence
    let mut enabled_server_map: HashMap<String, (bool, i32)> = HashMap::new();

    // Process all active suits in priority order (already sorted by priority DESC)
    for suit in active_suits {
        if let Some(suit_id) = &suit.id {
            // Get all server configs in this suit
            let server_configs =
                crate::conf::operations::get_config_suit_servers(pool, suit_id).await?;

            // Process each server config
            for server_config in server_configs {
                // Only update the map if this server hasn't been seen yet or if the current suit has higher priority
                if !enabled_server_map.contains_key(&server_config.server_id)
                    || enabled_server_map.get(&server_config.server_id).unwrap().1 < suit.priority
                {
                    enabled_server_map.insert(
                        server_config.server_id.clone(),
                        (server_config.enabled, suit.priority),
                    );
                }
            }
        }
    }

    // If no server configurations were found in any active suits, return no servers (whitelist mode)
    if enabled_server_map.is_empty() {
        tracing::info!(
            "No server configurations in any active suits, returning no servers (whitelist mode)"
        );
        return Ok(Vec::new());
    }

    // Filter servers by enabled status
    let enabled_servers: Vec<Server> = all_servers
        .into_iter()
        .filter(|server| {
            if let Some(id) = &server.id {
                // Check both the suit-level enabled status AND the global enabled status
                enabled_server_map
                    .get(id)
                    .is_some_and(|(enabled, _)| *enabled)
                    && server.enabled.as_bool() // Add this check for global enabled status
            } else {
                false // Server without ID is not enabled
            }
        })
        .collect();

    tracing::info!(
        "Found {} enabled servers across all active config suits",
        enabled_servers.len()
    );

    Ok(enabled_servers)
}

/// Helper function to get enabled servers from a specific suit
async fn get_enabled_servers_from_suit(
    pool: &Pool<Sqlite>,
    suit_id: &str,
    all_servers: &[Server],
) -> Result<Vec<Server>> {
    // Get all enabled servers in the suit
    let server_configs = crate::conf::operations::get_config_suit_servers(pool, suit_id).await?;

    // If there are no server configs in the suit, return no servers (whitelist mode)
    if server_configs.is_empty() {
        tracing::info!(
            "No server configurations in suit {}, returning no servers (whitelist mode)",
            suit_id
        );
        return Ok(Vec::new());
    }

    // Create a set of enabled server IDs
    let mut enabled_server_ids = HashSet::new();
    for server_config in server_configs {
        if server_config.enabled {
            enabled_server_ids.insert(server_config.server_id);
        }
    }

    // Filter servers by enabled status
    let enabled_servers: Vec<Server> = all_servers
        .iter()
        .filter(|server| {
            if let Some(id) = &server.id {
                // Check both the suit-level enabled status AND the global enabled status
                enabled_server_ids.contains(id) && server.enabled.as_bool()
            } else {
                false // Server without ID is not enabled
            }
        })
        .cloned()
        .collect();

    tracing::info!(
        "Found {} enabled servers in suit {}",
        enabled_servers.len(),
        suit_id
    );

    Ok(enabled_servers)
}

/// Check if a server is enabled in any active config suit
///
/// This function checks if a server is enabled in any active config suit.
/// Returns true if the server is enabled in at least one active suit, false otherwise.
pub async fn is_server_enabled_in_any_suit(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<bool> {
    // Get all active config suits
    let active_suits = crate::conf::operations::suit::get_active_config_suits(pool).await?;

    // If there are no active suits, try to get the default suit
    if active_suits.is_empty() {
        let default_suit = crate::conf::operations::suit::get_default_config_suit(pool).await?;

        // If there's no default suit, try the legacy "default" named suit
        if default_suit.is_none() {
            let legacy_default =
                crate::conf::operations::get_config_suit_by_name(pool, "default").await?;

            // If there's no legacy default suit either, return false (not enabled)
            if legacy_default.is_none() {
                tracing::debug!("No active or default config suits found, server is not enabled");
                return Ok(false);
            }

            // Use the legacy default suit
            let suit_id = legacy_default.unwrap().id.unwrap();
            return is_server_enabled_in_suit(pool, server_id, &suit_id).await;
        }

        // Use the default suit
        let suit_id = default_suit.unwrap().id.unwrap();
        return is_server_enabled_in_suit(pool, server_id, &suit_id).await;
    }

    // Check each active suit
    for suit in active_suits {
        if let Some(suit_id) = &suit.id {
            if is_server_enabled_in_suit(pool, server_id, suit_id).await? {
                return Ok(true);
            }
        }
    }

    // Server is not enabled in any active suit
    Ok(false)
}

/// Check if a server is enabled in a specific config suit
///
/// This function checks if a server is enabled in a specific config suit.
/// Returns true if the server is enabled in the suit, false otherwise.
async fn is_server_enabled_in_suit(
    pool: &Pool<Sqlite>,
    server_id: &str,
    suit_id: &str,
) -> Result<bool> {
    // Get all server configs in this suit
    let server_configs = crate::conf::operations::get_config_suit_servers(pool, suit_id).await?;

    // Check if the server is enabled in this suit
    for server_config in server_configs {
        if server_config.server_id == server_id {
            // We found the server in this suit, now we need to check the global status
            let server = get_server_by_id(pool, server_id).await?;
            if let Some(server) = server {
                // Return true only if both the suit-level and global status are enabled
                return Ok(server_config.enabled && server.enabled.as_bool());
            }
            // If we couldn't find the server (shouldn't happen), just return the suit status
            return Ok(server_config.enabled);
        }
    }

    // Server is not in this suit, so it's not enabled
    Ok(false)
}

/// Check if a server is in a specific config suit
///
/// This function checks if a server is in a specific config suit, regardless of enabled status.
/// Returns true if the server is in the suit, false otherwise.
pub async fn is_server_in_suit(
    pool: &Pool<Sqlite>,
    server_id: &str,
    suit_id: &str,
) -> Result<bool> {
    // Get all server configs in this suit
    let server_configs = crate::conf::operations::get_config_suit_servers(pool, suit_id).await?;

    // Check if the server is in this suit
    for server_config in server_configs {
        if server_config.server_id == server_id {
            return Ok(true);
        }
    }

    // Server is not in this suit
    Ok(false)
}

/// Update a server's global enabled status
///
/// This function updates the global enabled status of a server in the database.
/// Returns true if the server was updated, false if the server was not found.
/// If the status is updated, it also publishes a ServerGlobalStatusChanged event.
pub async fn update_server_global_status(
    pool: &Pool<Sqlite>,
    server_id: &str,
    enabled: bool,
) -> Result<bool> {
    tracing::debug!(
        "Updating global enabled status for server ID {} to {}",
        server_id,
        enabled
    );

    let result = sqlx::query(
        r#"
        UPDATE server_config
        SET enabled = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(enabled)
    .bind(server_id)
    .execute(pool)
    .await
    .context("Failed to update server global status")?;

    let updated = result.rows_affected() > 0;

    // If the server was updated, publish an event
    if updated {
        // Get the server name
        if let Ok(Some(server)) = get_server_by_id(pool, server_id).await {
            // Publish the event
            crate::core::events::EventBus::global().publish(
                crate::core::events::Event::ServerGlobalStatusChanged {
                    server_id: server_id.to_string(),
                    server_name: server.name,
                    enabled,
                },
            );

            tracing::info!(
                "Published ServerGlobalStatusChanged event for server ID {} ({})",
                server_id,
                enabled
            );
        }
    }

    Ok(updated)
}

/// Get a server's global enabled status
///
/// This function retrieves the global enabled status of a server from the database.
/// Returns Some(bool) if the server was found, None if the server was not found.
pub async fn get_server_global_status(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<Option<bool>> {
    tracing::debug!("Getting global enabled status for server ID {}", server_id);

    let enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM server_config
        WHERE id = ?
        "#,
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get server global status")?;

    Ok(enabled)
}
