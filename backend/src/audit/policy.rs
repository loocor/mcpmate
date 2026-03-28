//! Audit retention policy and background worker.
//!
//! This module implements Task 9 from the audit log implementation plan:
//! - Time + capacity retention policy configuration
//! - Periodic retention sweep orchestration
//! - Startup and scheduled execution with concurrency safety

use std::sync::Arc;

use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::storage::AuditStore;

/// Default retention interval in seconds (1 hour).
const DEFAULT_SWEEP_INTERVAL_SECS: u64 = 3600;

/// Default maximum age in days for retention.
const DEFAULT_MAX_AGE_DAYS: u32 = 30;

/// Default maximum row count for retention.
const DEFAULT_MAX_ROWS: u32 = 100_000;

/// Audit retention policy type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AuditRetentionPolicy {
    /// No automatic retention; keep all records.
    Off,
    /// Keep records newer than a specified number of days.
    KeepDays {
        /// Maximum age in days. Records older than this are deleted.
        days: u32,
    },
    /// Keep only the most recent N records.
    KeepCount {
        /// Maximum number of records to retain.
        count: u32,
    },
    /// Apply both age and count limits (records must satisfy both conditions to be kept).
    Combined {
        /// Maximum age in days.
        days: u32,
        /// Maximum number of records.
        count: u32,
    },
}

impl Default for AuditRetentionPolicy {
    fn default() -> Self {
        Self::Combined {
            days: DEFAULT_MAX_AGE_DAYS,
            count: DEFAULT_MAX_ROWS,
        }
    }
}

impl AuditRetentionPolicy {
    /// Returns true if retention is disabled.
    pub fn is_off(&self) -> bool {
        matches!(self, Self::Off)
    }

    /// Returns the maximum age in days, if applicable.
    pub fn max_age_days(&self) -> Option<u32> {
        match self {
            Self::KeepDays { days } | Self::Combined { days, .. } => Some(*days),
            Self::Off | Self::KeepCount { .. } => None,
        }
    }

    /// Returns the maximum row count, if applicable.
    pub fn max_rows(&self) -> Option<u32> {
        match self {
            Self::KeepCount { count } | Self::Combined { count, .. } => Some(*count),
            Self::Off | Self::KeepDays { .. } => None,
        }
    }
}

/// Complete audit retention policy setting including sweep interval.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AuditRetentionPolicySetting {
    /// The retention policy to apply.
    pub policy: AuditRetentionPolicy,
    /// Interval between retention sweeps in seconds.
    #[serde(default = "default_sweep_interval_secs")]
    pub sweep_interval_secs: u64,
}

fn default_sweep_interval_secs() -> u64 {
    DEFAULT_SWEEP_INTERVAL_SECS
}

impl Default for AuditRetentionPolicySetting {
    fn default() -> Self {
        Self {
            policy: AuditRetentionPolicy::default(),
            sweep_interval_secs: DEFAULT_SWEEP_INTERVAL_SECS,
        }
    }
}

impl AuditRetentionPolicySetting {
    /// Creates a new policy setting with default sweep interval.
    pub fn new(policy: AuditRetentionPolicy) -> Self {
        Self {
            policy,
            sweep_interval_secs: DEFAULT_SWEEP_INTERVAL_SECS,
        }
    }

    /// Creates a policy setting with a custom sweep interval.
    pub fn with_sweep_interval(
        mut self,
        secs: u64,
    ) -> Self {
        self.sweep_interval_secs = secs;
        self
    }
}

/// Runs the audit retention worker in the background.
///
/// The worker periodically applies the retention policy to purge old or excess
/// audit records. It respects the cancellation token for graceful shutdown.
pub async fn run_retention_worker(
    store: Arc<AuditStore>,
    policy: AuditRetentionPolicySetting,
    cancellation_token: CancellationToken,
) {
    if policy.policy.is_off() {
        tracing::info!("Audit retention policy is off; worker will not run");
        return;
    }

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(policy.sweep_interval_secs));

    tracing::info!(
        policy = ?policy.policy,
        sweep_interval_secs = policy.sweep_interval_secs,
        "Starting audit retention worker"
    );

    loop {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                tracing::info!("Audit retention worker shutting down");
                break;
            }
            _ = interval.tick() => {
                if let Err(error) = apply_retention_policy(&store, &policy.policy).await {
                    tracing::warn!(error = %error, "Failed to apply audit retention policy");
                }
            }
        }
    }
}

