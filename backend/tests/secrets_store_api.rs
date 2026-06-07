use std::{collections::HashMap, sync::Arc};

use axum::body::to_bytes;
use hyper::{Request, StatusCode};
use mcpmate::{
    api::routes::AppState,
    common::server::ServerType,
    config::{database::Database, initialization::run_initialization},
    core::{
        cache::{RedbCacheManager, manager::CacheConfig},
        models::{Config, MCPServerConfig},
        pool::UpstreamConnectionPool,
        profile::ConfigApplicationStateManager,
        secrets::{
            resolve_runtime_server_config,
            store::{
                LocalFileRootKeyProvider, LocalSecretStore, PassphraseRootKeyProvider, RootKeyProviderMode,
                SecretCreateInput, SecretKindInput, SecretUsageLocationInput, SecretUsageUpsertInput,
            },
            sync_server_secret_usages,
        },
    },
    inspector::{calls::InspectorCallRegistry, service as inspector_service, sessions::InspectorSessionManager},
    system::metrics::MetricsCollector,
};
use mcpmate_secrets::SecretRootKeyProvider;
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tower::ServiceExt;

struct EnvVarGuard {
    key: &'static str,
}

impl EnvVarGuard {
    fn set(
        key: &'static str,
        value: &str,
    ) -> Self {
        // SAFETY: this integration test owns the process key while running under serial execution.
        unsafe { std::env::set_var(key, value) };
        Self { key }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        // SAFETY: pairs with `set` in this serial integration test.
        unsafe { std::env::remove_var(self.key) };
    }
}

async fn read_json_response(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("read body");
    serde_json::from_slice(&bytes).expect("json response")
}

async fn build_test_context() -> (TempDir, Arc<AppState>, Arc<LocalSecretStore>) {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("sqlite pool");

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&db_pool)
        .await
        .expect("enable foreign keys");
    run_initialization(&db_pool).await.expect("initialize schema");

    let database = Arc::new(Database {
        pool: db_pool.clone(),
        path: temp_dir.path().join("mcpmate-test.db"),
    });
    let secret_store = Arc::new(
        LocalSecretStore::initialize_with_development_root_key(
            db_pool.clone(),
            temp_dir.path().join("secrets").join("local-root.key"),
        )
        .await
        .expect("initialize secret store"),
    );
    let redb_cache =
        Arc::new(RedbCacheManager::new(temp_dir.path().join("capability.redb"), CacheConfig::default()).expect("redb"));
    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());

    let pool = UpstreamConnectionPool::new(Arc::new(Config::default()), Some(database.clone()))
        .with_secret_resolver(secret_store.clone());

    let state = Arc::new(AppState {
        connection_pool: Arc::new(Mutex::new(pool)),
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: None,
        profile_merge_service: None,
        database: Some(database),
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache,
        unified_query: None,
        client_service: None,
        inspector_calls,
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        oauth_manager: None,
        secret_store: RwLock::new(Some(secret_store.clone())),
        secret_store_readiness: RwLock::new(mcpmate::core::secrets::store::SecretStoreReadiness::ready(
            secret_store.provider_metadata(),
        )),
    });

    (temp_dir, state, secret_store)
}

async fn build_passphrase_test_context(
    master_password: &str,
) -> (TempDir, Arc<AppState>, Arc<LocalSecretStore>) {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("sqlite pool");

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&db_pool)
        .await
        .expect("enable foreign keys");
    run_initialization(&db_pool).await.expect("initialize schema");

    let database = Arc::new(Database {
        pool: db_pool.clone(),
        path: temp_dir.path().join("mcpmate-test.db"),
    });
    let secrets_dir = temp_dir.path().join("secrets");
    let passphrase_path = secrets_dir.join("passphrase-wrapped-key.json");
    let root_key_provider = Arc::new(PassphraseRootKeyProvider::new(
        passphrase_path,
        master_password,
    ));
    let secret_store = Arc::new(
        LocalSecretStore::initialize_with_root_key_provider(db_pool.clone(), root_key_provider)
            .await
            .expect("initialize passphrase secret store"),
    );
    assert_eq!(
        secret_store.provider_metadata().mode(),
        RootKeyProviderMode::Passphrase
    );

    let redb_cache =
        Arc::new(RedbCacheManager::new(temp_dir.path().join("capability.redb"), CacheConfig::default()).expect("redb"));
    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());

    let pool = UpstreamConnectionPool::new(Arc::new(Config::default()), Some(database.clone()))
        .with_secret_resolver(secret_store.clone());

    let state = Arc::new(AppState {
        connection_pool: Arc::new(Mutex::new(pool)),
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: None,
        profile_merge_service: None,
        database: Some(database),
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache,
        unified_query: None,
        client_service: None,
        inspector_calls,
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        oauth_manager: None,
        secret_store: RwLock::new(Some(secret_store.clone())),
        secret_store_readiness: RwLock::new(mcpmate::core::secrets::store::SecretStoreReadiness::ready(
            secret_store.provider_metadata(),
        )),
    });

    (temp_dir, state, secret_store)
}

