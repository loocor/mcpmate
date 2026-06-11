//! Database configuration loader for core MCPMate
//! Contains functions for loading configuration from the database - completely independent from core

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::{
    config::{
        database::Database,
        models::Server,
        server::{ServerEnabledService, get_server_args, get_server_env},
    },
    core::profile::merge::ProfileMerger,
    core::{
        models::{Config, MCPServerConfig},
        proxy::args::StartupMode,
        secrets::store::LocalSecretStore,
    },
};

fn empty_config() -> Config {
    Config {
        mcp_servers: HashMap::new(),
        pagination: None,
    }
}

async fn build_config_from_servers(
    db: &Database,
    servers: &[Server],
    secret_store: Option<Arc<LocalSecretStore>>,
) -> Result<Config> {
    let mut config = empty_config();
    let oauth_manager = crate::core::oauth::OAuthManager::new_optional_store(db.pool.clone(), secret_store);

    for server in servers {
        let args = if let Some(id) = &server.id {
            let server_args = get_server_args(&db.pool, id)
                .await
                .context("Failed to get server arguments")?;

            if server_args.is_empty() {
                None
            } else {
                let mut sorted_args: Vec<_> = server_args.into_iter().collect();
                sorted_args.sort_by_key(|arg| arg.arg_index);
                Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect())
            }
        } else {
            None
        };

        let env = if let Some(id) = &server.id {
            let env_map = get_server_env(&db.pool, id)
                .await
                .context("Failed to get server environment variables")?;

            if env_map.is_empty() { None } else { Some(env_map) }
        } else {
            None
        };

        let headers = if let Some(id) = &server.id {
            let manual_headers = match crate::config::server::get_server_headers(&db.pool, id).await {
                Ok(map) if !map.is_empty() => Some(map),
                _ => None,
            };
            oauth_manager
                .get_effective_server_headers(id, manual_headers)
                .await
                .context("Failed to get effective server headers")?
        } else {
            None
        };

        let server_config = MCPServerConfig {
            kind: server.server_type,
            command: server.command.clone(),
            args,
            url: server.url.clone(),
            env,
            headers,
        };

        if let Some(server_id) = &server.id {
            config.mcp_servers.insert(server_id.clone(), server_config);
        }
    }

    Ok(config)
}

async fn get_globally_enabled_servers(db: &Database) -> Result<Vec<Server>> {
    let mut servers = crate::config::server::get_all_servers(&db.pool)
        .await
        .context("Failed to load all servers from database")?;
    servers.retain(|server| server.id.is_some() && server.enabled.as_bool());
    servers.sort_by(|left, right| left.name.cmp(&right.name).then_with(|| left.id.cmp(&right.id)));
    Ok(servers)
}

/// Unified function to load servers from active profile
/// Returns both Server list and Config formats
pub async fn load_servers_from_active_profile(db: &Database) -> anyhow::Result<(Vec<Server>, Config)> {
    // Use ProfileMerger's merge logic
    let merger = ProfileMerger::new(Arc::new(db.clone()));
    let merge_result = merger
        .merge_all_configs()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to merge configurations: {}", e))?;

    let mut servers = Vec::new();
    for server_config in &merge_result.servers {
        if let Ok(Some(server)) = crate::config::server::get_server_by_id(&db.pool, &server_config.server_id).await {
            servers.push(server);
        }
    }
    let config = build_config_from_servers(db, &servers, None).await?;

    tracing::info!("Loaded {} servers from active profile (unified loader)", servers.len());

    Ok((servers, config))
}

/// Get enabled servers from all active profile using unified service
async fn get_enabled_servers_from_active_profile(pool: &sqlx::Pool<sqlx::Sqlite>) -> anyhow::Result<Vec<Server>> {
    // Use the unified server enabled service
    let service = ServerEnabledService::new(pool.clone());
    let servers = service.get_all_enabled_servers().await?;
    Ok(servers)
}

pub async fn load_pool_base_config(
    db: &Database,
    secret_store: Option<Arc<LocalSecretStore>>,
) -> Result<Config> {
    let servers = get_globally_enabled_servers(db).await?;
    let config = build_config_from_servers(db, &servers, secret_store).await?;

    tracing::info!(
        "Loaded {} globally enabled servers for pool base configuration",
        config.mcp_servers.len()
    );

    Ok(config)
}

