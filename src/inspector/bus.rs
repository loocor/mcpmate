use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, broadcast};

use super::sse::SseEvent;

#[derive(Clone, Default)]
pub struct CallBus {
    inner: Arc<RwLock<HashMap<String, broadcast::Sender<SseEvent>>>>,
    cancelled: Arc<RwLock<HashMap<String, bool>>>,
}

impl CallBus {
    pub fn new() -> Self {
        Self::default()
    }
    pub async fn create_call(
        &self,
        call_id: &str,
        capacity: usize,
    ) -> broadcast::Receiver<SseEvent> {
        let mut map = self.inner.write().await;
        let (tx, rx) = broadcast::channel::<SseEvent>(capacity.max(16));
        map.insert(call_id.to_string(), tx);
        let mut flags = self.cancelled.write().await;
        flags.insert(call_id.to_string(), false);
        rx
    }
    pub async fn subscribe(
        &self,
        call_id: &str,
    ) -> Option<broadcast::Receiver<SseEvent>> {
        let map = self.inner.read().await;
        map.get(call_id).map(|tx| tx.subscribe())
    }
    pub async fn publish(
        &self,
        call_id: &str,
        event: SseEvent,
    ) -> bool {
        let map = self.inner.read().await;
        if let Some(tx) = map.get(call_id) {
            let _ = tx.send(event);
            true
        } else {
            false
        }
    }
    pub async fn finish(
        &self,
        call_id: &str,
    ) {
        let mut map = self.inner.write().await;
        map.remove(call_id);
        let mut flags = self.cancelled.write().await;
        flags.remove(call_id);
    }
    pub async fn cancel(
        &self,
        call_id: &str,
    ) {
        let mut f = self.cancelled.write().await;
        if let Some(v) = f.get_mut(call_id) {
            *v = true;
        }
    }
    pub async fn is_cancelled(
        &self,
        call_id: &str,
    ) -> bool {
        let f = self.cancelled.read().await;
        f.get(call_id).copied().unwrap_or(false)
    }
}

// Global singleton for bus
use once_cell::sync::Lazy;
static GLOBAL_BUS: Lazy<CallBus> = Lazy::new(CallBus::new);
pub fn global() -> CallBus {
    GLOBAL_BUS.clone()
}
