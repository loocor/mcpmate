use mcpmate_secrets::store::SecretOriginInput;
use sqlx::{Pool, Sqlite};

pub async fn prepare_config_database(pool: &Pool<Sqlite>) {
    mcpmate_migrations::prepare_config_database(pool, mcpmate_migrations::DatabaseSource::InMemory)
        .await
        .expect("prepare test config database");
}

pub async fn prepare_audit_database(pool: &Pool<Sqlite>) {
    mcpmate_migrations::prepare_audit_database(pool, mcpmate_migrations::DatabaseSource::InMemory)
        .await
        .expect("prepare test audit database");
}

/// Build a `SecretOriginInput` for an OAuth-managed secret slot.
pub fn oauth_secret_origin(
    server_id: &str,
    server_name: &str,
    field_key: &str,
) -> SecretOriginInput {
    SecretOriginInput {
        server_id: Some(server_id.to_string()),
        server_name: Some(server_name.to_string()),
        server_kind: Some("streamable_http".to_string()),
        source: Some("oauth".to_string()),
        field_group: Some("oauth".to_string()),
        field_key: Some(field_key.to_string()),
        field_index: None,
        field_path: Some(format!("oauth.{field_key}")),
    }
}
