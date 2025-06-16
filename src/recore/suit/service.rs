//! Suit service core implementation
//!
//! Provides business logic operations for configuration suits, including
//! configuration merging, tool checking and other functions

use crate::config::database::Database;
use crate::recore::foundation::error::RecoreResult;
use crate::recore::suit::merge::SuitMerger;
use crate::recore::suit::types::*;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Suit service interface
///
/// Responsible for core business logic of configuration suits, including:
/// - Configuration merging algorithms
/// - Tool enablement checking
/// - Server configuration aggregation
#[derive(Debug, Clone)]
pub struct SuitService {
    /// Configuration merger
    merger: SuitMerger,
    /// Cached merge result
    cached_merge_result: Arc<RwLock<Option<SuitMergeResult>>>,
}

impl SuitService {
    /// Create a new SuitService instance
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            merger: SuitMerger::new(db),
            cached_merge_result: Arc::new(RwLock::new(None)),
        }
    }

    /// Get merged server configuration list
    ///
    /// # Returns
    /// - `Ok(Vec<MergedServerConfig>)`: List of merged server configurations
    /// - `Err(RecoreError)`: Error during merging process
    pub async fn get_merged_servers(&self) -> RecoreResult<Vec<MergedServerConfig>> {
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
    /// - `Err(RecoreError)`: Error during query process
    pub async fn get_merged_tools_for_server(
        &self,
        server_id: &str,
    ) -> RecoreResult<Vec<MergedToolConfig>> {
        self.merger.merge_tools_for_server(server_id).await
    }

    /// Check if a specific tool is enabled in configuration suits
    ///
    /// # Arguments
    /// - `tool_name`: Tool name
    ///
    /// # Returns
    /// - `Ok(ToolEnabledResult)`: Tool enablement check result
    /// - `Err(RecoreError)`: Error during checking process
    pub async fn is_tool_enabled(
        &self,
        tool_name: &str,
    ) -> RecoreResult<ToolEnabledResult> {
        let merge_result = self.get_or_create_merge_result().await?;

        // Find matching tool configuration
        if let Some(tool_config) = merge_result.tools.iter().find(|t| t.tool_name == tool_name) {
            Ok(ToolEnabledResult {
                tool_name: tool_name.to_string(),
                enabled: tool_config.enabled,
                enabled_servers: tool_config.server_ids.clone(),
                related_suits: tool_config.source_suits.clone(),
            })
        } else {
            // Tool not defined in any configuration suit, default to disabled
            Ok(ToolEnabledResult {
                tool_name: tool_name.to_string(),
                enabled: false,
                enabled_servers: vec![],
                related_suits: vec![],
            })
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
    /// - `Err(RecoreError)`: Error during checking process
    pub async fn is_tool_enabled_for_server(
        &self,
        server_id: &str,
        tool_name: &str,
    ) -> RecoreResult<bool> {
        self.merger.is_tool_enabled(server_id, tool_name).await
    }

    /// Merge configurations from all active configuration suits
    ///
    /// # Returns
    /// - `Ok(SuitMergeResult)`: Complete merge result
    /// - `Err(RecoreError)`: Error during merging process
    pub async fn merge_all_configs(&self) -> RecoreResult<SuitMergeResult> {
        self.get_or_create_merge_result().await
    }

    /// Invalidate cache and force configuration re-merging
    pub async fn invalidate_cache(&self) {
        let mut cache = self.cached_merge_result.write().await;
        *cache = None;
    }

    /// Get or create merge result
    async fn get_or_create_merge_result(&self) -> RecoreResult<SuitMergeResult> {
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
}
