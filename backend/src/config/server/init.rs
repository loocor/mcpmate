// Server database initialization
// Contains functions for initializing server-related database tables

use anyhow::Result;
use sqlx::{Pool, Sqlite};
use tracing;

use crate::common::constants::database::tables;

/// Initialize all server-related database tables
pub async fn initialize_server_tables(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Initializing server-related database tables");

    create_server_config_table(pool).await?;
    create_server_args_table(pool).await?;
    create_server_env_table(pool).await?;
    create_server_headers_table(pool).await?;
    create_server_meta_table(pool).await?;
    create_server_oauth_config_table(pool).await?;
    create_server_oauth_tokens_table(pool).await?;

    verify_server_tables(pool).await?;
    cleanup_pending_import_servers(pool).await?;

    tracing::debug!("Server-related database tables initialized successfully");
    Ok(())
}

async fn cleanup_pending_import_servers(pool: &Pool<Sqlite>) -> Result<()> {
    let result = sqlx::query(
        r#"
        DELETE FROM server_config
        WHERE pending_import = 1
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to clean pending_import server records: {}", e);
        anyhow::anyhow!("Failed to clean pending_import server records: {}", e)
    })?;

    let removed = result.rows_affected();
    if removed > 0 {
        tracing::info!(removed, "Removed stale pending_import server records during startup");
    }

    Ok(())
}

/// Create server_config table if it doesn't exist
async fn create_server_config_table(pool: &Pool<Sqlite>) -> Result<()> {
    use crate::common::constants::transport;

    tracing::debug!("Creating server_config table if it doesn't exist");

    let create_sql = format!(
        r#"
        CREATE TABLE IF NOT EXISTS server_config (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            server_type TEXT NOT NULL CHECK (
                server_type IN ('{}', '{}', '{}')
            ),
            command TEXT,
            url TEXT,
            registry_server_id TEXT UNIQUE,
            capabilities TEXT,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            unify_direct_exposure_eligible BOOLEAN NOT NULL DEFAULT 0,
            pending_import BOOLEAN NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        transport::STDIO,
        transport::SSE,
        transport::STREAMABLE_HTTP
    );

    sqlx::query(&create_sql).execute(pool).await.map_err(|e| {
        tracing::error!("Failed to create server_config table: {}", e);
        anyhow::anyhow!("Failed to create server_config table: {}", e)
    })?;

    tracing::debug!("server_config table created or already exists");
    ensure_column(pool, "server_config", "pending_import", "BOOLEAN NOT NULL DEFAULT 0").await?;
    ensure_column(
        pool,
        "server_config",
        "unify_direct_exposure_eligible",
        "BOOLEAN NOT NULL DEFAULT 0",
    )
    .await?;
    Ok(())
}

/// Create server_args table if it doesn't exist
async fn create_server_args_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_args table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_args (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            arg_index INTEGER NOT NULL,
            arg_value TEXT NOT NULL,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, arg_index)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_args table: {}", e);
        anyhow::anyhow!("Failed to create server_args table: {}", e)
    })?;

    tracing::debug!("server_args table created or already exists");
    Ok(())
}

/// Create server_env table if it doesn't exist
async fn create_server_env_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_env table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_env (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            env_key TEXT NOT NULL,
            env_value TEXT NOT NULL,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, env_key)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_env table: {}", e);
        anyhow::anyhow!("Failed to create server_env table: {}", e)
    })?;

    tracing::debug!("server_env table created or already exists");
    Ok(())
}

/// Create server_headers table (HTTP default headers) if it doesn't exist
async fn create_server_headers_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_headers table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_headers (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            header_key TEXT NOT NULL,
            header_value TEXT NOT NULL,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id, header_key)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_headers table: {}", e);
        anyhow::anyhow!("Failed to create server_headers table: {}", e)
    })?;

    tracing::debug!("server_headers table created or already exists");
    Ok(())
}

