use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rmcp::RoleClient;
use rmcp::service::Peer;
use tokio::sync::RwLock;

use crate::api::models::inspector::InspectorMode;

const SESSION_TTL: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub struct InspectorSessionInfo {
    pub session_id: String,
    pub server_id: String,
    pub mode: InspectorMode,
    pub expires_at_epoch_ms: u128,
}

struct SessionEntry {
    session_id: String,
    server_id: String,
    mode: InspectorMode,
    peer: Peer<RoleClient>,
    validation_session: Option<String>,
    expires_at: Instant,
}

#[derive(Default, Clone)]
pub struct InspectorSessionManager {
    inner: Arc<RwLock<HashMap<String, SessionEntry>>>,
}

#[derive(Clone)]
pub struct ActiveSession {
    pub session_id: String,
    pub server_id: String,
    pub mode: InspectorMode,
    pub peer: Peer<RoleClient>,
}

impl InspectorSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn open_session(
        &self,
        session_id: String,
        server_id: String,
        mode: InspectorMode,
        peer: Peer<RoleClient>,
        validation_session: Option<String>,
    ) -> InspectorSessionInfo {
        let now = Instant::now();
        let entry = SessionEntry {
            session_id: session_id.clone(),
            server_id: server_id.clone(),
            mode,
            peer,
            validation_session,
            expires_at: now + SESSION_TTL,
        };

        {
            let mut sessions = self.inner.write().await;
            sessions.insert(session_id.clone(), entry);
        }

        InspectorSessionInfo {
            session_id,
            server_id,
            mode,
            expires_at_epoch_ms: session_expiry_epoch(now + SESSION_TTL),
        }
    }

    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Option<ActiveSession> {
        let mut remove = false;
        let result = {
            let mut sessions = self.inner.write().await;
            if let Some(entry) = sessions.get_mut(session_id) {
                if Instant::now() > entry.expires_at {
                    remove = true;
                    None
                } else {
                    entry.expires_at = Instant::now() + SESSION_TTL;
                    Some(ActiveSession {
                        session_id: entry.session_id.clone(),
                        server_id: entry.server_id.clone(),
                        mode: entry.mode,
                        peer: entry.peer.clone(),
                    })
                }
            } else {
                None
            }
        };
        if remove {
            self.inner.write().await.remove(session_id);
        }
        result
    }

    pub async fn close_session(
        &self,
        session_id: &str,
    ) -> Option<ClosedSessionInfo> {
        let mut sessions = self.inner.write().await;
        sessions.remove(session_id).map(|entry| ClosedSessionInfo {
            server_id: entry.server_id,
            mode: entry.mode,
            validation_session: entry.validation_session,
        })
    }

    pub async fn sweep_expired(&self) {
        let mut sessions = self.inner.write().await;
        let now = Instant::now();
        sessions.retain(|_, entry| entry.expires_at > now);
    }
}

#[derive(Clone)]
pub struct ClosedSessionInfo {
    pub server_id: String,
    pub mode: InspectorMode,
    pub validation_session: Option<String>,
}

fn session_expiry_epoch(instant: Instant) -> u128 {
    let system_time = SystemTime::now() + instant.saturating_duration_since(Instant::now());
    system_time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default()
}
