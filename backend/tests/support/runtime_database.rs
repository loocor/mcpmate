use std::sync::Arc;

use mcpmate::{config::database::Database, config::initialization::run_initialization};
use mcpmate_capability_store::DerivedCapabilityCache;
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::TempDir;

#[path = "database.rs"]
mod database;

pub async fn open_database(temp_dir: &TempDir) -> Arc<Database> {
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect("sqlite::memory:")
        .await
        .expect("open test database");
    database::prepare_config(&pool).await;
    run_initialization(&pool).await.expect("initialize test database");
    Arc::new(Database {
        pool,
        path: temp_dir.path().join("runtime.db"),
        capability_cache: Arc::new(DerivedCapabilityCache::default()),
    })
}
