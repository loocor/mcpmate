use std::sync::Arc;

use axum::{
    Json,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use tokio_stream::wrappers::BroadcastStream;

use crate::{
    api::{
        handlers::ApiError,
        models::audit::{
            AuditListData, AuditListReq, AuditListResp, AuditPolicyData, AuditPolicyResp, AuditPolicySetReq,
        },
        routes::AppState,
    },
    audit::{AuditRetentionPolicySetting, AuditStore},
};

pub async fn list_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AuditListReq>,
) -> Result<Json<AuditListResp>, ApiError> {
    let audit_service = state
        .audit_service
        .clone()
        .ok_or_else(|| ApiError::InternalError("Audit service is unavailable".to_string()))?;

    let filter = query.clone().into_filter();
    let page = audit_service
        .list(&filter, query.cursor.as_deref(), query.limit)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(AuditListResp::success(AuditListData {
        events: page.events,
        next_cursor: page.next_cursor,
    })))
}

pub async fn get_policy(State(state): State<Arc<AppState>>) -> Result<Json<AuditPolicyResp>, ApiError> {
    let audit_database = state
        .audit_database
        .clone()
        .ok_or_else(|| ApiError::InternalError("Audit database is unavailable".to_string()))?;

    let store = AuditStore::from_database(audit_database.as_ref());
    let setting = store.get_policy().await.map_err(ApiError::from)?;

    Ok(Json(AuditPolicyResp::success(AuditPolicyData::from(setting))))
}

pub async fn set_policy(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AuditPolicySetReq>,
) -> Result<Json<AuditPolicyResp>, ApiError> {
    let audit_database = state
        .audit_database
        .clone()
        .ok_or_else(|| ApiError::InternalError("Audit database is unavailable".to_string()))?;

    let store = AuditStore::from_database(audit_database.as_ref());
    let setting = AuditRetentionPolicySetting::from(req);
    store.set_policy(&setting).await.map_err(ApiError::from)?;

    Ok(Json(AuditPolicyResp::success(AuditPolicyData::from(setting))))
}

pub async fn audit_events_ws(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    if let Some(service) = state.audit_service.clone() {
        let receiver = service.subscribe();
        ws.on_upgrade(move |socket| handle_audit_ws(socket, receiver))
    } else {
        ws.on_upgrade(move |mut socket| async move {
            let _ = socket.close().await;
        })
    }
}