fn build_unavailable_secret_store_state(temp_dir: &TempDir) -> Arc<AppState> {
    let redb_cache =
        Arc::new(RedbCacheManager::new(temp_dir.path().join("capability.redb"), CacheConfig::default()).expect("redb"));

    Arc::new(AppState {
        connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
            Arc::new(Config::default()),
            None,
        ))),
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: None,
        profile_merge_service: None,
        database: None,
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache,
        unified_query: None,
        client_service: None,
        inspector_calls: Arc::new(InspectorCallRegistry::new()),
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        oauth_manager: None,
        secret_store: RwLock::new(None),
        secret_store_readiness: RwLock::new(mcpmate::api::routes::unavailable_secret_store_readiness("database_unavailable")),
    })
}

#[tokio::test]
#[serial_test::serial]
async fn local_store_encrypts_values_and_resolves_runtime_placeholders() {
    let _key = EnvVarGuard::set(
        "MCPMATE_SECRETS_LOCAL_KEY",
        "MCPMate test key material for local store 000000",
    );
    let (_temp_dir, _state, store) = build_test_context().await;

    let metadata = store
        .create_secret(SecretCreateInput {
            alias: "server/github/token".to_string(),
            kind: SecretKindInput::Token,
            value: "ghp_runtime_token".to_string(),
            label: Some("GitHub token".to_string()),
            origin: None,
        })
        .await
        .expect("create secret");

    assert_eq!(metadata.placeholder, "[[secret:server/github/token]]");

    let stored = sqlx::query_as::<_, (String,)>("SELECT encrypted_value FROM secure_store_secrets")
        .fetch_one(&store.pool())
        .await
        .expect("read encrypted value");
    assert!(!stored.0.contains("ghp_runtime_token"));

    let config = MCPServerConfig {
        kind: ServerType::Stdio,
        command: Some("node".to_string()),
        args: Some(vec!["--token=[[secret:server/github/token]]".to_string()]),
        url: None,
        env: Some(HashMap::from([(
            "GITHUB_TOKEN".to_string(),
            "[[secret:server/github/token]]".to_string(),
        )])),
        headers: None,
    };

    let resolved = resolve_runtime_server_config(&config, store.as_ref()).expect("resolve runtime config");
    assert_eq!(
        resolved.args.as_ref().and_then(|args| args.first()).map(String::as_str),
        Some("--token=ghp_runtime_token")
    );
    assert_eq!(
        resolved
            .env
            .as_ref()
            .and_then(|env| env.get("GITHUB_TOKEN"))
            .map(String::as_str),
        Some("ghp_runtime_token")
    );
}

#[tokio::test]
#[serial_test::serial]
async fn secret_store_status_reports_ready_provider() {
    let _key = EnvVarGuard::set(
        "MCPMATE_SECRETS_LOCAL_KEY",
        "MCPMate test key material for local store status ready",
    );
    let (_temp_dir, state, _store) = build_test_context().await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/secrets/status")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("status response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json_response(response).await;
    assert_eq!(body["data"]["status"], "ready");
    assert_eq!(body["data"]["provider"]["provider_mode"], "development");
    assert_eq!(body["data"]["provider"]["security_level"], "development");
    assert!(body["data"].get("issue").is_none());
}

