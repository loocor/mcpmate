use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use axum::{Router, body::Body, body::to_bytes};
use hyper::{Request, StatusCode};
use mcpmate::api::routes::{self, AppState};
use mcpmate::clients::ClientConfigService;
use mcpmate::common::profile::ProfileType;
use mcpmate::config::database::Database;
use mcpmate::config::models::Profile;
use mcpmate::config::profile::{init::initialize_profile_tables, upsert_profile};
use mcpmate::core::cache::{RedbCacheManager, manager::CacheConfig};
use mcpmate::core::models::Config;
use mcpmate::core::pool::UpstreamConnectionPool;
use mcpmate::core::profile::ConfigApplicationStateManager;
use mcpmate::inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager};
use mcpmate::system::metrics::MetricsCollector;
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tempfile::TempDir;
use tokio::sync::Mutex;
use tower::ServiceExt;

async fn setup_context() -> Result<(TempDir, sqlx::Pool<sqlx::Sqlite>, Arc<AppState>, String)> {
    let temp_dir = TempDir::new().context("create temp dir")?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .context("create in-memory sqlite pool")?;

    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await?;
    sqlx::query(
        r#"
        CREATE TABLE server_config (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;
    initialize_profile_tables(&pool).await?;

    let mut profile = Profile::new("customer support".to_string(), ProfileType::Scenario);
    profile.is_active = true;
    let profile_id = upsert_profile(&pool, &profile).await?;

    let database = Arc::new(Database {
        pool: pool.clone(),
        path: temp_dir.path().join("mcpmate-test.db"),
    });
    let state = build_app_state(temp_dir.path().join("profile-guidance.redb"), Some(database));

    Ok((temp_dir, pool, Arc::new(state), profile_id))
}

fn build_app_state(
    cache_path: PathBuf,
    database: Option<Arc<Database>>,
) -> AppState {
    AppState {
        connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
            Arc::new(Config::default()),
            database.clone(),
        ))),
        metrics_collector: Arc::new(MetricsCollector::new(Duration::from_secs(1))),
        http_proxy: None,
        profile_merge_service: None,
        database,
        audit_database: None,
        audit_service: None,
        config_application_state: Arc::new(ConfigApplicationStateManager::new()),
        redb_cache: Arc::new(RedbCacheManager::new(cache_path, CacheConfig::default()).expect("redb cache")),
        unified_query: None,
        client_service: None::<Arc<ClientConfigService>>,
        inspector_calls: Arc::new(InspectorCallRegistry::new()),
        inspector_sessions: Arc::new(InspectorSessionManager::new()),
        oauth_manager: None,
    }
}

async fn read_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
    serde_json::from_slice(&bytes).expect("json response")
}

#[tokio::test]
async fn profile_guidance_routes_upsert_and_list_records() -> Result<()> {
    let (_temp_dir, _pool, state, profile_id) = setup_context().await?;
    let app = Router::new().merge(routes::profile::routes(state));

    let upsert_response = app
        .clone()
        .oneshot(
            Request::post("/mcp/profile/guidance/upsert")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "profile_id": profile_id,
                        "slug": "triage",
                        "title": "Triage customer tickets",
                        "summary": "Use CRM and ticket tools in order.",
                        "scenario": "Customer support triage",
                        "activation": "Use when a customer support ticket needs context gathering.",
                        "capability_refs": [{
                            "kind": "tool",
                            "id": "ticket_lookup",
                            "name": "Ticket lookup",
                            "server_name": "support"
                        }],
                        "validation_notes": "Confirm the ticket id before calling tools.",
                        "avoid": "Do not contact the customer before reviewing history.",
                        "content_markdown": "## Workflow\nLoad customer context first.",
                        "source_uri": "https://example.com/skills/triage/SKILL.md",
                        "enabled": true
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("route response");

    let upsert_status = upsert_response.status();
    let upsert_body = read_json(upsert_response).await;
    assert_eq!(upsert_status, StatusCode::OK, "{upsert_body}");
    assert_eq!(upsert_body["data"]["guidance"]["slug"], "triage");
    assert_eq!(upsert_body["data"]["guidance"]["scenario"], "Customer support triage");
    assert_eq!(
        upsert_body["data"]["guidance"]["capability_refs"][0]["id"],
        "ticket_lookup"
    );

    let list_response = app
        .clone()
        .oneshot(
            Request::get(format!("/mcp/profile/guidance/list?profile_id={profile_id}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("route response");

    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = read_json(list_response).await;
    assert_eq!(list_body["data"]["guidance"][0]["title"], "Triage customer tickets");

    let delete_response = app
        .oneshot(
            Request::delete("/mcp/profile/guidance/delete")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "profile_id": profile_id,
                        "slug": "triage"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("route response");

    let delete_status = delete_response.status();
    let delete_body = read_json(delete_response).await;
    assert_eq!(delete_status, StatusCode::OK, "{delete_body}");
    assert_eq!(delete_body["data"]["success_count"], 1);

    Ok(())
}
