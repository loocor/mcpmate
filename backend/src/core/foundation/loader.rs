//! Database configuration loader for core MCPMate
//! Contains functions for loading configuration from the database - completely independent from core

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::{
    common::server::ServerType,
    config::{
        database::Database,
        models::Server,
        server::{ServerEnabledService, get_server_args, get_server_env, headers::is_authorization_header_key},
    },
    core::profile::merge::ProfileMerger,
    core::{
        models::{Config, MCPServerConfig},
        oauth::OAuthManager,
        proxy::args::StartupMode,
        secrets::store::LocalSecretStore,
    },
};

type DegradedLoad<T> = (T, bool);

const STARTUP_DIAGNOSTIC_COMPONENT: &str = "startup_loader";
const STARTUP_DIAGNOSTIC_PHASE: &str = "pool_base_config_load";

fn empty_config() -> Config {
    Config {
        mcp_servers: HashMap::new(),
        pagination: None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConfigBuildPolicy {
    Strict,
    /// Startup pool base load: per-server field failures warn and degrade instead of aborting core.
    /// Structural failures (for example stdio args) skip the server entry instead of inserting
    /// a broken configuration that would fail unpredictably at connection time.
    DegradePerServer,
}

impl ConfigBuildPolicy {
    fn degrades(self) -> bool {
        self == Self::DegradePerServer
    }
}

#[derive(Clone, Copy)]
enum StartupServerSource {
    GlobalPool,
    ActiveProfile,
}

#[derive(Clone, Copy)]
struct StartupSkipReason {
    code: &'static str,
    detail: &'static str,
}

fn startup_skip_reason(
    server: &Server,
    args_degraded: bool,
    env_degraded: bool,
    headers_degraded: bool,
) -> Option<StartupSkipReason> {
    if env_degraded {
        return Some(StartupSkipReason {
            code: "server_env_unavailable",
            detail: "server environment variables could not be loaded",
        });
    }

    if headers_degraded {
        return Some(StartupSkipReason {
            code: "server_headers_unavailable",
            detail: "server headers could not be safely materialized",
        });
    }

    if server.server_type == ServerType::Stdio {
        if args_degraded {
            return Some(StartupSkipReason {
                code: "stdio_args_unavailable",
                detail: "stdio server arguments could not be loaded",
            });
        }
        if server.command.as_deref().is_none_or(str::is_empty) {
            return Some(StartupSkipReason {
                code: "stdio_command_missing",
                detail: "stdio server has no command configured",
            });
        }
    }

    if matches!(server.server_type, ServerType::Sse | ServerType::StreamableHttp)
        && server.url.as_deref().is_none_or(str::is_empty)
    {
        return Some(StartupSkipReason {
            code: "remote_url_missing",
            detail: "remote server has no URL configured",
        });
    }

    if args_degraded {
        return Some(StartupSkipReason {
            code: "server_args_unavailable",
            detail: "server arguments could not be loaded",
        });
    }

    None
}

fn has_manual_authorization(headers: Option<&HashMap<String, String>>) -> bool {
    headers.is_some_and(|headers| headers.keys().any(|key| is_authorization_header_key(key)))
}

fn warn_degraded_server_field(
    server_id: &str,
    server_name: &str,
    degraded_field: &'static str,
    reason_code: &'static str,
    action_taken: &'static str,
    error: impl std::fmt::Display,
    message: &'static str,
) {
    tracing::warn!(
        component = STARTUP_DIAGNOSTIC_COMPONENT,
        phase = STARTUP_DIAGNOSTIC_PHASE,
        server_id = %server_id,
        server_name = %server_name,
        degraded = true,
        startup_continues = true,
        server_startup_allowed = true,
        degraded_field,
        reason_code,
        action_taken,
        error = %error,
        "{message}"
    );
}

fn warn_omit_server_from_startup(
    server_id: &str,
    server_name: &str,
    reason: StartupSkipReason,
) {
    tracing::warn!(
        component = STARTUP_DIAGNOSTIC_COMPONENT,
        phase = STARTUP_DIAGNOSTIC_PHASE,
        server_id = %server_id,
        server_name = %server_name,
        degraded = true,
        startup_continues = true,
        server_startup_allowed = false,
        degraded_field = "server_config",
        action_taken = "omit_server_from_startup_pool",
        reason_code = reason.code,
        detail = reason.detail,
        "Omitting server from startup pool configuration"
    );
}

fn degraded_load_error<T>(
    server_id: &str,
    server_name: &str,
    degrade: bool,
    degraded_field: &'static str,
    reason_code: &'static str,
    warn_message: &'static str,
    error: anyhow::Error,
    error_context: &'static str,
) -> Result<DegradedLoad<Option<T>>> {
    if degrade {
        warn_degraded_server_field(
            server_id,
            server_name,
            degraded_field,
            reason_code,
            "skip_server_field",
            &error,
            warn_message,
        );
        Ok((None, true))
    } else {
        Err(error).context(error_context)
    }
}

fn optional_nonempty_map(map: HashMap<String, String>) -> Option<HashMap<String, String>> {
    if map.is_empty() { None } else { Some(map) }
}

async fn load_optional_string_map(
    server_id: &str,
    server_name: &str,
    degrade: bool,
    degraded_field: &'static str,
    reason_code: &'static str,
    warn_message: &'static str,
    error_context: &'static str,
    load: impl std::future::Future<Output = Result<HashMap<String, String>>>,
) -> Result<DegradedLoad<Option<HashMap<String, String>>>> {
    match load.await {
        Ok(map) => Ok((optional_nonempty_map(map), false)),
        Err(error) => degraded_load_error(
            server_id,
            server_name,
            degrade,
            degraded_field,
            reason_code,
            warn_message,
            error,
            error_context,
        ),
    }
}

async fn load_server_args(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    server_id: &str,
    server_name: &str,
    degrade: bool,
) -> Result<DegradedLoad<Option<Vec<String>>>> {
    match get_server_args(pool, server_id).await {
        Ok(server_args) if server_args.is_empty() => Ok((None, false)),
        Ok(server_args) => {
            let mut sorted_args: Vec<_> = server_args.into_iter().collect();
            sorted_args.sort_by_key(|arg| arg.arg_index);
            Ok((Some(sorted_args.into_iter().map(|arg| arg.arg_value).collect()), false))
        }
        Err(error) => degraded_load_error(
            server_id,
            server_name,
            degrade,
            "args",
            "server_args_load_failed",
            "Skipping server arguments while loading startup configuration",
            error,
            "Failed to get server arguments",
        ),
    }
}

async fn load_server_env(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    server_id: &str,
    server_name: &str,
    degrade: bool,
) -> Result<DegradedLoad<Option<HashMap<String, String>>>> {
    load_optional_string_map(
        server_id,
        server_name,
        degrade,
        "env",
        "server_env_load_failed",
        "Skipping server environment variables while loading startup configuration",
        "Failed to get server environment variables",
        get_server_env(pool, server_id),
    )
    .await
}

async fn load_server_headers(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    oauth_manager: &OAuthManager,
    server_id: &str,
    server_name: &str,
    degrade: bool,
) -> Result<DegradedLoad<Option<HashMap<String, String>>>> {
    let (manual_headers, manual_headers_degraded) = load_optional_string_map(
        server_id,
        server_name,
        degrade,
        "manual_headers",
        "server_headers_load_failed",
        "Skipping server headers while loading startup configuration",
        "Failed to get server headers",
        crate::config::server::get_server_headers(pool, server_id),
    )
    .await?;
    if manual_headers_degraded {
        return Ok((None, true));
    }

    match oauth_manager
        .get_effective_server_headers(server_id, manual_headers.clone())
        .await
    {
        Ok(headers) => Ok((headers, false)),
        Err(error) if degrade => {
            let action_taken = if has_manual_authorization(manual_headers.as_ref()) {
                "preserve_manual_authorization_headers"
            } else {
                "omit_server_from_startup_pool"
            };
            warn_degraded_server_field(
                server_id,
                server_name,
                "oauth_headers",
                "oauth_header_injection_failed",
                action_taken,
                &error,
                "Skipping OAuth header injection while loading startup configuration",
            );
            Ok((
                manual_headers.clone(),
                !has_manual_authorization(manual_headers.as_ref()),
            ))
        }
        Err(error) => Err(error).context("Failed to get effective server headers"),
    }
}

async fn build_config_from_servers(
    db: &Database,
    servers: &[Server],
    secret_store: Option<Arc<LocalSecretStore>>,
    build_policy: ConfigBuildPolicy,
) -> Result<Config> {
    let mut config = empty_config();
    let oauth_manager = OAuthManager::new_optional_store(db.pool.clone(), secret_store);
    let degrade = build_policy.degrades();

    for server in servers {
        let Some(server_id) = server.id.as_ref() else {
            continue;
        };

        let (args, args_degraded) = load_server_args(&db.pool, server_id, &server.name, degrade).await?;
        let (env, env_degraded) = load_server_env(&db.pool, server_id, &server.name, degrade).await?;
        let (headers, headers_degraded) =
            load_server_headers(&db.pool, &oauth_manager, server_id, &server.name, degrade).await?;

        if degrade && let Some(reason) = startup_skip_reason(server, args_degraded, env_degraded, headers_degraded) {
            warn_omit_server_from_startup(server_id, &server.name, reason);
            continue;
        }

        config.mcp_servers.insert(
            server_id.clone(),
            MCPServerConfig {
                kind: server.server_type,
                command: server.command.clone(),
                args,
                url: server.url.clone(),
                env,
                headers,
            },
        );
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

async fn servers_for_startup_mode(
    db: &Database,
    startup_mode: &StartupMode,
    source: StartupServerSource,
) -> Result<Vec<Server>> {
    match startup_mode {
        StartupMode::Minimal | StartupMode::NoProfile => Ok(Vec::new()),
        StartupMode::Default => match source {
            StartupServerSource::GlobalPool => get_globally_enabled_servers(db).await,
            StartupServerSource::ActiveProfile => get_enabled_servers_from_active_profile(&db.pool)
                .await
                .context("Failed to get enabled servers from active profile"),
        },
        StartupMode::SpecificProfile(profile_ids) => ServerEnabledService::new(db.pool.clone())
            .get_enabled_servers_from_profile(profile_ids)
            .await
            .context("Failed to get enabled servers from specific profile"),
    }
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
    let config = build_config_from_servers(db, &servers, None, ConfigBuildPolicy::Strict).await?;

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
    let config = build_config_from_servers(db, &servers, secret_store, ConfigBuildPolicy::Strict).await?;

    tracing::info!(
        "Loaded {} globally enabled servers for pool base configuration",
        config.mcp_servers.len()
    );

    Ok(config)
}

/// Load pool base configuration for core startup.
///
/// Uses [`ConfigBuildPolicy::DegradePerServer`]: per-server args, env, and header/OAuth failures
/// warn and degrade instead of aborting core initialization. Servers that would be structurally
/// unsafe to run (for example stdio without loadable args) are omitted from the startup config.
/// Reload paths keep [`ConfigBuildPolicy::Strict`].
pub async fn load_pool_base_config_with_params(
    db: &Database,
    startup_mode: &StartupMode,
    secret_store: Option<Arc<LocalSecretStore>>,
) -> Result<Config> {
    tracing::info!(
        "Loading pool base configuration from database with startup mode: {:?}",
        startup_mode
    );

    match startup_mode {
        StartupMode::Minimal | StartupMode::NoProfile => {
            tracing::info!("Minimal/NoProfile mode: not loading any pool servers");
        }
        StartupMode::Default => {
            tracing::info!("Default mode: loading pool base config from globally enabled servers");
        }
        StartupMode::SpecificProfile(profile_ids) => {
            tracing::info!(
                "Specific profile mode: loading pool servers from profile: {:?}",
                profile_ids
            );
        }
    }

    let servers = servers_for_startup_mode(db, startup_mode, StartupServerSource::GlobalPool).await?;
    build_config_from_servers(db, &servers, secret_store, ConfigBuildPolicy::DegradePerServer).await
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

    match startup_mode {
        StartupMode::Minimal | StartupMode::NoProfile => {
            tracing::info!("Minimal/NoProfile mode: not loading any servers");
        }
        StartupMode::Default => {
            tracing::info!("Default mode: loading servers from all active profile");
        }
        StartupMode::SpecificProfile(profile_ids) => {
            tracing::info!("Specific profile mode: loading servers from profile: {:?}", profile_ids);
        }
    }

    let servers = servers_for_startup_mode(db, startup_mode, StartupServerSource::ActiveProfile).await?;
    let config = build_config_from_servers(db, &servers, None, ConfigBuildPolicy::Strict).await?;

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
    async fn startup_pool_base_config_omits_stdio_server_when_args_cannot_be_loaded() {
        let (_temp_dir, db) = create_test_database().await;
        insert_server(&db.pool, "server-stdio-args", "Stdio Args Server", true).await;
        sqlx::query(
            r#"
            INSERT INTO server_args (id, server_id, server_name, arg_index, arg_value)
            VALUES ('arg-1', 'server-stdio-args', 'Stdio Args Server', 0, 'server.js')
            "#,
        )
        .execute(&db.pool)
        .await
        .expect("insert server args");
        sqlx::query("DROP TABLE server_args")
            .execute(&db.pool)
            .await
            .expect("drop server_args table");

        let startup_config = load_pool_base_config_with_params(&db, &StartupMode::Default, None)
            .await
            .expect("startup pool load should continue without the broken stdio server");
        assert!(!startup_config.mcp_servers.contains_key("server-stdio-args"));
    }

    #[tokio::test]
    async fn startup_pool_base_config_omits_stdio_server_without_command() {
        let (_temp_dir, db) = create_test_database().await;
        insert_server(&db.pool, "server-stdio-no-command", "Stdio Missing Command", true).await;
        sqlx::query("UPDATE server_config SET command = NULL WHERE id = 'server-stdio-no-command'")
            .execute(&db.pool)
            .await
            .expect("remove stdio command");

        let startup_config = load_pool_base_config_with_params(&db, &StartupMode::Default, None)
            .await
            .expect("startup pool load should continue without the broken stdio server");

        assert!(!startup_config.mcp_servers.contains_key("server-stdio-no-command"));
    }

    #[tokio::test]
    async fn startup_pool_base_config_omits_remote_server_without_url() {
        let (_temp_dir, db) = create_test_database().await;
        insert_http_server(&db.pool, "server-remote-no-url", "Remote Missing URL", true).await;
        sqlx::query("UPDATE server_config SET url = NULL WHERE id = 'server-remote-no-url'")
            .execute(&db.pool)
            .await
            .expect("remove remote URL");

        let startup_config = load_pool_base_config_with_params(&db, &StartupMode::Default, None)
            .await
            .expect("startup pool load should continue without the broken remote server");

        assert!(!startup_config.mcp_servers.contains_key("server-remote-no-url"));
    }

    #[tokio::test]
    async fn manual_header_read_error_is_strict_outside_startup_and_omitted_at_startup() {
        let (_temp_dir, db) = create_test_database().await;
        insert_server(&db.pool, "server-header-read", "Header Read Server", true).await;
        sqlx::query("DROP TABLE server_headers")
            .execute(&db.pool)
            .await
            .expect("drop server_headers table");

        let strict_error = load_pool_base_config(&db, None)
            .await
            .expect_err("strict pool load should fail when manual headers cannot be read");
        assert!(
            strict_error.to_string().contains("Failed to get server headers"),
            "unexpected error: {strict_error}"
        );

        let startup_config = load_pool_base_config_with_params(&db, &StartupMode::Default, None)
            .await
            .expect("startup pool load should continue without the unsafe server");
        assert!(!startup_config.mcp_servers.contains_key("server-header-read"));
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

    const OAUTH_SERVER_ID: &str = "server-oauth";
    const OAUTH_SERVER_NAME: &str = "OAuth Server";

    enum OAuthTokenEndpointResponse {
        RefreshSuccess,
        InvalidGrant,
    }

    struct ExpiredOAuthFixture {
        _temp_dir: TempDir,
        _mock_server: MockServer,
        db: Database,
        secret_store: Arc<LocalSecretStore>,
    }

    async fn mount_oauth_token_endpoint(
        mock_server: &MockServer,
        response: OAuthTokenEndpointResponse,
    ) {
        let template = match response {
            OAuthTokenEndpointResponse::RefreshSuccess => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "access-new",
                "token_type": "bearer",
                "expires_in": 3600
            })),
            OAuthTokenEndpointResponse::InvalidGrant => ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "invalid_grant"
            })),
        };

        let mut mock = Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=refresh-123"));

        if matches!(response, OAuthTokenEndpointResponse::RefreshSuccess) {
            mock = mock.and(body_string_contains("resource=https%3A%2F%2Fexample.com%2Fmcp"));
        }

        mock.respond_with(template).mount(mock_server).await;
    }

    async fn setup_expired_oauth_server(
        token_response: OAuthTokenEndpointResponse,
        manual_headers: Option<HashMap<String, String>>,
    ) -> ExpiredOAuthFixture {
        let (temp_dir, db) = create_test_database().await;
        let secret_store = Arc::new(
            LocalSecretStore::initialize_with_development_root_key(
                db.pool.clone(),
                temp_dir.path().join("secrets").join("local-root.key"),
            )
            .await
            .expect("initialize secret store"),
        );
        insert_http_server(&db.pool, OAUTH_SERVER_ID, OAUTH_SERVER_NAME, true).await;
        let mock_server = MockServer::start().await;
        mount_oauth_token_endpoint(&mock_server, token_response).await;

        upsert_server_oauth_config(
            &db.pool,
            &ServerOAuthConfig {
                id: None,
                server_id: OAUTH_SERVER_ID.to_string(),
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

        if let Some(headers) = manual_headers {
            crate::config::server::upsert_server_headers(&db.pool, OAUTH_SERVER_ID, &headers)
                .await
                .expect("save manual headers");
        }

        let access_token = secret_store
            .create_secret(SecretCreateInput {
                alias: format!("oauth/{OAUTH_SERVER_ID}/access-token"),
                kind: SecretKindInput::OAuthAccessToken,
                value: "access-old".to_string(),
                label: Some(format!("OAuth access token for {OAUTH_SERVER_NAME}")),
                origin: Some(oauth_secret_origin(OAUTH_SERVER_ID, OAUTH_SERVER_NAME, "access-token")),
            })
            .await
            .expect("store access token")
            .placeholder;
        let refresh_token = secret_store
            .create_secret(SecretCreateInput {
                alias: format!("oauth/{OAUTH_SERVER_ID}/refresh-token"),
                kind: SecretKindInput::OAuthRefreshToken,
                value: "refresh-123".to_string(),
                label: Some(format!("OAuth refresh token for {OAUTH_SERVER_NAME}")),
                origin: Some(oauth_secret_origin(OAUTH_SERVER_ID, OAUTH_SERVER_NAME, "refresh-token")),
            })
            .await
            .expect("store refresh token")
            .placeholder;

        upsert_server_oauth_token(
            &db.pool,
            &ServerOAuthToken {
                id: None,
                server_id: OAUTH_SERVER_ID.to_string(),
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

        ExpiredOAuthFixture {
            _temp_dir: temp_dir,
            _mock_server: mock_server,
            db,
            secret_store,
        }
    }

    async fn load_startup_default_pool_config(fixture: &ExpiredOAuthFixture) -> Config {
        load_pool_base_config_with_params(
            &fixture.db,
            &StartupMode::Default,
            Some(Arc::clone(&fixture.secret_store)),
        )
        .await
        .expect("startup config should load")
    }

    #[tokio::test]
    async fn load_pool_base_config_refreshes_expired_oauth_headers() {
        let fixture = setup_expired_oauth_server(OAuthTokenEndpointResponse::RefreshSuccess, None).await;

        let config = load_pool_base_config(&fixture.db, Some(fixture.secret_store))
            .await
            .expect("load pool config");
        let headers = config
            .mcp_servers
            .get(OAUTH_SERVER_ID)
            .and_then(|server| server.headers.as_ref())
            .expect("headers");

        assert_eq!(
            headers.get("authorization").map(String::as_str),
            Some("Bearer access-new")
        );
    }

    #[tokio::test]
    async fn load_pool_base_config_fails_when_oauth_refresh_fails() {
        let fixture = setup_expired_oauth_server(OAuthTokenEndpointResponse::InvalidGrant, None).await;

        let error = load_pool_base_config(&fixture.db, Some(fixture.secret_store))
            .await
            .expect_err("strict pool reload should fail");

        assert!(
            error.to_string().contains("Failed to get effective server headers"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn startup_pool_base_config_continues_when_oauth_refresh_fails() {
        let fixture = setup_expired_oauth_server(OAuthTokenEndpointResponse::InvalidGrant, None).await;

        let config = load_startup_default_pool_config(&fixture).await;

        assert!(!config.mcp_servers.contains_key(OAUTH_SERVER_ID));
    }

    #[tokio::test]
    async fn startup_pool_base_config_preserves_manual_authorization_when_oauth_refresh_fails() {
        let manual_headers = HashMap::from([("Authorization".to_string(), "Bearer manual-token".to_string())]);
        let fixture = setup_expired_oauth_server(OAuthTokenEndpointResponse::InvalidGrant, Some(manual_headers)).await;

        let config = load_startup_default_pool_config(&fixture).await;
        let headers = config
            .mcp_servers
            .get(OAUTH_SERVER_ID)
            .and_then(|server| server.headers.as_ref())
            .expect("manual authorization headers should be preserved");

        let authorization = headers
            .iter()
            .find_map(|(key, value)| key.eq_ignore_ascii_case("authorization").then_some(value.as_str()));

        assert_eq!(authorization, Some("Bearer manual-token"));
    }

    #[tokio::test]
    async fn startup_pool_base_config_continues_when_one_oauth_server_refresh_fails() {
        let fixture = setup_expired_oauth_server(OAuthTokenEndpointResponse::InvalidGrant, None).await;
        insert_server(&fixture.db.pool, "server-stdio", "Stdio Server", true).await;

        let config = load_startup_default_pool_config(&fixture).await;

        let stdio_server = config
            .mcp_servers
            .get("server-stdio")
            .expect("stdio server remains configured");

        assert!(!config.mcp_servers.contains_key(OAUTH_SERVER_ID));
        assert_eq!(stdio_server.command.as_deref(), Some("demo-command"));
    }

    #[tokio::test]
    async fn startup_oauth_refresh_failure_emits_diagnostic_reason_code() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

        impl Write for CaptureWriter {
            fn write(
                &mut self,
                buf: &[u8],
            ) -> std::io::Result<usize> {
                self.0.lock().expect("capture lock").extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_writer({
                let buffer = Arc::clone(&buffer);
                move || CaptureWriter(Arc::clone(&buffer))
            })
            .with_ansi(false)
            .without_time()
            .with_level(false)
            .with_target(false)
            .finish();

        let fixture = setup_expired_oauth_server(OAuthTokenEndpointResponse::InvalidGrant, None).await;
        {
            let _guard = tracing::subscriber::set_default(subscriber);
            load_startup_default_pool_config(&fixture).await;
        }

        let output = String::from_utf8(buffer.lock().expect("capture lock").clone()).expect("utf8 logs");
        assert!(
            output.contains("oauth_header_injection_failed"),
            "expected oauth degrade reason_code in logs: {output}"
        );
        assert!(
            output.contains("startup_loader"),
            "expected startup_loader component in logs: {output}"
        );
        assert!(
            output.contains("startup_continues=true") || output.contains("startup_continues: true"),
            "expected startup_continues marker in logs: {output}"
        );
    }
}
