use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rmcp::RoleClient;
use rmcp::service::Peer;
use tokio::sync::RwLock;

use crate::api::models::inspector::InspectorMode;
use crate::core::pool::{ValidationReservationLease, ValidationReservationToken};

const SESSION_TTL: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub(crate) struct InspectorSessionInfo {
    pub session_id: String,
    pub server_id: String,
    pub mode: InspectorMode,
    pub expires_at_epoch_ms: u128,
}

struct SessionEntry {
    server_id: String,
    mode: InspectorMode,
    peer: Option<Peer<RoleClient>>,
    validation_reservation: Option<ValidationReservationToken>,
    expires_at: Instant,
    closing: bool,
}

#[derive(Default, Clone)]
pub struct InspectorSessionManager {
    inner: Arc<RwLock<HashMap<String, SessionEntry>>>,
}

#[derive(Clone)]
pub(crate) struct ActiveSession {
    pub server_id: String,
    pub mode: InspectorMode,
    pub peer: Option<Peer<RoleClient>>,
    pub validation_reservation: Option<ValidationReservationToken>,
}

pub(crate) enum SessionLookup {
    Active(ActiveSession),
    Expired(InspectorSessionClosing),
    Missing,
}

#[derive(Clone)]
pub(crate) struct InspectorSessionCloseInfo {
    pub mode: InspectorMode,
    pub validation_reservation: Option<ValidationReservationToken>,
}

pub(crate) struct InspectorSessionClosing {
    manager: InspectorSessionManager,
    session_id: String,
    info: InspectorSessionCloseInfo,
    armed: bool,
}

impl InspectorSessionClosing {
    pub(crate) fn info(&self) -> &InspectorSessionCloseInfo {
        &self.info
    }

    pub(crate) async fn complete(mut self) -> bool {
        let removed = self
            .manager
            .complete_close(&self.session_id, self.info.validation_reservation.as_ref())
            .await;
        if removed {
            self.armed = false;
        }
        removed
    }
}

impl Drop for InspectorSessionClosing {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let manager = self.manager.clone();
        let session_id = self.session_id.clone();
        let reservation = self.info.validation_reservation.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                manager.abort_close(&session_id, reservation.as_ref()).await;
            });
        }
    }
}

impl InspectorSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn open_session(
        &self,
        session_id: String,
        server_id: String,
        mode: InspectorMode,
        peer: Option<Peer<RoleClient>>,
        validation_lease: Option<ValidationReservationLease>,
    ) -> InspectorSessionInfo {
        let now = Instant::now();
        let validation_reservation = validation_lease.as_ref().map(|lease| lease.token().clone());
        let entry = SessionEntry {
            server_id: server_id.clone(),
            mode,
            peer,
            validation_reservation,
            expires_at: now + SESSION_TTL,
            closing: false,
        };

        {
            let mut sessions = self.inner.write().await;
            sessions.insert(session_id.clone(), entry);
            if let Some(lease) = validation_lease {
                lease.into_persistent_token();
            }
        }

        InspectorSessionInfo {
            session_id,
            server_id,
            mode,
            expires_at_epoch_ms: session_expiry_epoch(now + SESSION_TTL),
        }
    }

    pub(crate) async fn get_session(
        &self,
        session_id: &str,
    ) -> SessionLookup {
        let mut sessions = self.inner.write().await;
        let Some(entry) = sessions.get_mut(session_id) else {
            return SessionLookup::Missing;
        };
        if entry.closing {
            return SessionLookup::Missing;
        }
        if Instant::now() > entry.expires_at {
            entry.closing = true;
            return SessionLookup::Expired(InspectorSessionClosing {
                manager: self.clone(),
                session_id: session_id.to_string(),
                info: InspectorSessionCloseInfo {
                    mode: entry.mode,
                    validation_reservation: entry.validation_reservation.clone(),
                },
                armed: true,
            });
        }
        entry.expires_at = Instant::now() + SESSION_TTL;
        SessionLookup::Active(ActiveSession {
            server_id: entry.server_id.clone(),
            mode: entry.mode,
            peer: entry.peer.clone(),
            validation_reservation: entry.validation_reservation.clone(),
        })
    }

    pub(crate) async fn begin_close(
        &self,
        session_id: &str,
    ) -> Option<InspectorSessionClosing> {
        let mut sessions = self.inner.write().await;
        let entry = sessions.get_mut(session_id)?;
        if entry.closing {
            return None;
        }
        entry.closing = true;
        Some(InspectorSessionClosing {
            manager: self.clone(),
            session_id: session_id.to_string(),
            info: InspectorSessionCloseInfo {
                mode: entry.mode,
                validation_reservation: entry.validation_reservation.clone(),
            },
            armed: true,
        })
    }

    async fn complete_close(
        &self,
        session_id: &str,
        reservation: Option<&ValidationReservationToken>,
    ) -> bool {
        let mut sessions = self.inner.write().await;
        let matches = sessions
            .get(session_id)
            .is_some_and(|entry| entry.closing && entry.validation_reservation.as_ref() == reservation);
        if matches {
            sessions.remove(session_id);
        }
        matches
    }

    async fn abort_close(
        &self,
        session_id: &str,
        reservation: Option<&ValidationReservationToken>,
    ) {
        let mut sessions = self.inner.write().await;
        if let Some(entry) = sessions.get_mut(session_id)
            && entry.validation_reservation.as_ref() == reservation
        {
            entry.closing = false;
        }
    }
}

