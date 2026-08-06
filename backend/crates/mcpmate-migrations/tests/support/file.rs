use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::path::Path;

pub async fn pool(path: &Path) -> SqlitePool {
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .expect("connect file-backed SQLite database");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("enable SQLite foreign keys");
    pool
}
