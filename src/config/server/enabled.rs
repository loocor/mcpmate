// Server enabled status management
// Contains unified server enabled status service and utility functions
//
// This file provides a single source of truth for determining server enabled status.
// It includes both the core ServerEnabledService and backward-compatible utility functions.

use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;

use super::crud::get_server_by_id;
use crate::{common::profile::USER_PROFILE_INITIAL_NAME, config::models::Server};

// ============================================================================
// CORE SERVICE - ServerEnabledService
// ============================================================================

/// Unified service for determining server enabled status
///
/// This service implements the canonical logic for checking if a server is enabled:
/// 1. Server must be enabled in at least one active profile
/// 2. Server must be globally enabled in the server_config table
///
/// Both conditions must be true for a server to be considered enabled.
#[derive(Debug, Clone)]
pub struct ServerEnabledService {
    pool: Pool<Sqlite>,
}

impl ServerEnabledService {
    /// Create a new ServerEnabledService instance
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Check if a server is enabled (canonical implementation)
    ///
    /// This is the single source of truth for server enabled status.
    /// All other components should use this method instead of implementing their own logic.
    ///
    /// # Arguments
    /// - `server_id`: Server ID to check
    ///
    /// # Returns
    /// - `Ok(true)`: Server is enabled (both in profile and globally)
    /// - `Ok(false)`: Server is disabled (either in profile or globally)
    /// - `Err(...)`: Database or other error
    pub async fn is_server_enabled(
        &self,
        server_id: &str,
    ) -> Result<bool> {
        // Check if server is enabled in any active profile
        let enabled_in_profile = self.is_server_enabled_in_any_active_profile(server_id).await?;

        if !enabled_in_profile {
            return Ok(false);
        }

        // Check global enabled status
        let globally_enabled = self.is_server_globally_enabled(server_id).await?;

        Ok(globally_enabled)
    }

    /// Get all enabled servers (canonical implementation)
    ///
    /// This method returns all servers that are both:
    /// 1. Enabled in at least one active profile
    /// 2. Globally enabled in the server_config table
    pub async fn get_all_enabled_servers(&self) -> Result<Vec<Server>> {
        // Get all servers first
        let all_servers = super::crud::get_all_servers(&self.pool).await?;

        if all_servers.is_empty() {
            return Ok(Vec::new());
        }

        // Get enabled server IDs from active profile
        let enabled_in_profile = self.get_server_ids_enabled_in_active_profile().await?;

        // Filter servers by both profile-level and global enabled status
        let enabled_servers: Vec<Server> = all_servers
            .into_iter()
            .filter(|server| {
                if let Some(id) = &server.id {
                    // Check both conditions: enabled in profile AND globally enabled
                    enabled_in_profile.contains(id) && server.enabled.as_bool()
                } else {
                    false // Server without ID is not enabled
                }
            })
            .collect();

        tracing::info!("Found {} enabled servers using unified service", enabled_servers.len());

        Ok(enabled_servers)
    }

    /// Get enabled servers from specific profile
    pub async fn get_enabled_servers_from_profile(
        &self,
        profile_ids: &[String],
    ) -> Result<Vec<Server>> {
        if profile_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Get all servers first
        let all_servers = super::crud::get_all_servers(&self.pool).await?;

        if all_servers.is_empty() {
            return Ok(Vec::new());
        }

        // Get enabled server IDs from specified profile
        let enabled_in_profile = self.get_server_ids_enabled_in_profile(profile_ids).await?;

        // Filter servers by both profile-level and global enabled status
        let enabled_servers: Vec<Server> = all_servers
            .into_iter()
            .filter(|server| {
                if let Some(id) = &server.id {
                    // Check both conditions: enabled in profile AND globally enabled
                    enabled_in_profile.contains(id) && server.enabled.as_bool()
                } else {
                    false // Server without ID is not enabled
                }
            })
            .collect();

        tracing::info!(
            "Found {} enabled servers from specified profile using unified service",
            enabled_servers.len()
        );

        Ok(enabled_servers)
    }

