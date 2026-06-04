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
                LocalSecretStore, SecretCreateInput, SecretKindInput, SecretUsageLocationInput, SecretUsageUpsertInput,
            },
            sync_server_secret_usages,
        },
    },
    inspector::{calls::InspectorCallRegistry, service as inspector_service, sessions::InspectorSessionManager},
    system::metrics::MetricsCollector,
};
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::TempDir;
use tokio::sync::Mutex;
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
        secret_store: Some(secret_store.clone()),
    });

    (temp_dir, state, secret_store)
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
