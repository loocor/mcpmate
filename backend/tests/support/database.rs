use mcpmate_migrations::{DatabaseSource, prepare_config_database};
use sqlx::SqlitePool;

use mcpmate::config::models::Profile;

pub async fn prepare_config(pool: &sqlx::SqlitePool) {
    prepare_config_database(pool, DatabaseSource::InMemory)
        .await
        .expect("prepare config database through migrations");
}

#[allow(dead_code)]
pub async fn insert_profile(
    pool: &SqlitePool,
    profile: &Profile,
) -> String {
    let profile_id = profile.id.clone().unwrap_or_else(|| mcpmate::generate_id!("prof"));
    sqlx::query(
        r#"
        INSERT INTO profile (
            id, name, description, type, role, multi_select,
            priority, is_active, is_default, authoring_generation
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&profile_id)
    .bind(&profile.name)
    .bind(&profile.description)
    .bind(profile.profile_type)
    .bind(profile.role)
    .bind(profile.multi_select)
    .bind(profile.priority)
    .bind(profile.is_active)
    .bind(profile.is_default)
    .bind(profile.authoring_generation)
    .execute(pool)
    .await
    .expect("insert test Profile");
    profile_id
}

#[allow(dead_code)]
pub async fn insert_profile_server_relationship(
    pool: &SqlitePool,
    profile_id: &str,
    server_id: &str,
    enabled: bool,
) {
    sqlx::query(
        r#"
        INSERT INTO profile_server_relationships
            (profile_id, server_id, enabled, new_ref_policy)
        VALUES (?, ?, ?, 'follow')
        "#,
    )
    .bind(profile_id)
    .bind(server_id)
    .bind(enabled)
    .execute(pool)
    .await
    .expect("insert test Profile server relationship");
}
