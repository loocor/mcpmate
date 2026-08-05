use mcpmate_migrations::{DatabaseSource, prepare_config_database};

pub async fn prepare_config(pool: &sqlx::SqlitePool) {
    prepare_config_database(pool, DatabaseSource::InMemory)
        .await
        .expect("prepare config database through migrations");
}
