use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rmcp::RoleClient;
use rmcp::service::Peer;
use tokio::sync::RwLock;

use crate::inspector::runtime::{self, InspectorRuntimeOwner};
use crate::inspector::target::InspectorTarget;

const SESSION_TTL: Duration = Duration::from_secs(300);
const SESSION_SWEEP_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct InspectorSessionInfo {
    pub session_id: String,
    pub target: InspectorTarget,
    pub expires_at_epoch_ms: u128,
}

struct SessionEntry {
    session_id: String,
    target: InspectorTarget,
    peer: Option<Peer<RoleClient>>,
    runtime_owner: Option<InspectorRuntimeOwner>,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct InspectorSessionManager {
    inner: Arc<RwLock<HashMap<String, SessionEntry>>>,
}

#[derive(Clone)]
pub struct ActiveSession {
    pub session_id: String,
    pub target: InspectorTarget,
    pub peer: Option<Peer<RoleClient>>,
    pub expires_at_epoch_ms: u128,
}

pub enum SessionLookup {
    Active(ActiveSession),
    Expired(ClosedSessionInfo),
    Missing,
}

impl InspectorSessionManager {
    pub fn new() -> Self {
        let manager = Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        };
        manager.start_expiry_sweeper();
        manager
    }

    pub async fn open_session(
        &self,
        session_id: String,
        target: InspectorTarget,
        peer: Option<Peer<RoleClient>>,
        runtime_owner: Option<InspectorRuntimeOwner>,
    ) -> InspectorSessionInfo {
        let now = Instant::now();
        let entry = SessionEntry {
            session_id: session_id.clone(),
            target: target.clone(),
            peer,
            runtime_owner,
            expires_at: now + SESSION_TTL,
        };

        {
            let mut sessions = self.inner.write().await;
            sessions.insert(session_id.clone(), entry);
        }

        InspectorSessionInfo {
            session_id,
            target,
            expires_at_epoch_ms: session_expiry_epoch(now + SESSION_TTL),
        }
    }

    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Option<ActiveSession> {
        match self.get_session_or_expired(session_id).await {
            SessionLookup::Active(session) => Some(session),
            SessionLookup::Expired(closed) => {
                closed.cleanup_runtime().await;
                None
            }
            SessionLookup::Missing => None,
        }
    }

    pub async fn get_session_or_expired(
        &self,
        session_id: &str,
    ) -> SessionLookup {
        let mut sessions = self.inner.write().await;
        let Some(entry) = sessions.get_mut(session_id) else {
            return SessionLookup::Missing;
        };

        if Instant::now() > entry.expires_at {
            return sessions
                .remove(session_id)
                .map(closed_session_info)
                .map(SessionLookup::Expired)
                .unwrap_or(SessionLookup::Missing);
        }

        let expires_at = Instant::now() + SESSION_TTL;
        entry.expires_at = expires_at;
        SessionLookup::Active(ActiveSession {
            session_id: entry.session_id.clone(),
            target: entry.target.clone(),
            peer: entry.peer.clone(),
            expires_at_epoch_ms: session_expiry_epoch(expires_at),
        })
    }

    pub async fn close_session(
        &self,
        session_id: &str,
    ) -> Option<ClosedSessionInfo> {
        let mut sessions = self.inner.write().await;
        sessions.remove(session_id).map(closed_session_info)
    }

    pub async fn sweep_expired(&self) -> Vec<ClosedSessionInfo> {
        let mut sessions = self.inner.write().await;
        let now = Instant::now();
        let expired_session_ids = sessions
            .iter()
            .filter(|(_, entry)| entry.expires_at <= now)
            .map(|(session_id, _)| session_id.clone())
            .collect::<Vec<_>>();

        expired_session_ids
            .into_iter()
            .filter_map(|session_id| sessions.remove(&session_id).map(closed_session_info))
            .collect()
    }

    fn start_expiry_sweeper(&self) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            tracing::warn!("Inspector session expiry sweeper was not started because no Tokio runtime is active");
            return;
        };
        let manager = self.clone();
        handle.spawn(async move {
            let mut interval = tokio::time::interval(SESSION_SWEEP_INTERVAL);
            loop {
                interval.tick().await;
                let closed_sessions = manager.sweep_expired().await;
                for closed in closed_sessions {
                    closed.cleanup_runtime().await;
                }
            }
        });
    }
}

impl Default for InspectorSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ClosedSessionInfo {
    pub target: InspectorTarget,
    pub runtime_owner: Option<InspectorRuntimeOwner>,
}

impl ClosedSessionInfo {
    pub async fn cleanup_runtime(self) {
        if let Some(owner) = self.runtime_owner {
            runtime::cancel_runtime_owner(owner).await;
        }
    }
}

fn closed_session_info(entry: SessionEntry) -> ClosedSessionInfo {
    ClosedSessionInfo {
        target: entry.target,
        runtime_owner: entry.runtime_owner,
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
    use super::*;
    use crate::inspector::contract::{InspectorMode, InspectorProxyMode, InspectorProxyScope};
    use crate::inspector::target::InspectorProxyTarget;

    #[tokio::test]
    async fn sweep_expired_removes_only_expired_sessions() {
        let manager = InspectorSessionManager {
            inner: Arc::new(RwLock::new(HashMap::new())),
        };
        let now = Instant::now();
        let expired_id = "expired-session".to_string();
        let active_id = "active-session".to_string();

        {
            let mut sessions = manager.inner.write().await;
            sessions.insert(
                expired_id.clone(),
                SessionEntry {
                    session_id: expired_id.clone(),
                    target: InspectorTarget::native("expired-server".to_string()),
                    peer: None,
                    runtime_owner: None,
                    expires_at: now - Duration::from_secs(1),
                },
            );
            sessions.insert(
                active_id.clone(),
                SessionEntry {
                    session_id: active_id.clone(),
                    target: InspectorTarget::proxy(
                        InspectorProxyTarget::from_parts(
                            Some(InspectorProxyMode::Unify),
                            Some(InspectorProxyScope::ActiveCatalog),
                            Some(vec!["active-server".to_string()]),
                        )
                        .expect("active catalog proxy target"),
                    ),
                    peer: None,
                    runtime_owner: None,
                    expires_at: now + Duration::from_secs(60),
                },
            );
        }

        let closed = manager.sweep_expired().await;

        assert_eq!(closed.len(), 1);
        let closed_session = closed.first().expect("expired session should be closed");
        assert_eq!(closed_session.target.server_id(), Some("expired-server"));
        assert_eq!(closed_session.target.mode(), InspectorMode::Native);
        assert!(closed_session.runtime_owner.is_none());

        let sessions = manager.inner.read().await;
        assert!(!sessions.contains_key(&expired_id));
        assert!(sessions.contains_key(&active_id));
    }
}