async fn handle_audit_ws(
    socket: WebSocket,
    receiver: tokio::sync::broadcast::Receiver<crate::audit::AuditEventDto>,
) {
    let (mut sender, _receiver) = socket.split();
    let mut stream = BroadcastStream::new(receiver);

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => match serde_json::to_string(&event) {
                Ok(json) => {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "Failed to serialize audit event for websocket");
                }
            },
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_)) => continue,
        }
    }

    let _ = sender.close().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::routes::AppState,
        audit::{AuditAction, AuditEvent, AuditService, AuditStatus, AuditStore},
        clients::ClientConfigService,
        config::audit_database::AuditDatabase,
        core::{
            cache::{RedbCacheManager, manager::CacheConfig},
            models::Config,
            pool::UpstreamConnectionPool,
        },
        inspector::{calls::InspectorCallRegistry, sessions::InspectorSessionManager},
        system::metrics::MetricsCollector,
    };
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
    use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};
    use tempfile::tempdir;
    use tokio::sync::Mutex;

    async fn test_state() -> Arc<AppState> {
        let temp_dir = tempdir().expect("temp dir");
        let audit_path = temp_dir.path().join("audit.db");
        let audit_url = format!("sqlite:{}", audit_path.display());
        let options = SqliteConnectOptions::from_str(&audit_url)
            .expect("options")
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_millis(5_000))
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true);
        let audit_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("connect audit db");

        let audit_database = Arc::new(AuditDatabase {
            pool: audit_pool.clone(),
            path: PathBuf::from(&audit_path),
        });
        let audit_store = Arc::new(AuditStore::new(audit_pool));
        audit_store.initialize().await.expect("initialize audit store");
        let audit_service = Arc::new(AuditService::new(audit_store).await.expect("audit service"));

        let cache_path = temp_dir.path().join("capability.redb");
        let redb_cache = Arc::new(RedbCacheManager::new(cache_path, CacheConfig::default()).expect("cache manager"));

        Arc::new(AppState {
            connection_pool: Arc::new(Mutex::new(UpstreamConnectionPool::new(
                Arc::new(Config::default()),
                None,
            ))),
            metrics_collector: Arc::new(MetricsCollector::new(Duration::from_secs(5))),
            http_proxy: None,
            profile_merge_service: None,
            database: None,
            audit_database: Some(audit_database),
            audit_service: Some(audit_service),
            config_application_state: Arc::new(crate::core::profile::ConfigApplicationStateManager::new()),
            redb_cache,
            unified_query: None,
            client_service: None::<Arc<ClientConfigService>>,
            inspector_calls: Arc::new(InspectorCallRegistry::new()),
            inspector_sessions: Arc::new(InspectorSessionManager::new()),
        })
    }

    #[tokio::test]
    async fn list_events_returns_persisted_audit_rows() {
        let state = test_state().await;
        state
            .audit_service
            .as_ref()
            .expect("audit service")
            .emit(
                AuditEvent::new(AuditAction::ServerCreate, AuditStatus::Success)
                    .with_server_id("server-a")
                    .with_http_route("POST", "/api/mcp/servers/create")
                    .build(),
            )
            .await;

        tokio::time::sleep(Duration::from_millis(25)).await;

        let response = list_events(
            State(state),
            Query(AuditListReq {
                limit: Some(10),
                ..AuditListReq::default()
            }),
        )
        .await
        .expect("list events response");

        assert!(response.0.success);
        let data = response.0.data.expect("audit list data");
        assert_eq!(data.events.len(), 1);
        assert_eq!(data.events[0].action, AuditAction::ServerCreate);
    }

    #[tokio::test]
    async fn list_events_applies_filters_and_cursor() {
        let state = test_state().await;
        let audit_service = state.audit_service.as_ref().expect("audit service");

        audit_service
            .emit(
                AuditEvent::new(AuditAction::ToolsCall, AuditStatus::Success)
                    .with_client_id("client-a")
                    .with_server_id("server-a")
                    .occurred_at_ms(1_000)
                    .build(),
            )
            .await;
        audit_service
            .emit(
                AuditEvent::new(AuditAction::ToolsCall, AuditStatus::Failed)
                    .with_client_id("client-a")
                    .with_server_id("server-a")
                    .occurred_at_ms(2_000)
                    .build(),
            )
            .await;
        audit_service
            .emit(
                AuditEvent::new(AuditAction::ServerEnable, AuditStatus::Success)
                    .with_server_id("server-a")
                    .occurred_at_ms(3_000)
                    .build(),
            )
            .await;

        tokio::time::sleep(Duration::from_millis(25)).await;

        let first = list_events(
            State(state.clone()),
            Query(AuditListReq {
                limit: Some(1),
                category: Some(crate::audit::AuditCategory::McpRequest),
                client_id: Some("client-a".to_string()),
                ..AuditListReq::default()
            }),
        )
        .await
        .expect("first filtered page");

        let first_data = first.0.data.expect("first page data");
        assert_eq!(first_data.events.len(), 1);
        assert_eq!(first_data.events[0].status, AuditStatus::Failed);
        assert!(first_data.next_cursor.is_some());

        let second = list_events(
            State(state),
            Query(AuditListReq {
                limit: Some(1),
                category: Some(crate::audit::AuditCategory::McpRequest),
                client_id: Some("client-a".to_string()),
                cursor: first_data.next_cursor,
                ..AuditListReq::default()
            }),
        )
        .await
        .expect("second filtered page");

        let second_data = second.0.data.expect("second page data");
        assert_eq!(second_data.events.len(), 1);
        assert_eq!(second_data.events[0].status, AuditStatus::Success);
        assert!(second_data.next_cursor.is_none());
    }
}