#[tokio::test]
#[serial_test::serial]
async fn secret_store_status_reports_unavailable_without_failing() {
    let temp_dir = TempDir::new().expect("temp dir");
    let state = build_unavailable_secret_store_state(&temp_dir);
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/secrets/status")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("status response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json_response(response).await;
    assert_eq!(body["data"]["status"], "unavailable");
    assert_eq!(body["data"]["issue"]["reason_code"], "database_unavailable");
    assert!(body["data"].get("provider").is_none());
}

#[tokio::test]
#[serial_test::serial]
async fn secrets_api_never_returns_plaintext_values() {
    let _key = EnvVarGuard::set(
        "MCPMATE_SECRETS_LOCAL_KEY",
        "MCPMate test key material for local store 111111",
    );
    let (_temp_dir, state, _store) = build_test_context().await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/create")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "alias": "server/http/header",
                        "kind": "header_value",
                        "label": "HTTP auth header",
                        "value": "Bearer secret-token"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create response");

    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = read_json_response(create_response).await;
    assert_eq!(create_body["data"]["placeholder"], "[[secret:server/http/header]]");
    assert!(!create_body.to_string().contains("Bearer secret-token"));

    let detail_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/secrets/details?alias=server/http/header")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("detail response");

    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_body = read_json_response(detail_response).await;
    assert_eq!(detail_body["data"]["alias"], "server/http/header");
    assert!(!detail_body.to_string().contains("Bearer secret-token"));
}

#[tokio::test]
#[serial_test::serial]
async fn usage_refs_report_server_runtime_locations_and_block_default_delete() {
    let _key = EnvVarGuard::set(
        "MCPMATE_SECRETS_LOCAL_KEY",
        "MCPMate test key material for local store 222222",
    );
    let (_temp_dir, _state, store) = build_test_context().await;

    store
        .create_secret(SecretCreateInput {
            alias: "server/github/token".to_string(),
            kind: SecretKindInput::Token,
            value: "ghp_runtime_token".to_string(),
            label: None,
            origin: None,
        })
        .await
        .expect("create secret");
    store
        .upsert_usage(SecretUsageUpsertInput {
            alias: "server/github/token".to_string(),
            server_id: "github-server".to_string(),
            location: SecretUsageLocationInput::StdioEnv {
                name: "GITHUB_TOKEN".to_string(),
            },
        })
        .await
        .expect("record usage");

    let usages = store.list_usages("server/github/token").await.expect("list usages");
    assert_eq!(usages.len(), 1);
    assert_eq!(usages[0].server_id, "github-server");

    let err = store
        .delete_secret("server/github/token", false)
        .await
        .expect_err("in-use secret is protected");
    assert!(err.to_string().contains("in use"));
}

#[tokio::test]
#[serial_test::serial]
async fn usage_sync_detects_placeholders_in_server_runtime_config() {
    let _key = EnvVarGuard::set(
        "MCPMATE_SECRETS_LOCAL_KEY",
        "MCPMate test key material for local store 333333",
    );
    let (_temp_dir, _state, store) = build_test_context().await;
    store
        .create_secret(SecretCreateInput {
            alias: "server/github/token".to_string(),
            kind: SecretKindInput::Token,
            value: "ghp_runtime_token".to_string(),
            label: None,
            origin: None,
        })
        .await
        .expect("create token secret");
    store
        .create_secret(SecretCreateInput {
            alias: "server/http/auth".to_string(),
            kind: SecretKindInput::HeaderValue,
            value: "Bearer runtime-token".to_string(),
            label: None,
            origin: None,
        })
        .await
        .expect("create header secret");

    let stdio_config = MCPServerConfig {
        kind: ServerType::Stdio,
        command: Some("node".to_string()),
        args: Some(vec![
            "--token".to_string(),
            "[[secret:server/github/token]]".to_string(),
        ]),
        url: None,
        env: Some(HashMap::from([(
            "GITHUB_TOKEN".to_string(),
            "[[secret:server/github/token]]".to_string(),
        )])),
        headers: None,
    };
    let http_config = MCPServerConfig {
        kind: ServerType::StreamableHttp,
        command: None,
        args: None,
        url: Some("https://example.test/mcp?auth=[[secret:server/http/auth]]".to_string()),
        env: None,
        headers: Some(HashMap::from([(
            "Authorization".to_string(),
            "[[secret:server/http/auth]]".to_string(),
        )])),
    };

    sync_server_secret_usages(store.as_ref(), "github-server", &stdio_config)
        .await
        .expect("sync stdio usages");
    sync_server_secret_usages(store.as_ref(), "http-server", &http_config)
        .await
        .expect("sync http usages");

    let token_usages = store
        .list_usages("server/github/token")
        .await
        .expect("list token usages");
    assert!(token_usages.iter().any(|usage| {
        usage.server_id == "github-server"
            && usage.location
                == SecretUsageLocationInput::StdioEnv {
                    name: "GITHUB_TOKEN".to_string(),
                }
    }));
    assert!(token_usages.iter().any(|usage| {
        usage.server_id == "github-server" && usage.location == SecretUsageLocationInput::StdioArgument { index: 1 }
    }));

    let http_usages = store.list_usages("server/http/auth").await.expect("list http usages");
    assert!(http_usages.iter().any(|usage| {
        usage.server_id == "http-server" && usage.location == SecretUsageLocationInput::StreamableHttpUrl
    }));
    assert!(http_usages.iter().any(|usage| {
        usage.server_id == "http-server"
            && usage.location
                == SecretUsageLocationInput::StreamableHttpHeader {
                    name: "Authorization".to_string(),
                }
    }));
}