/// Applies the retention policy once.
///
/// Returns the total number of records deleted.
pub async fn apply_retention_policy(
    store: &AuditStore,
    policy: &AuditRetentionPolicy,
) -> Result<u64> {
    if policy.is_off() {
        return Ok(0);
    }

    let mut total_deleted = 0u64;

    // Apply age-based retention first
    if let Some(days) = policy.max_age_days() {
        let cutoff_ms = chrono::Utc::now().timestamp_millis() - (days as i64 * 24 * 60 * 60 * 1000);

        let deleted = store.purge_older_than(cutoff_ms).await?;
        if deleted > 0 {
            tracing::info!(
                days = days,
                deleted = deleted,
                "Purged audit records older than retention age"
            );
        }
        total_deleted += deleted;
    }

    // Apply capacity-based retention
    if let Some(count) = policy.max_rows() {
        let deleted = store.enforce_capacity(count as i64).await?;
        if deleted > 0 {
            tracing::info!(
                max_rows = count,
                deleted = deleted,
                "Purged excess audit records to enforce capacity limit"
            );
        }
        total_deleted += deleted;
    }

    Ok(total_deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_default_is_combined() {
        let policy = AuditRetentionPolicy::default();
        assert!(matches!(policy, AuditRetentionPolicy::Combined { .. }));
        assert!(!policy.is_off());
    }

    #[test]
    fn policy_off_disables_retention() {
        let policy = AuditRetentionPolicy::Off;
        assert!(policy.is_off());
        assert_eq!(policy.max_age_days(), None);
        assert_eq!(policy.max_rows(), None);
    }

    #[test]
    fn policy_keep_days_extracts_age() {
        let policy = AuditRetentionPolicy::KeepDays { days: 7 };
        assert!(!policy.is_off());
        assert_eq!(policy.max_age_days(), Some(7));
        assert_eq!(policy.max_rows(), None);
    }

    #[test]
    fn policy_keep_count_extracts_count() {
        let policy = AuditRetentionPolicy::KeepCount { count: 1000 };
        assert!(!policy.is_off());
        assert_eq!(policy.max_age_days(), None);
        assert_eq!(policy.max_rows(), Some(1000));
    }

    #[test]
    fn policy_combined_extracts_both() {
        let policy = AuditRetentionPolicy::Combined { days: 14, count: 5000 };
        assert!(!policy.is_off());
        assert_eq!(policy.max_age_days(), Some(14));
        assert_eq!(policy.max_rows(), Some(5000));
    }

    #[test]
    fn setting_default_uses_policy_default() {
        let setting = AuditRetentionPolicySetting::default();
        assert!(matches!(setting.policy, AuditRetentionPolicy::Combined { .. }));
        assert_eq!(setting.sweep_interval_secs, DEFAULT_SWEEP_INTERVAL_SECS);
    }

    #[test]
    fn setting_new_uses_default_interval() {
        let setting = AuditRetentionPolicySetting::new(AuditRetentionPolicy::KeepDays { days: 7 });
        assert_eq!(setting.sweep_interval_secs, DEFAULT_SWEEP_INTERVAL_SECS);
    }

    #[test]
    fn setting_with_sweep_interval_customizes() {
        let setting = AuditRetentionPolicySetting::new(AuditRetentionPolicy::Off).with_sweep_interval(7200);
        assert_eq!(setting.sweep_interval_secs, 7200);
    }

    #[test]
    fn policy_serializes_correctly() {
        let policy = AuditRetentionPolicy::Combined { days: 30, count: 10000 };
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("combined"));

        let parsed: AuditRetentionPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, parsed);
    }

    #[test]
    fn setting_serializes_correctly() {
        let setting = AuditRetentionPolicySetting {
            policy: AuditRetentionPolicy::KeepDays { days: 7 },
            sweep_interval_secs: 1800,
        };
        let json = serde_json::to_string(&setting).unwrap();

        let parsed: AuditRetentionPolicySetting = serde_json::from_str(&json).unwrap();
        assert_eq!(setting.policy, parsed.policy);
        assert_eq!(setting.sweep_interval_secs, parsed.sweep_interval_secs);
    }
}
