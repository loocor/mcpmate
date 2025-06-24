//! Simplified Configuration Application State Manager
//!
//! This module provides basic state tracking for configuration application processes,
//! maintaining API compatibility while simplifying the implementation.

use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::api::models::system::ConfigApplicationStatus;

/// Simplified configuration application state manager
#[derive(Debug)]
pub struct ConfigApplicationStateManager {
    /// Current status for API compatibility
    current_status: Arc<RwLock<Option<ConfigApplicationStatus>>>,
}

impl ConfigApplicationStateManager {
    /// Create a new configuration application state manager
    pub fn new() -> Self {
        Self {
            current_status: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the state manager (simplified - no event subscription)
    pub async fn initialize(&self) {
        info!("Simplified configuration application state manager initialized");
    }

    /// Get the current configuration application status for API endpoints
    pub async fn get_current_status(&self) -> Option<ConfigApplicationStatus> {
        let status_guard = self.current_status.read().await;
        status_guard.clone()
    }

    /// Set configuration application status (simplified interface)
    pub async fn set_status(&self, status: Option<ConfigApplicationStatus>) {
        let mut status_guard = self.current_status.write().await;

        if let Some(ref status) = status {
            debug!("Configuration application status updated: in_progress={}", status.in_progress);
        } else {
            debug!("Configuration application status cleared");
        }

        *status_guard = status;
    }

    /// Start configuration application (simplified)
    pub async fn start_application(&self, suit_id: String, total_servers: usize) {
        let status = ConfigApplicationStatus {
            in_progress: true,
            suit_id: Some(suit_id.clone()),
            current_stage: Some("Starting".to_string()),
            progress_percentage: Some(0),
            estimated_remaining_seconds: Some((total_servers * 2) as u32),
            started_at: Some(Utc::now().to_rfc3339()),
            total_servers: Some(total_servers),
            servers_started: Some(0),
            servers_stopped: Some(0),
            failed_operations: None,
        };

        self.set_status(Some(status)).await;
        info!("Configuration application started for suit: {}", suit_id);
    }

    /// Complete configuration application (simplified)
    pub async fn complete_application(&self) {
        let mut status_guard = self.current_status.write().await;
        if let Some(ref mut status) = *status_guard {
            status.in_progress = false;
            status.current_stage = Some("Completed".to_string());
            status.progress_percentage = Some(100);
            status.estimated_remaining_seconds = None;
        }

        info!("Configuration application completed");

        // Clear status after a short delay
        let current_status_clone = self.current_status.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let mut guard = current_status_clone.write().await;
            *guard = None;
            debug!("Cleared completed configuration application status");
        });
    }


}

impl Clone for ConfigApplicationStateManager {
    fn clone(&self) -> Self {
        Self {
            current_status: self.current_status.clone(),
        }
    }
}

impl Default for ConfigApplicationStateManager {
    fn default() -> Self {
        Self::new()
    }
}
