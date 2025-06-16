//! Suit service core implementation
//!
//! Provides business logic operations for configuration suits, including
//! configuration merging, tool checking and other functions

use crate::config::database::Database;
use crate::recore::foundation::error::{RecoreError, RecoreResult};
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
    /// - `server_name`: Server name
    /// - `tool_name`: Tool name
    ///
    /// # Returns
    /// - `Ok(bool)`: Whether the tool is enabled
    /// - `Err(RecoreError)`: Error during checking process
    pub async fn is_tool_enabled(
        &self,
        server_name: &str,
        tool_name: &str,
    ) -> RecoreResult<bool> {
        let merge_result = self.get_or_create_merge_result().await?;

        // Find matching tool configuration
        if let Some(tool_config) = merge_result.tools.iter().find(|t| t.tool_name == tool_name) {
            // Check if this server is in the enabled servers list
            Ok(tool_config.enabled && tool_config.server_ids.contains(&server_name.to_string()))
        } else {
            // Tool not defined in any configuration suit, default to disabled
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

    /// Resolve a tool name to server and original tool name
    ///
    /// This function resolves a unique tool name to the server name and original tool name
    /// using the configuration suits.
    ///
    /// # Arguments
    /// * `tool_name` - The tool name to resolve
    ///
    /// # Returns
    /// * `Result<(String, String)>` - (server_name, original_tool_name)
    pub async fn resolve_tool_name(
        &self,
        tool_name: &str,
    ) -> RecoreResult<(String, String)> {
        // Implement tool name resolution using configuration suits
        tracing::debug!("Resolving tool name '{}'", tool_name);

        let merge_result = self.get_or_create_merge_result().await?;

        // Find the tool in the merged configuration
        for tool_config in &merge_result.tools {
            if tool_config.tool_name == tool_name && tool_config.enabled {
                // Get the first enabled server for this tool
                if let Some(server_id) = tool_config.server_ids.first() {
                    // Find the server name from the merged servers
                    for server_config in &merge_result.servers {
                        if server_config.server_id == *server_id {
                            return Ok((server_config.name.clone(), tool_name.to_string()));
                        }
                    }
                }
            }
        }

        // Tool not found in any enabled configuration
        Err(RecoreError::generic_error(
            &format!(
                "Tool '{}' not found in any enabled configuration suits",
                tool_name
            ),
            None,
        ))
    }
}