pub async fn load_pool_base_config_with_params(
    db: &Database,
    startup_mode: &StartupMode,
    secret_store: Option<Arc<LocalSecretStore>>,
) -> Result<Config> {
    tracing::info!(
        "Loading pool base configuration from database with startup mode: {:?}",
        startup_mode
    );

    let servers = match startup_mode {
        StartupMode::Minimal | StartupMode::NoProfile => {
            tracing::info!("Minimal/NoProfile mode: not loading any pool servers");
            Vec::new()
        }
        StartupMode::Default => {
            tracing::info!("Default mode: loading pool base config from globally enabled servers");
            get_globally_enabled_servers(db).await?
        }
        StartupMode::SpecificProfile(profile_ids) => {
            tracing::info!(
                "Specific profile mode: loading pool servers from profile: {:?}",
                profile_ids
            );
            let service = ServerEnabledService::new(db.pool.clone());
            service
                .get_enabled_servers_from_profile(profile_ids)
                .await
                .context("Failed to get enabled servers from specific profile")?
        }
    };

    build_config_from_servers(db, &servers, secret_store).await
}

/// Load the MCP server configuration from the database with startup parameters
pub async fn load_server_config_with_params(
    db: &Database,
    startup_mode: &StartupMode,
) -> Result<Config> {
    tracing::info!(
        "Loading server configuration from database with startup mode: {:?}",
        startup_mode
    );

    // Get enabled servers based on startup mode
    let servers = match startup_mode {
        StartupMode::Minimal | StartupMode::NoProfile => {
            tracing::info!("Minimal/NoProfile mode: not loading any servers");
            Vec::new()
        }
        StartupMode::Default => {
            tracing::info!("Default mode: loading servers from all active profile");
            get_enabled_servers_from_active_profile(&db.pool)
                .await
                .context("Failed to get enabled servers from active profile")?
        }
        StartupMode::SpecificProfile(profile_ids) => {
            tracing::info!("Specific profile mode: loading servers from profile: {:?}", profile_ids);
            // Use unified service for specific profile
            let service = ServerEnabledService::new(db.pool.clone());
            service
                .get_enabled_servers_from_profile(profile_ids)
                .await
                .context("Failed to get enabled servers from specific profile")?
        }
    };

    let config = build_config_from_servers(db, &servers, None).await?;

    tracing::info!(
        "Successfully loaded {} enabled servers from database using core loader (mode: {:?})",
        config.mcp_servers.len(),
        startup_mode
    );

    // Publish ConfigReloaded event using core events
    crate::core::events::EventBus::global().publish(crate::core::events::Event::ConfigReloaded);
    tracing::info!("Published ConfigReloaded event using core events");

    Ok(config)
}

