use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use anyhow::{Context, Result};
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

use crate::common::paths::global_paths;

#[derive(Debug, Clone)]
pub struct AuditDatabase {
    pub pool: Pool<Sqlite>,
    pub path: PathBuf,
}

impl AuditDatabase {
    pub async fn new() -> Result<Self> {
        let database_url = global_paths().audit_database_url();
        let path = global_paths().audit_database_path();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create audit database directory: {}", parent.display()))?;
        }

        let options = SqliteConnectOptions::from_str(&database_url)
            .with_context(|| format!("Failed to parse audit database URL: {database_url}"))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5_000))
            .synchronous(SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .context("Failed to connect to audit SQLite database")?;

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .context("Failed to enable foreign keys for audit database")?;

        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&pool)
            .await
            .context("Failed to enable WAL mode for audit database")?;

        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&pool)
            .await
            .context("Failed to configure busy_timeout for audit database")?;

        Ok(Self { pool, path })
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::paths::MCPMatePaths;
    use tempfile::tempdir;

    fn with_temp_paths() -> (tempfile::TempDir, MCPMatePaths) {
        let dir = tempdir().expect("temp dir");
        let paths = MCPMatePaths::from_base_dir(dir.path()).expect("paths");
        (dir, paths)
    }

    #[tokio::test]
    async fn audit_database_creation() {
        let (_dir, paths) = with_temp_paths();
        let path = paths.audit_database_path();
        std::fs::create_dir_all(paths.base_dir()).expect("base dir");

        let database_url = paths.audit_database_url();
        let options = SqliteConnectOptions::from_str(&database_url)
            .expect("options")
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5_000))
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .expect("connect");

        sqlx::query("SELECT 1").execute(&pool).await.expect("query");
        assert!(path.exists());
        pool.close().await;
    }

    #[tokio::test]
    async fn audit_database_wal_mode() {
        let (_dir, paths) = with_temp_paths();
        std::fs::create_dir_all(paths.base_dir()).expect("base dir");
        let database_url = paths.audit_database_url();
        let options = SqliteConnectOptions::from_str(&database_url)
            .expect("options")
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5_000))
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .expect("connect");

        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .expect("journal mode");
        assert_eq!(mode.to_ascii_lowercase(), "wal");
        pool.close().await;
    }

    #[tokio::test]
    async fn audit_database_busy_timeout() {
        let (_dir, paths) = with_temp_paths();
        std::fs::create_dir_all(paths.base_dir()).expect("base dir");
        let database_url = paths.audit_database_url();
        let options = SqliteConnectOptions::from_str(&database_url)
            .expect("options")
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5_000))
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .expect("connect");

        let timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(&pool)
            .await
            .expect("busy timeout");
        assert_eq!(timeout, 5_000);
        pool.close().await;
    }

    #[test]
    fn audit_database_separate_from_config_db() {
        let (_dir, paths) = with_temp_paths();
        assert_ne!(paths.audit_database_path(), paths.database_path());
    }
}