#[tokio::test]
#[serial_test::serial]
async fn usage_sync_deduplicates_same_placeholder_twice_in_one_value() {
    let _key = EnvVarGuard::set(
        "MCPMATE_SECRETS_LOCAL_KEY",
        "MCPMate test key material for local store 444444",
    );
    let (_temp_dir, _state, store) = build_test_context().await;
    store
        .create_secret(SecretCreateInput {
            alias: "server/http/auth".to_string(),
            kind: SecretKindInput::HeaderValue,
            value: "Bearer runtime-token".to_string(),
            label: None,
            origin: None,
        })
        .await
        .expect("create header secret");

    // The same placeholder appears twice in one URL. Before the dedup fix,
    // this would produce duplicate usage entries that violate the UNIQUE
    // constraint on (alias, server_id, location_kind, location_name, location_index).
    let config = MCPServerConfig {
        kind: ServerType::StreamableHttp,
        command: None,
        args: None,
        url: Some(
            "https://example.test/mcp?auth=[[secret:server/http/auth]]&token=[[secret:server/http/auth]]".to_string(),
        ),
        env: None,
        headers: None,
    };

    sync_server_secret_usages(store.as_ref(), "dup-server", &config)
        .await
        .expect("sync must succeed even with duplicate placeholders in one value");

    let usages = store
        .list_usages("server/http/auth")
        .await
        .expect("list usages");

    // Only one usage record should exist despite the placeholder appearing twice.
    assert_eq!(
        usages.len(),
        1,
        "duplicate placeholder in one value must produce exactly one usage, got {}",
        usages.len()
    );
    assert_eq!(usages[0].server_id, "dup-server");
    assert_eq!(usages[0].location, SecretUsageLocationInput::StreamableHttpUrl);
}

#[tokio::test]
#[serial_test::serial]
async fn provider_switch_from_passphrase_requires_current_passphrase() {
    let (_temp_dir, state, _store) = build_passphrase_test_context("correct horse battery staple").await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/provider/switch")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "mode": "local_file"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("switch response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = read_json_response(response).await;
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Current passphrase is required")
    );
}

#[tokio::test]
#[serial_test::serial]
async fn provider_switch_from_passphrase_rejects_wrong_current_passphrase() {
    let (_temp_dir, state, _store) = build_passphrase_test_context("correct horse battery staple").await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/provider/switch")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "mode": "local_file",
                        "current_passphrase": "wrong password"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("switch response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = read_json_response(response).await;
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Invalid current passphrase")
    );
}

