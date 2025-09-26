use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallStatus {
    Pending,
    Running,
    Ok,
    Error,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct CallSummary {
    pub call_id: String,
    pub mode: String,
    pub capability: String,
    pub action: String,
    pub target: Option<String>,
    pub status: CallStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub elapsed_ms: Option<u64>,
    pub progress: Option<u8>,
    pub last_seq: u64,
    pub last_error: Option<String>,
}

impl CallSummary {
    pub fn new(
        call_id: &str,
        mode: &str,
        capability: &str,
        action: &str,
        target: Option<String>,
    ) -> Self {
        Self {
            call_id: call_id.to_string(),
            mode: mode.to_string(),
            capability: capability.to_string(),
            action: action.to_string(),
            target,
            status: CallStatus::Pending,
            started_at: Utc::now(),
            finished_at: None,
            elapsed_ms: None,
            progress: None,
            last_seq: 0,
            last_error: None,
        }
    }
}

#[derive(Default, Clone)]
pub struct CallRegistry {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Default)]
struct Inner {
    map: HashMap<String, CallSummary>,
    order: VecDeque<String>,
    capacity: usize,
}

static GLOBAL: Lazy<CallRegistry> = Lazy::new(|| {
    let r = CallRegistry::default();
    tokio::spawn({
        let r = r.clone();
        async move {
            r.set_capacity(200).await;
        }
    });
    r
});

impl CallRegistry {
    pub fn global() -> Self {
        GLOBAL.clone()
    }
    pub async fn set_capacity(
        &self,
        capacity: usize,
    ) {
        let mut i = self.inner.write().await;
        i.capacity = capacity.max(16);
        while i.order.len() > i.capacity {
            if let Some(old) = i.order.pop_back() {
                i.map.remove(&old);
            }
        }
    }
    pub async fn insert(
        &self,
        summary: CallSummary,
    ) {
        let mut i = self.inner.write().await;
        i.order.push_front(summary.call_id.clone());
        i.map.insert(summary.call_id.clone(), summary);
        while i.order.len() > i.capacity {
            if let Some(old) = i.order.pop_back() {
                i.map.remove(&old);
            }
        }
    }
    pub async fn update_progress(
        &self,
        call_id: &str,
        seq: u64,
        p: u8,
    ) {
        let mut i = self.inner.write().await;
        if let Some(s) = i.map.get_mut(call_id) {
            s.status = CallStatus::Running;
            s.last_seq = seq;
            s.progress = Some(p);
        }
    }
    pub async fn update_status(
        &self,
        call_id: &str,
        status: CallStatus,
        seq: u64,
        err: Option<String>,
        elapsed: Option<u64>,
    ) {
        let mut i = self.inner.write().await;
        if let Some(s) = i.map.get_mut(call_id) {
            s.status = status;
            s.last_seq = seq;
            s.finished_at = Some(Utc::now());
            s.elapsed_ms = elapsed;
            s.last_error = err;
        }
    }
    pub async fn recent(
        &self,
        limit: usize,
    ) -> Vec<CallSummary> {
        let i = self.inner.read().await;
        i.order
            .iter()
            .take(limit)
            .filter_map(|id| i.map.get(id).cloned())
            .collect()
    }
    pub async fn get(
        &self,
        call_id: &str,
    ) -> Option<CallSummary> {
        let i = self.inner.read().await;
        i.map.get(call_id).cloned()
    }
    pub async fn clear(&self) {
        let mut i = self.inner.write().await;
        i.map.clear();
        i.order.clear();
    }
}