/// Create server_meta table if it doesn't exist
async fn create_server_meta_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_meta table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_meta (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            server_name TEXT NOT NULL,
            author TEXT,
            category TEXT,
            description TEXT,
            extras_json TEXT,
            icons_json TEXT,
            protocol_version TEXT,
            rating INTEGER,
            recommended_scenario TEXT,
            registry_meta_json TEXT,
            registry_version TEXT,
            repository TEXT,
            server_version TEXT,
            website TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE,
            UNIQUE(server_id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_meta table: {}", e);
        anyhow::anyhow!("Failed to create server_meta table: {}", e)
    })?;

    tracing::debug!("server_meta table created or already exists");

    // Backfill new columns when upgrading an existing development database
    ensure_column(pool, "server_meta", "registry_version", "TEXT").await?;
    ensure_column(pool, "server_meta", "registry_meta_json", "TEXT").await?;
    ensure_column(pool, "server_meta", "extras_json", "TEXT").await?;

    Ok(())
}

async fn create_server_oauth_config_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_oauth_config table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_oauth_config (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL UNIQUE,
            authorization_endpoint TEXT NOT NULL,
            token_endpoint TEXT NOT NULL,
            client_id TEXT NOT NULL,
            client_secret TEXT,
            scopes TEXT,
            redirect_uri TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_oauth_config table: {}", e);
        anyhow::anyhow!("Failed to create server_oauth_config table: {}", e)
    })?;

    Ok(())
}

async fn create_server_oauth_tokens_table(pool: &Pool<Sqlite>) -> Result<()> {
    tracing::debug!("Creating server_oauth_tokens table if it doesn't exist");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS server_oauth_tokens (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL UNIQUE,
            access_token TEXT NOT NULL,
            refresh_token TEXT,
            token_type TEXT NOT NULL DEFAULT 'bearer',
            expires_at TEXT,
            scope TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES server_config (id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create server_oauth_tokens table: {}", e);
        anyhow::anyhow!("Failed to create server_oauth_tokens table: {}", e)
    })?;

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
        Ok(_) => {
            tracing::debug!("Added column {}.{}", table, column);
            Ok(())
        }
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column name") => {
            tracing::trace!("Column {}.{} already exists", table, column);
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to add column {}.{}: {}", table, column, e);
            Err(anyhow::anyhow!("Failed to add column {}.{}: {}", table, column, e))
        }
    }
}

/// Verify that all server tables were created successfully
async fn verify_server_tables(pool: &Pool<Sqlite>) -> Result<()> {
    for table in [
        tables::SERVER_CONFIG,
        tables::SERVER_ARGS,
        tables::SERVER_ENV,
        tables::SERVER_HEADERS,
        tables::SERVER_META,
        tables::SERVER_OAUTH_CONFIG,
        tables::SERVER_OAUTH_TOKENS,
    ] {
        sqlx::query(&format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"
        ))
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to verify {} table: {}", table, e);
            anyhow::anyhow!("Failed to verify {} table: {}", table, e)
        })?
        .ok_or_else(|| {
            let err = format!("{table} table not found after creation");
            tracing::error!("{}", err);
            anyhow::anyhow!(err)
        })?;

        tracing::debug!("Verified {} table exists", table);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        common::{server::ServerType, status::EnabledStatus},
        config::{models::Server, server::crud::upsert_server},
    };

    async fn setup_pool() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable foreign keys");
        pool
    }

    fn build_server(
        id: &str,
        name: &str,
        pending_import: bool,
    ) -> Server {
        Server {
            id: Some(id.to_string()),
            name: name.to_string(),
            server_type: ServerType::StreamableHttp,
            command: None,
            url: Some(format!("https://example.com/{name}")),
            registry_server_id: None,
            capabilities: None,
            enabled: EnabledStatus::Enabled,
            unify_direct_exposure_eligible: false,
            pending_import,
            created_at: None,
            updated_at: None,
        }
    }

    #[tokio::test]
    async fn initialize_server_tables_removes_pending_import_records() {
        let pool = setup_pool().await;
        initialize_server_tables(&pool).await.expect("initialize tables");

        upsert_server(&pool, &build_server("serv_visible", "visible-server", false))
            .await
            .expect("insert visible server");
        upsert_server(&pool, &build_server("serv_pending", "pending-server", true))
            .await
            .expect("insert pending server");

        initialize_server_tables(&pool)
            .await
            .expect("reinitialize tables and cleanup pending records");

        let remaining_names = sqlx::query_scalar::<_, String>("SELECT name FROM server_config ORDER BY name ASC")
            .fetch_all(&pool)
            .await
            .expect("list remaining servers");

        assert_eq!(remaining_names, vec!["visible-server".to_string()]);
    }
}