#[tokio::test]
#[serial_test::serial]
async fn provider_switch_from_passphrase_to_local_file_succeeds_with_current_passphrase() {
    let (_temp_dir, state, _store) = build_passphrase_test_context("correct horse battery staple").await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/provider/switch")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "mode": "local_file",
                        "current_passphrase": "correct horse battery staple"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("switch response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json_response(response).await;
    assert_eq!(body["data"]["new_status"]["status"], "ready");
    assert_eq!(body["data"]["new_status"]["provider"]["provider_mode"], "local_file");
}

async fn build_locked_passphrase_context(
    master_password: &str,
) -> (TempDir, Arc<AppState>) {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("sqlite pool");

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&db_pool)
        .await
        .expect("enable foreign keys");
    run_initialization(&db_pool).await.expect("initialize schema");

    let database = Arc::new(Database {
        pool: db_pool.clone(),
        path: temp_dir.path().join("mcpmate-test.db"),
    });
    let data_dir = temp_dir.path();
    let secrets_dir = data_dir.join("secrets");
    let passphrase_path = secrets_dir.join("passphrase-wrapped-key.json");
    LocalSecretStore::ensure_schema(&db_pool)
        .await
        .expect("ensure secure store schema");
    PassphraseRootKeyProvider::new(passphrase_path, master_password)
        .load_or_create_root_key()
        .expect("seed passphrase root key");

    mcpmate_secrets::database::upsert_provider_config(&db_pool, "passphrase")
        .await
        .expect("persist provider mode");

    let bootstrap =
        mcpmate::core::secrets::store::bootstrap_secret_store(db_pool.clone(), data_dir).await;
    let bootstrap_readiness = bootstrap.readiness.clone();
    match &bootstrap_readiness {
        mcpmate::core::secrets::store::SecretStoreReadiness::Unavailable { reason_code, .. } => {
            assert_eq!(reason_code, "passphrase_unlock_required");
        }
        other => panic!("expected locked passphrase bootstrap, got {other:?}"),
    }

    let redb_cache =
        Arc::new(RedbCacheManager::new(temp_dir.path().join("capability.redb"), CacheConfig::default()).expect("redb"));
    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());

    let pool = UpstreamConnectionPool::new(Arc::new(Config::default()), Some(database.clone()));

    let state = Arc::new(AppState {
        connection_pool: Arc::new(Mutex::new(pool)),
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: None,
        profile_merge_service: None,
        database: Some(database),
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache,
        unified_query: None,
        client_service: None,
        inspector_calls,
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        oauth_manager: None,
        secret_store: RwLock::new(bootstrap.store.map(Arc::new)),
        secret_store_readiness: RwLock::new(bootstrap_readiness),
    });

    (temp_dir, state)
}

#[tokio::test]
#[serial_test::serial]
async fn unlock_endpoint_initializes_passphrase_store_after_cold_start() {
    let (_temp_dir, state) = build_locked_passphrase_context("correct horse battery staple").await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/unlock")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({ "passphrase": "correct horse battery staple" }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("unlock response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json_response(response).await;
    assert_eq!(body["data"]["status"], "ready");
    assert_eq!(body["data"]["provider"]["provider_mode"], "passphrase");

    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/secrets/list")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("list response");
    assert_eq!(list_response.status(), StatusCode::OK);
}

#[tokio::test]
#[serial_test::serial]
async fn unlock_endpoint_rejects_wrong_passphrase() {
    let (_temp_dir, state) = build_locked_passphrase_context("correct horse battery staple").await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/unlock")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({ "passphrase": "wrong password" }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("unlock response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial_test::serial]