/// Load the MCP server configuration from the database (legacy function for backward compatibility)
pub async fn load_server_config(db: &Database) -> Result<Config> {
    load_server_config_with_params(db, &StartupMode::Default).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        initialization::run_initialization,
        models::{ServerOAuthConfig, ServerOAuthToken},
        server::{upsert_server_oauth_config, upsert_server_oauth_token},
    };
    use crate::core::secrets::store::{LocalSecretStore, SecretCreateInput, SecretKindInput};
    use crate::test_helpers::oauth_secret_origin;
    use chrono::{Duration, Utc};
    use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
    use tempfile::TempDir;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_string_contains, method, path},
    };

    async fn create_test_database() -> (TempDir, Database) {
        let temp_dir = TempDir::new().expect("temp dir");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite pool");

        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("enable foreign keys");
        run_initialization(&pool).await.expect("initialize schema");
        let db_path = temp_dir.path().join("test.db");

        (temp_dir, Database { pool, path: db_path })
    }

    async fn insert_server(
        pool: &SqlitePool,
        server_id: &str,
        name: &str,
        enabled: bool,
    ) {
        sqlx::query(
            r#"
            INSERT INTO server_config (id, name, server_type, command, enabled)
            VALUES (?, ?, 'stdio', 'demo-command', ?)
            "#,
        )
        .bind(server_id)
        .bind(name)
        .bind(enabled)
        .execute(pool)
        .await
        .expect("insert server");
    }

    async fn insert_http_server(
        pool: &SqlitePool,
        server_id: &str,
        name: &str,
        enabled: bool,
    ) {
        sqlx::query(
            r#"
            INSERT INTO server_config (id, name, server_type, url, enabled)
            VALUES (?, ?, 'streamable_http', 'https://example.com/mcp', ?)
            "#,
        )
        .bind(server_id)
        .bind(name)
        .bind(enabled)
        .execute(pool)
        .await
        .expect("insert http server");
    }

    #[tokio::test]
    async fn load_pool_base_config_uses_globally_enabled_servers_without_profile_merge() {
        let (_temp_dir, db) = create_test_database().await;

        insert_server(&db.pool, "server-global", "Global Server", true).await;

        let pool_config = load_pool_base_config(&db, None).await.expect("load pool base config");
        let (_, active_profile_config) = load_servers_from_active_profile(&db)
            .await
            .expect("load active-profile config");

        assert!(pool_config.mcp_servers.contains_key("server-global"));
        assert!(!active_profile_config.mcp_servers.contains_key("server-global"));
    }

    #[tokio::test]
    async fn load_pool_base_config_refreshes_expired_oauth_headers() {
        let (temp_dir, db) = create_test_database().await;
        let secret_store = Arc::new(
            LocalSecretStore::initialize_with_development_root_key(
                db.pool.clone(),
                temp_dir.path().join("secrets").join("local-root.key"),
            )
            .await
            .expect("initialize secret store"),
        );
        insert_http_server(&db.pool, "server-oauth", "OAuth Server", true).await;
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=refresh-123"))
            .and(body_string_contains("resource=https%3A%2F%2Fexample.com%2Fmcp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "access-new",
                "token_type": "bearer",
                "expires_in": 3600
            })))
            .mount(&mock_server)
            .await;

        upsert_server_oauth_config(
            &db.pool,
            &ServerOAuthConfig {
                id: None,
                server_id: "server-oauth".to_string(),
                authorization_endpoint: format!("{}/authorize", mock_server.uri()),
                token_endpoint: format!("{}/token", mock_server.uri()),
                client_id: "client-1".to_string(),
                client_secret: None,
                scopes: Some("read write".to_string()),
                redirect_uri: "http://localhost:5173/oauth/callback".to_string(),
                created_at: None,
                updated_at: None,
            },
        )
        .await
        .expect("save oauth config");
        let access_token = secret_store
            .create_secret(SecretCreateInput {
                alias: "oauth/server-oauth/access-token".to_string(),
                kind: SecretKindInput::OAuthAccessToken,
                value: "access-old".to_string(),
                label: Some("OAuth access token for OAuth Server".to_string()),
                origin: Some(oauth_secret_origin("server-oauth", "OAuth Server", "access-token")),
            })
            .await
            .expect("store access token")
            .placeholder;
        let refresh_token = secret_store
            .create_secret(SecretCreateInput {
                alias: "oauth/server-oauth/refresh-token".to_string(),
                kind: SecretKindInput::OAuthRefreshToken,
                value: "refresh-123".to_string(),
                label: Some("OAuth refresh token for OAuth Server".to_string()),
                origin: Some(oauth_secret_origin("server-oauth", "OAuth Server", "refresh-token")),
            })
            .await
            .expect("store refresh token")
            .placeholder;

        upsert_server_oauth_token(
            &db.pool,
            &ServerOAuthToken {
                id: None,
                server_id: "server-oauth".to_string(),
                access_token,
                refresh_token: Some(refresh_token),
                token_type: "bearer".to_string(),
                expires_at: Some((Utc::now() - Duration::minutes(1)).to_rfc3339()),
                scope: Some("read write".to_string()),
                created_at: None,
                updated_at: None,
            },
        )
        .await
        .expect("store expired token");

        let config = load_pool_base_config(&db, Some(secret_store))
            .await
            .expect("load pool config");
        let headers = config
            .mcp_servers
            .get("server-oauth")
            .and_then(|server| server.headers.as_ref())
            .expect("headers");

        assert_eq!(
            headers.get("authorization").map(String::as_str),
            Some("Bearer access-new")
        );
    }
}