fn session_expiry_epoch(instant: Instant) -> u128 {
    let system_time = SystemTime::now() + instant.saturating_duration_since(Instant::now());
    system_time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::Arc,
        time::{Duration, Instant},
    };

    use tokio::sync::Mutex;

    use super::{InspectorSessionManager, SessionLookup};
    use crate::{
        api::models::inspector::InspectorMode,
        core::{models::Config, pool::UpstreamConnectionPool},
    };

    fn empty_pool() -> UpstreamConnectionPool {
        UpstreamConnectionPool::new(
            Arc::new(Config {
                mcp_servers: HashMap::new(),
                pagination: None,
            }),
            None,
        )
    }

    #[tokio::test]
    async fn cancelled_manager_commit_keeps_lease_armed() {
        let pool = Arc::new(Mutex::new(empty_pool()));
        let lease =
            UpstreamConnectionPool::reserve_validation_session(&pool, "manager-cancel", Duration::from_secs(60)).await;
        let manager = InspectorSessionManager::new();
        let lock = manager.inner.write().await;
        let task_manager = manager.clone();
        let open = tokio::spawn(async move {
            task_manager
                .open_session(
                    "session-1".to_string(),
                    "server-1".to_string(),
                    InspectorMode::Native,
                    None,
                    Some(lease),
                )
                .await
        });
        tokio::task::yield_now().await;
        open.abort();
        match open.await {
            Err(error) => assert!(error.is_cancelled()),
            Ok(_) => panic!("open should be cancelled"),
        }
        drop(lock);

        tokio::time::timeout(Duration::from_secs(1), async {
            while pool.lock().await.validation_sessions.contains_key("manager-cancel") {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cancelled manager commit must release the armed lease");
    }

    #[tokio::test]
    async fn expired_entry_retains_retryable_closing_authority() {
        let pool = Arc::new(Mutex::new(empty_pool()));
        let lease =
            UpstreamConnectionPool::reserve_validation_session(&pool, "expired-entry", Duration::from_secs(60)).await;
        let token = lease.token().clone();
        let manager = InspectorSessionManager::new();
        manager
            .open_session(
                "session-1".to_string(),
                "server-1".to_string(),
                InspectorMode::Native,
                None,
                Some(lease),
            )
            .await;
        manager
            .inner
            .write()
            .await
            .get_mut("session-1")
            .expect("session entry")
            .expires_at = Instant::now() - Duration::from_millis(1);

        let closing = match manager.get_session("session-1").await {
            SessionLookup::Expired(closing) => closing,
            _ => panic!("expired entry should retain closing authority"),
        };
        assert_eq!(closing.info().validation_reservation.as_ref(), Some(&token));
        assert!(manager.inner.read().await.contains_key("session-1"));
        drop(closing);
        tokio::task::yield_now().await;
        assert!(manager.inner.read().await.contains_key("session-1"));
    }

    #[tokio::test]
    async fn cancelled_close_retains_authority_for_retry() {
        let pool = Arc::new(Mutex::new(empty_pool()));
        let lease =
            UpstreamConnectionPool::reserve_validation_session(&pool, "retry-close", Duration::from_secs(60)).await;
        let token = lease.token().clone();
        let manager = InspectorSessionManager::new();
        manager
            .open_session(
                "session-1".to_string(),
                "server-1".to_string(),
                InspectorMode::Native,
                None,
                Some(lease),
            )
            .await;

        let pool_lock = pool.lock().await;
        let closing = manager.begin_close("session-1").await.expect("begin close");
        let task_pool = pool.clone();
        let close = tokio::spawn(async move {
            UpstreamConnectionPool::release_validation_reservation(&task_pool, &token)
                .await
                .expect("release reservation");
            closing.complete().await
        });
        tokio::task::yield_now().await;
        close.abort();
        assert!(close.await.expect_err("close should be cancelled").is_cancelled());
        drop(pool_lock);
        tokio::task::yield_now().await;

        assert!(manager.inner.read().await.contains_key("session-1"));
        let retry = manager.begin_close("session-1").await.expect("retry close");
        let retry_token = retry.info().validation_reservation.clone().expect("native reservation");
        UpstreamConnectionPool::release_validation_reservation(&pool, &retry_token)
            .await
            .expect("retry release");
        assert!(retry.complete().await);
        assert!(!manager.inner.read().await.contains_key("session-1"));
    }

    #[tokio::test]
    async fn close_authority_is_issued_once_until_abort() {
        let manager = InspectorSessionManager::new();
        manager
            .open_session(
                "session-1".to_string(),
                "server-1".to_string(),
                InspectorMode::Proxy,
                None,
                None,
            )
            .await;

        let closing = manager.begin_close("session-1").await.expect("begin close");
        assert!(manager.begin_close("session-1").await.is_none());
        assert!(matches!(manager.get_session("session-1").await, SessionLookup::Missing));

        drop(closing);
        tokio::task::yield_now().await;
        assert!(matches!(
            manager.get_session("session-1").await,
            SessionLookup::Active(_)
        ));
    }
}