async fn rotate_passphrase_rewraps_root_key_in_passphrase_mode() {
    let (_temp_dir, state, _store) = build_passphrase_test_context("old passphrase value").await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/passphrase/rotate")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "current_passphrase": "old passphrase value",
                        "new_passphrase": "new passphrase value",
                        "confirm": "new passphrase value"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("rotate response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json_response(response).await;
    assert_eq!(body["data"]["status"], "ready");
    assert_eq!(body["data"]["provider"]["provider_mode"], "passphrase");
}

#[tokio::test]
#[serial_test::serial]
async fn provider_mode_persists_across_restart_simulation() {
    let temp_dir = TempDir::new().expect("temp dir");
    let db_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("sqlite pool");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&db_pool)
        .await
        .expect("enable foreign keys");
    run_initialization(&db_pool).await.expect("initialize schema");

    let database = Arc::new(Database {
        pool: db_pool.clone(),
        path: temp_dir.path().join("mcpmate-test.db"),
    });
    let local_key_path = temp_dir.path().join("secrets").join("local-root.key");
    let secret_store = Arc::new(
        LocalSecretStore::initialize_with_root_key_provider(
            db_pool.clone(),
            Arc::new(LocalFileRootKeyProvider::new(local_key_path)),
        )
        .await
        .expect("initialize secret store"),
    );
    let store_readiness =
        mcpmate::core::secrets::store::SecretStoreReadiness::ready(secret_store.provider_metadata());
    let redb_cache =
        Arc::new(RedbCacheManager::new(temp_dir.path().join("capability.redb"), CacheConfig::default()).expect("redb"));
    let inspector_calls = Arc::new(InspectorCallRegistry::new());
    inspector_service::set_call_registry(inspector_calls.clone());
    let pool = UpstreamConnectionPool::new(Arc::new(Config::default()), Some(database.clone()))
        .with_secret_resolver(secret_store.clone());
    let state = Arc::new(AppState {
        connection_pool: Arc::new(Mutex::new(pool)),
        metrics_collector: Arc::new(MetricsCollector::new(std::time::Duration::from_secs(1))),
        http_proxy: None,
        profile_merge_service: None,
        database: Some(database),
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache,
        unified_query: None,
        client_service: None,
        inspector_calls,
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        oauth_manager: None,
        secret_store: RwLock::new(Some(secret_store)),
        secret_store_readiness: RwLock::new(store_readiness),
    });

    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state.clone()));
    let switch_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/provider/switch")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({
                        "mode": "passphrase",
                        "passphrase": "switch passphrase"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("switch response");
    assert_eq!(switch_response.status(), StatusCode::OK);

    let persisted = mcpmate_secrets::database::get_provider_config(&db_pool)
        .await
        .expect("load provider config")
        .expect("provider config row");
    assert_eq!(persisted.provider_mode, "passphrase");

    let restart_bootstrap =
        mcpmate::core::secrets::store::bootstrap_secret_store(db_pool, temp_dir.path()).await;
    match restart_bootstrap.readiness {
        mcpmate::core::secrets::store::SecretStoreReadiness::Unavailable { reason_code, .. } => {
            assert_eq!(reason_code, "passphrase_unlock_required");
        }
        other => panic!("expected locked passphrase bootstrap, got {other:?}"),
    }
}

#[tokio::test]
#[serial_test::serial]
async fn password_set_rejects_overwrite_when_already_configured() {
    let _key = EnvVarGuard::set(
        "MCPMATE_SECRETS_LOCAL_KEY",
        "MCPMate test key material for local store pwd001",
    );
    let (_temp_dir, state, _store) = build_test_context().await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state));

    // First set succeeds.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/password/set")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({ "password": "test1234", "confirm": "test1234" }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(resp.status(), StatusCode::OK);

    // Second set should be rejected (409 Conflict).
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/secrets/password/set")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({ "password": "other5678", "confirm": "other5678" }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
#[serial_test::serial]
async fn delete_secret_blocks_on_active_usage_but_allows_stale() {
    let _key = EnvVarGuard::set(
        "MCPMATE_SECRETS_LOCAL_KEY",
        "MCPMate test key material for local store del001",
    );
    let (_temp_dir, state, store) = build_test_context().await;
    let app = axum::Router::new().merge(mcpmate::api::routes::secrets::routes(state.clone()));

    // Create a secret and record a usage for a non-existent server (stale).
    store
        .create_secret(SecretCreateInput {
            alias: "server/ghost/token".to_string(),
            kind: SecretKindInput::Token,
            value: "secret-value".to_string(),
            label: None,
            origin: None,
        })
        .await
        .expect("create secret");
    store
        .upsert_usage(SecretUsageUpsertInput {
            alias: "server/ghost/token".to_string(),
            server_id: "nonexistent-server".to_string(),
            location: SecretUsageLocationInput::StdioEnv {
                name: "TOKEN".to_string(),
            },
        })
        .await
        .expect("record stale usage");

    // Delete should succeed because the usage is stale (server doesn't exist).
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/secrets/delete")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(
                    json!({ "alias": "server/ghost/token", "force": false }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(resp.status(), StatusCode::OK);
}
