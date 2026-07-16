//! Profile service core implementation
//!
//! Provides business logic operations for profile, including
//! configuration merging, tool checking and other functions

use crate::config::database::Database;
use crate::core::foundation::error::CoreResult;
use crate::core::profile::merge::ProfileMerger;
use crate::core::profile::types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Profile service interface
///
/// Responsible for core business logic of profile, including:
/// - Configuration merging algorithms
/// - Tool enablement checking
/// - Server configuration aggregation
#[derive(Debug, Clone)]
pub struct ProfileService {
    /// Configuration merger
    merger: ProfileMerger,
    /// Cached merge result
    cached_merge_result: Arc<RwLock<Option<ProfileMergeResult>>>,
}

impl ProfileService {
    /// Create a new ProfileService instance
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            merger: ProfileMerger::new(db),
            cached_merge_result: Arc::new(RwLock::new(None)),
        }
    }

    /// Get merged server configuration list
    ///
    /// # Returns
    /// - `Ok(Vec<MergedServerConfig>)`: List of merged server configurations
    /// - `Err(CoreError)`: Error during merging process
    pub async fn get_merged_servers(&self) -> CoreResult<Vec<MergedServerConfig>> {
        let merge_result = self.get_or_create_merge_result().await?;
        Ok(merge_result.servers)
    }

    /// Get merged tool configurations for a specific server
    ///
    /// # Arguments
    /// - `server_id`: Server ID
    ///
    /// # Returns
    /// - `Ok(Vec<MergedToolConfig>)`: List of tool configurations for this server
    /// - `Err(CoreError)`: Error during query process
    pub async fn get_merged_tools_for_server(
        &self,
        server_id: &str,
    ) -> CoreResult<Vec<MergedToolConfig>> {
        self.merger.merge_tools_for_server(server_id).await
    }

    /// Check if a specific tool is enabled in profile
    ///
    /// # Arguments
    /// - `server_name`: Server name
    /// - `tool_name`: Tool name
    ///
    /// # Returns
    /// - `Ok(bool)`: Whether the tool is enabled
    /// - `Err(CoreError)`: Error during checking process
    pub async fn is_tool_enabled(
        &self,
        server_name: &str,
        tool_name: &str,
    ) -> CoreResult<bool> {
        let merge_result = self.get_or_create_merge_result().await?;

        // Find matching tool configuration
        if let Some(tool_config) = merge_result.tools.iter().find(|t| t.tool_name == tool_name) {
            // Check if this server is in the enabled servers list
            Ok(tool_config.enabled && tool_config.server_ids.contains(&server_name.to_string()))
        } else {
            // Tool not defined in any profile, default to disabled
            Ok(false)
        }
    }

    /// Check if a tool is enabled for a specific server
    ///
    /// # Arguments
    /// - `server_id`: Server ID
    /// - `tool_name`: Tool name
    ///
    /// # Returns
    /// - `Ok(bool)`: Whether the tool is enabled
    /// - `Err(CoreError)`: Error during checking process
    pub async fn is_tool_enabled_for_server(
        &self,
        server_id: &str,
        tool_name: &str,
    ) -> CoreResult<bool> {
        self.merger.is_tool_enabled(server_id, tool_name).await
    }

    /// Merge configurations from all active profile
    ///
    /// # Returns
    /// - `Ok(ProfileMergeResult)`: Complete merge result
    /// - `Err(CoreError)`: Error during merging process
    pub async fn merge_all_configs(&self) -> CoreResult<ProfileMergeResult> {
        self.get_or_create_merge_result().await
    }

    /// Invalidate cache and force configuration re-merging
    pub async fn invalidate_cache(&self) {
        let mut cache = self.cached_merge_result.write().await;
        *cache = None;
    }

    /// Get or create merge result
    async fn get_or_create_merge_result(&self) -> CoreResult<ProfileMergeResult> {
        // First try to get from cache
        {
            let cache = self.cached_merge_result.read().await;
            if let Some(ref result) = *cache {
                return Ok(result.clone());
            }
        }

        // Cache miss, perform merge operation
        let merge_result = self.merger.merge_all_configs().await?;

        // Update cache
        {
            let mut cache = self.cached_merge_result.write().await;
            *cache = Some(merge_result.clone());
        }

        Ok(merge_result)
    }

    // -------------------- Visibility helpers --------------------
    pub async fn allowed_tool_unique_set(&self) -> Option<std::collections::HashSet<String>> {
        if let Ok(m) = self.get_or_create_merge_result().await {
            return m.allowed_tool_set();
        }
        None
    }

    pub async fn allowed_resource_unique_set(&self) -> Option<std::collections::HashSet<String>> {
        if let Ok(m) = self.get_or_create_merge_result().await {
            return m.allowed_resource_set();
        }
        None
    }

    pub async fn allowed_prompt_unique_set(&self) -> Option<std::collections::HashSet<String>> {
        if let Ok(m) = self.get_or_create_merge_result().await {
            return m.allowed_prompt_set();
        }
        None
    }
}