    /// Check if a server is enabled in any active profile
    pub async fn is_server_enabled_in_any_active_profile(
        &self,
        server_id: &str,
    ) -> Result<bool> {
        // Get all active profile
        let active_profile = crate::config::profile::get_active_profile(&self.pool).await?;

        // If no active profile, check default profile
        if active_profile.is_empty() {
            return self.is_server_enabled_in_default_profile(server_id).await;
        }

        // Check each active profile
        for profile in active_profile {
            if let Some(profile_id) = &profile.id {
                if self
                    .is_server_enabled_in_specific_profile(server_id, profile_id)
                    .await?
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if a server is enabled in a specific profile
    async fn is_server_enabled_in_specific_profile(
        &self,
        server_id: &str,
        profile_id: &str,
    ) -> Result<bool> {
        let server_configs = crate::config::profile::get_profile_servers(&self.pool, profile_id).await?;

        for server_config in server_configs {
            if server_config.server_id == server_id {
                return Ok(server_config.enabled);
            }
        }

        Ok(false) // Server not in this profile
    }

    /// Check if a server is enabled in the default profile
    async fn is_server_enabled_in_default_profile(
        &self,
        server_id: &str,
    ) -> Result<bool> {
        let default_profiles = crate::config::profile::get_default_profiles(&self.pool).await?;
        for profile in default_profiles {
            if !profile.is_active {
                continue;
            }
            if let Some(profile_id) = profile.id.as_ref() {
                if self
                    .is_server_enabled_in_specific_profile(server_id, profile_id)
                    .await?
                {
                    return Ok(true);
                }
            }
        }

        if let Some(profile_id) = self.try_get_legacy_default_profile().await? {
            return self.is_server_enabled_in_specific_profile(server_id, &profile_id).await;
        }

        Ok(false)
    }

    /// Check if a server is globally enabled
    async fn is_server_globally_enabled(
        &self,
        server_id: &str,
    ) -> Result<bool> {
        let server = get_server_by_id(&self.pool, server_id).await?;

        if let Some(server) = server {
            Ok(server.enabled.as_bool())
        } else {
            Ok(false) // Server not found
        }
    }

    /// Get all server IDs that are enabled in active profile
    async fn get_server_ids_enabled_in_active_profile(&self) -> Result<HashSet<String>> {
        let active_profile = crate::config::profile::get_active_profile(&self.pool).await?;

        if active_profile.is_empty() {
            return self.get_server_ids_enabled_in_default_profile().await;
        }

        let mut enabled_server_ids = HashSet::new();

        for profile in active_profile {
            if let Some(profile_id) = &profile.id {
                let profile_enabled_ids = self.get_server_ids_enabled_in_specific_profile(profile_id).await?;
                enabled_server_ids.extend(profile_enabled_ids);
            }
        }

        Ok(enabled_server_ids)
    }

    /// Get server IDs enabled in specific profile
    async fn get_server_ids_enabled_in_profile(
        &self,
        profile_ids: &[String],
    ) -> Result<HashSet<String>> {
        let mut enabled_server_ids = HashSet::new();

        for profile_id in profile_ids {
            let profile_enabled_ids = self.get_server_ids_enabled_in_specific_profile(profile_id).await?;
            enabled_server_ids.extend(profile_enabled_ids);
        }

        Ok(enabled_server_ids)
    }

    /// Get server IDs enabled in a specific profile
    async fn get_server_ids_enabled_in_specific_profile(
        &self,
        profile_id: &str,
    ) -> Result<HashSet<String>> {
        let server_configs = crate::config::profile::get_profile_servers(&self.pool, profile_id).await?;

        let enabled_server_ids: HashSet<String> = server_configs
            .into_iter()
            .filter(|config| config.enabled)
            .map(|config| config.server_id)
            .collect();

        Ok(enabled_server_ids)
    }

    /// Get server IDs enabled in the default profile
    async fn get_server_ids_enabled_in_default_profile(&self) -> Result<HashSet<String>> {
        let mut enabled = HashSet::new();

        let default_profiles = crate::config::profile::get_default_profiles(&self.pool).await?;
        for profile in default_profiles {
            if !profile.is_active {
                continue;
            }
            if let Some(profile_id) = profile.id.as_ref() {
                let ids = self.get_server_ids_enabled_in_specific_profile(profile_id).await?;
                enabled.extend(ids);
            }
        }

        if enabled.is_empty() {
            if let Some(profile_id) = self.try_get_legacy_default_profile().await? {
                let ids = self.get_server_ids_enabled_in_specific_profile(&profile_id).await?;
                enabled.extend(ids);
            }
        }

        Ok(enabled)
    }

    /// Helper method to try getting the legacy default profile ID
    async fn try_get_legacy_default_profile(&self) -> Result<Option<String>> {
        let legacy_default = crate::config::profile::get_profile_by_name(&self.pool, USER_PROFILE_INITIAL_NAME).await?;
        Ok(legacy_default.and_then(|profile| profile.id))
    }
}

// ============================================================================
// BACKWARD-COMPATIBLE PUBLIC FUNCTIONS
// ============================================================================

/// Get all enabled servers from the database based on profile
///
/// DEPRECATED: Use ServerEnabledService::get_all_enabled_servers() instead
pub async fn get_enabled_servers(pool: &Pool<Sqlite>) -> Result<Vec<Server>> {
    let service = ServerEnabledService::new(pool.clone());
    service.get_all_enabled_servers().await
}

/// Get enabled servers from specific profile
///
/// DEPRECATED: Use ServerEnabledService::get_enabled_servers_from_profile() instead
pub async fn get_enabled_servers_by_profile(
    pool: &Pool<Sqlite>,
    profile_ids: &[String],
) -> Result<Vec<Server>> {
    let service = ServerEnabledService::new(pool.clone());
    service.get_enabled_servers_from_profile(profile_ids).await
}

/// Check if a server is enabled in any active profile
///
/// DEPRECATED: Use ServerEnabledService::is_server_enabled() instead
pub async fn is_server_enabled_in_any_profile(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<bool> {
    let service = ServerEnabledService::new(pool.clone());
    service.is_server_enabled(server_id).await
}

/// Check if a server is enabled in any active profile (ignoring global status)
pub async fn is_server_enabled_in_any_active_profile(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<bool> {
    let service = ServerEnabledService::new(pool.clone());
    service.is_server_enabled_in_any_active_profile(server_id).await
}

// ============================================================================
// UTILITY FUNCTIONS (unique functionality not in ServerEnabledService)
// ============================================================================

/// Check if a server is in a specific profile
///
/// This function checks if a server is in a specific profile, regardless of enabled status.
/// Returns true if the server is in the profile, false otherwise.
pub async fn is_server_in_profile(
    pool: &Pool<Sqlite>,
    server_id: &str,
    profile_id: &str,
) -> Result<bool> {
    // Get all server configs in this profile
    let server_configs = crate::config::profile::get_profile_servers(pool, profile_id).await?;

    // Check if the server is in this profile
    for server_config in server_configs {
        if server_config.server_id == server_id {
            return Ok(true);
        }
    }

    // Server is not in this profile
    Ok(false)
}

/// Update a server's global enabled status
///
/// This function updates the global enabled status of a server in the database.
/// Returns true if the server was updated, false if the server was not found.
/// If the status is updated, it also publishes a ServerGlobalStatusChanged event.
pub async fn update_server_global_status(
    pool: &Pool<Sqlite>,
    server_id: &str,
    enabled: bool,
) -> Result<bool> {
    tracing::debug!(
        "Updating global enabled status for server ID {} to {}",
        server_id,
        enabled
    );

    let result = sqlx::query(
        r#"
        UPDATE server_config
        SET enabled = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        "#,
    )
    .bind(enabled)
    .bind(server_id)
    .execute(pool)
    .await
    .context("Failed to update server global status")?;

    let updated = result.rows_affected() > 0;

    // If the server was updated, publish an event
    if updated {
        // Get the server name
        if let Ok(Some(server)) = get_server_by_id(pool, server_id).await {
            // Publish the event
            crate::core::events::EventBus::global().publish(crate::core::events::Event::ServerGlobalStatusChanged {
                server_id: server_id.to_string(),
                server_name: server.name,
                enabled,
            });

            tracing::info!(
                "Published ServerGlobalStatusChanged event for server ID {} ({})",
                server_id,
                enabled
            );
        }
    }

    Ok(updated)
}

/// Get a server's global enabled status
///
/// This function retrieves the global enabled status of a server from the database.
/// Returns Some(bool) if the server was found, None if the server was not found.
pub async fn get_server_global_status(
    pool: &Pool<Sqlite>,
    server_id: &str,
) -> Result<Option<bool>> {
    tracing::debug!("Getting global enabled status for server ID {}", server_id);

    let enabled = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT enabled FROM server_config
        WHERE id = ?
        "#,
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await
    .context("Failed to get server global status")?;

    Ok(enabled)
}
