//! Unified data synchronization framework
//!
//! Provides standardized patterns for synchronizing data between different
//! storage systems (database, cache, external services) to eliminate
//! code duplication and ensure consistent sync behavior.

use anyhow::{Context, Result};
use tracing;

/// Synchronization operation result
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of items processed
    pub processed: usize,
    /// Number of items successfully synced
    pub synced: usize,
    /// Number of items that failed to sync
    pub failed: usize,
    /// Error messages for failed items
    pub errors: Vec<String>,
}

impl SyncResult {
    /// Create a new sync result
    pub fn new() -> Self {
        Self {
            processed: 0,
            synced: 0,
            failed: 0,
            errors: Vec::new(),
        }
    }

    /// Add a successful sync
    pub fn add_success(&mut self) {
        self.processed += 1;
        self.synced += 1;
    }

    /// Add a failed sync
    pub fn add_failure(
        &mut self,
        error: String,
    ) {
        self.processed += 1;
        self.failed += 1;
        self.errors.push(error);
    }

    /// Check if all syncs were successful
    pub fn is_success(&self) -> bool {
        self.failed == 0 && self.processed > 0
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.processed == 0 {
            0.0
        } else {
            (self.synced as f64 / self.processed as f64) * 100.0
        }
    }
}

impl Default for SyncResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Synchronization context containing common data needed for sync operations
#[derive(Debug, Clone)]
pub struct SyncContext {
    /// Server ID being synced
    pub server_id: String,
    /// Profile IDs that include this server
    pub profile_ids: Vec<String>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl SyncContext {
    /// Create a new sync context
    pub fn new(server_id: String) -> Self {
        Self {
            server_id,
            profile_ids: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add a profile ID to the context
    pub fn add_profile(
        &mut self,
        profile_id: String,
    ) {
        self.profile_ids.push(profile_id);
    }

    /// Add metadata to the context
    pub fn add_metadata(
        &mut self,
        key: String,
        value: String,
    ) {
        self.metadata.insert(key, value);
    }
}

/// Generic synchronization helper for common sync patterns
pub struct SyncHelper;

impl SyncHelper {
    /// Get server and associated profile for sync operations
    ///
    /// This is a common pattern used across different sync operations
    pub async fn get_server_context(
        db_pool: &sqlx::Pool<sqlx::Sqlite>,
        server_id: &str,
    ) -> Result<SyncContext> {
        tracing::debug!("Getting sync context for server '{}'", server_id);

        // Verify the server exists
        let server = crate::config::server::get_server_by_id(db_pool, server_id)
            .await
            .with_context(|| format!("Failed to get server '{}'", server_id))?;

        if server.is_none() {
            return Err(anyhow::anyhow!("Server '{}' not found", server_id));
        }

        // Get all profile that have this server enabled
        let profile_with_server = Self::get_profile_with_server(db_pool, server_id).await?;

        let mut context = SyncContext::new(server_id.to_string());

        // Add profile IDs to context
        for profile in profile_with_server {
            if let Some(profile_id) = profile.id {
                context.add_profile(profile_id);
            }
        }

        tracing::debug!(
            "Found {} profile for server '{}'",
            context.profile_ids.len(),
            server_id
        );

        Ok(context)
    }

    /// Helper function to get profile that have a specific server enabled
    async fn get_profile_with_server(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        server_id: &str,
    ) -> Result<Vec<crate::config::models::Profile>> {
        use crate::common::database::fetch_where;

        tracing::debug!("Getting profile that include server '{}'", server_id);

        // Get all profile
        let all_profile = crate::config::profile::get_all_profile(pool)
            .await
            .context("Failed to get all profile")?;

        // Filter profile that have this server enabled
        let mut profile_with_server = Vec::new();

        for profile in all_profile {
            if let Some(profile_id) = &profile.id {
                // Check if this profile has the server enabled
                let server_enabled: Vec<crate::config::models::ProfileServer> =
                    fetch_where(pool, "profile_server", "profile_id", profile_id, None)
                        .await
                        .context("Failed to check profile server associations")?;

                if server_enabled.iter().any(|s| s.server_id == server_id && s.enabled) {
                    profile_with_server.push(profile);
                }
            }
        }

        tracing::debug!(
            "Found {} profile with server '{}' enabled",
            profile_with_server.len(),
            server_id
        );

        Ok(profile_with_server)
    }

    /// Execute a batch sync operation with consistent error handling and logging
    ///
    /// This provides a standard pattern for batch operations across different sync types
    pub async fn execute_batch_sync<T, F, Fut>(
        items: Vec<T>,
        operation_name: &str,
        sync_fn: F,
    ) -> SyncResult
    where
        F: Fn(T) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut result = SyncResult::new();
        let total_items = items.len();

        tracing::info!("Starting batch {} for {} items", operation_name, total_items);

        for (index, item) in items.into_iter().enumerate() {
            match sync_fn(item).await {
                Ok(()) => {
                    result.add_success();
                    tracing::debug!("Successfully synced item {} of {}", index + 1, total_items);
                }
                Err(e) => {
                    let error_msg = format!("Failed to sync item {}: {}", index + 1, e);
                    result.add_failure(error_msg.clone());
                    tracing::warn!("{}", error_msg);
                }
            }
        }

        tracing::info!(
            "Completed batch {}: {}/{} items synced successfully ({:.1}% success rate)",
            operation_name,
            result.synced,
            result.processed,
            result.success_rate()
        );

        if !result.errors.is_empty() {
            tracing::warn!("Sync errors encountered: {:?}", result.errors);
        }

        result
    }

    /// Execute concurrent sync operations with controlled parallelism
    ///
    /// This provides a standard pattern for concurrent operations with proper error handling
    pub async fn execute_concurrent_sync<T, F, Fut>(
        items: Vec<T>,
        operation_name: &str,
        max_concurrent: usize,
        sync_fn: F,
    ) -> SyncResult
    where
        T: Send + 'static,
        F: Fn(T) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        use futures::stream::{self, StreamExt};

        let mut result = SyncResult::new();
        let total_items = items.len();

        tracing::info!(
            "Starting concurrent {} for {} items with max {} concurrent operations",
            operation_name,
            total_items,
            max_concurrent
        );

        let results: Vec<Result<()>> = stream::iter(items)
            .map(|item| {
                let sync_fn = sync_fn.clone();
                async move { sync_fn(item).await }
            })
            .buffer_unordered(max_concurrent)
            .collect()
            .await;

        for (index, sync_result) in results.into_iter().enumerate() {
            match sync_result {
                Ok(()) => {
                    result.add_success();
                    tracing::debug!("Successfully synced item {} of {}", index + 1, total_items);
                }
                Err(e) => {
                    let error_msg = format!("Failed to sync item {}: {}", index + 1, e);
                    result.add_failure(error_msg.clone());
                    tracing::warn!("{}", error_msg);
                }
            }
        }

        tracing::info!(
            "Completed concurrent {}: {}/{} items synced successfully ({:.1}% success rate)",
            operation_name,
            result.synced,
            result.processed,
            result.success_rate()
        );

        if !result.errors.is_empty() {
            tracing::warn!("Sync errors encountered: {:?}", result.errors);
        }

        result
    }

    /// Clean up orphaned records in a target table based on a source table
    ///
    /// This is a common pattern for maintaining referential integrity during sync operations
    pub async fn cleanup_orphaned_records(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        target_table: &str,
        target_foreign_key: &str,
        source_table: &str,
        source_primary_key: &str,
    ) -> Result<usize> {
        let query = format!(
            r#"
            DELETE FROM {}
            WHERE {} NOT IN (
                SELECT {} FROM {}
            )
            "#,
            target_table, target_foreign_key, source_primary_key, source_table
        );

        tracing::debug!(
            "Cleaning up orphaned records in {} where {} not in {}.{}",
            target_table,
            target_foreign_key,
            source_table,
            source_primary_key
        );

        let result = sqlx::query(&query)
            .execute(pool)
            .await
            .with_context(|| format!("Failed to cleanup orphaned records in {} table", target_table))?;

        let deleted_count = result.rows_affected() as usize;

        if deleted_count > 0 {
            tracing::info!(
                "Cleaned up {} orphaned records from {} table",
                deleted_count,
                target_table
            );
        } else {
            tracing::debug!("No orphaned records found in {} table", target_table);
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_result() {
        let mut result = SyncResult::new();
        assert_eq!(result.processed, 0);
        assert_eq!(result.synced, 0);
        assert_eq!(result.failed, 0);
        assert!(result.errors.is_empty());

        result.add_success();
        assert_eq!(result.processed, 1);
        assert_eq!(result.synced, 1);
        assert_eq!(result.failed, 0);
        assert!(result.is_success());

        result.add_failure("Test error".to_string());
        assert_eq!(result.processed, 2);
        assert_eq!(result.synced, 1);
        assert_eq!(result.failed, 1);
        assert!(!result.is_success());
        assert_eq!(result.success_rate(), 50.0);
    }

    #[test]
    fn test_sync_context() {
        let mut context = SyncContext::new("server1".to_string());
        assert_eq!(context.server_id, "server1");
        assert!(context.profile_ids.is_empty());
        assert!(context.metadata.is_empty());

        context.add_profile("profile1".to_string());
        context.add_metadata("key1".to_string(), "value1".to_string());

        assert_eq!(context.profile_ids.len(), 1);
        assert_eq!(context.metadata.len(), 1);
        assert_eq!(context.metadata.get("key1"), Some(&"value1".to_string()));
    }

    #[tokio::test]
    async fn test_execute_batch_sync() {
        let items = vec![1, 2, 3, 4, 5];

        let result = SyncHelper::execute_batch_sync(items, "test_operation", |item| async move {
            if item % 2 == 0 {
                Err(anyhow::anyhow!("Even number error"))
            } else {
                Ok(())
            }
        })
        .await;

        assert_eq!(result.processed, 5);
        assert_eq!(result.synced, 3); // 1, 3, 5
        assert_eq!(result.failed, 2); // 2, 4
        assert_eq!(result.success_rate(), 60.0);
    }
}
