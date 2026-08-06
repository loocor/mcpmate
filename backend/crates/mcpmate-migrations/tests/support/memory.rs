use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

pub async fn pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect in-memory SQLite database");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("enable SQLite foreign keys");
    pool
}
