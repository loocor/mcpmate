//! Configuration suit merging algorithm implementation
//!
//! Merges server and tool configurations from multiple active configuration suits

use crate::config::database::Database;
use crate::recore::foundation::error::{RecoreError, RecoreResult};
use crate::recore::suit::types::*;
use std::sync::Arc;

/// Configuration suit merger
///
/// Responsible for merging server and tool configurations from multiple active configuration suits
#[derive(Debug, Clone)]
pub struct SuitMerger {
    /// Database reference
    db: Arc<Database>,
}

impl SuitMerger {
    /// Create a new SuitMerger instance
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Execute complete configuration merging operation
    ///
    /// # Returns
    /// - `Ok(SuitMergeResult)`: Merge result
    /// - `Err(RecoreError)`: Error during merging process
    pub async fn merge_all_configs(&self) -> RecoreResult<SuitMergeResult> {
        // Get all active configuration suits
        let active_suits = crate::config::suit::get_active_config_suits(&self.db.pool)
            .await
            .map_err(|e| {
                RecoreError::generic_error(
                    &format!("Failed to get active config suits: {}", e),
                    Some(e.into()),
                )
            })?;

        tracing::debug!("Found {} active configuration suits", active_suits.len());

        // Merge servers from all active suits
        let merged_servers = self.merge_servers(&active_suits).await?;

        // Merge tools from all active suits
        let merged_tools = self.merge_all_tools(&active_suits).await?;

        // Build merge result
        let suit_ids: Vec<String> = active_suits.iter().filter_map(|s| s.id.clone()).collect();

        Ok(SuitMergeResult {
            servers: merged_servers,
            tools: merged_tools,
            merged_suits: suit_ids,
            merged_at: chrono::Utc::now(),
        })
    }

    /// Get merged tool configurations for a specific server
    ///
    /// # Arguments
    /// - `server_id`: Server ID
    ///
    /// # Returns
    /// - `Ok(Vec<MergedToolConfig>)`: List of tool configurations for this server
    /// - `Err(RecoreError)`: Error during merging process
    pub async fn merge_tools_for_server(
        &self,
        server_id: &str,
    ) -> RecoreResult<Vec<MergedToolConfig>> {
        // Get all active configuration suits
        let active_suits = crate::config::suit::get_active_config_suits(&self.db.pool)
            .await
            .map_err(|e| {
                RecoreError::generic_error(
                    &format!("Failed to get active config suits: {}", e),
                    Some(e.into()),
                )
            })?;

        // Merge tools for the specific server
        self.merge_tools_for_specific_server(&active_suits, server_id)
            .await
    }

    /// Check if a specific tool is enabled
    ///
    /// # Arguments
    /// - `server_id`: Server ID
    /// - `tool_name`: Tool name
    ///
    /// # Returns
    /// - `Ok(bool)`: Whether the tool is enabled
    /// - `Err(RecoreError)`: Error during checking process
    pub async fn is_tool_enabled(
        &self,
        server_id: &str,
        tool_name: &str,
    ) -> RecoreResult<bool> {
        // Get all active configuration suits
        let active_suits = crate::config::suit::get_active_config_suits(&self.db.pool)
            .await
            .map_err(|e| {
                RecoreError::generic_error(
                    &format!("Failed to get active config suits: {}", e),
                    Some(e.into()),
                )
            })?;

        // Check if the tool is enabled in any of the active suits for this server
        for suit in &active_suits {
            if let Some(suit_id) = &suit.id {
                if let Ok(servers) =
                    crate::config::suit::get_config_suit_servers(&self.db.pool, suit_id).await
                {
                    // Check if this suit contains the specified server
                    let has_server = servers
                        .iter()
                        .any(|s| s.server_id == server_id && s.enabled);

                    if has_server {
                        if let Ok(tools) =
                            crate::config::suit::get_config_suit_tools(&self.db.pool, suit_id).await
                        {
                            for tool in tools {
                                if tool.tool_name == tool_name
                                    && tool.enabled
                                    && tool.server_id == server_id
                                {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Merge servers from all active configuration suits
    async fn merge_servers(
        &self,
        active_suits: &[crate::config::models::ConfigSuit],
    ) -> RecoreResult<Vec<MergedServerConfig>> {
        use std::collections::HashMap;

        let mut server_map: HashMap<String, MergedServerConfig> = HashMap::new();

        for suit in active_suits {
            if let Some(suit_id) = &suit.id {
                if let Ok(suit_servers) =
                    crate::config::suit::get_config_suit_servers(&self.db.pool, suit_id).await
                {
                    for suit_server in suit_servers {
                        if !suit_server.enabled {
                            continue; // Skip disabled servers
                        }

                        // Get actual server details from server_config table
                        let server_details = match sqlx::query_as::<_, (String, String)>(
                            "SELECT name, address FROM server_config WHERE id = ?",
                        )
                        .bind(&suit_server.server_id)
                        .fetch_optional(&self.db.pool)
                        .await
                        {
                            Ok(Some((name, address))) => (name, address),
                            Ok(None) => {
                                tracing::warn!(
                                    "Server {} not found in server_config",
                                    suit_server.server_id
                                );
                                continue;
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to get server details for {}: {}",
                                    suit_server.server_id,
                                    e
                                );
                                continue;
                            }
                        };

                        let entry = server_map
                            .entry(suit_server.server_id.clone())
                            .or_insert_with(|| MergedServerConfig {
                                server_id: suit_server.server_id.clone(),
                                name: server_details.0.clone(),
                                address: server_details.1.clone(),
                                enabled_tools: vec![],
                                source_suits: vec![],
                            });

                        // Add this suit as a source
                        if !entry.source_suits.contains(suit_id) {
                            entry.source_suits.push(suit_id.clone());
                        }

                        // Get tools for this server in this suit
                        if let Ok(tools) =
                            crate::config::suit::get_config_suit_tools(&self.db.pool, suit_id).await
                        {
                            for tool in tools {
                                if tool.enabled
                                    && tool.server_id == suit_server.server_id
                                    && !entry.enabled_tools.contains(&tool.tool_name)
                                {
                                    entry.enabled_tools.push(tool.tool_name);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(server_map.into_values().collect())
    }

    /// Merge all tools from active configuration suits
    async fn merge_all_tools(
        &self,
        active_suits: &[crate::config::models::ConfigSuit],
    ) -> RecoreResult<Vec<MergedToolConfig>> {
        use std::collections::HashMap;

        let mut tool_map: HashMap<String, MergedToolConfig> = HashMap::new();

        for suit in active_suits {
            if let Some(suit_id) = &suit.id {
                if let Ok(tools) =
                    crate::config::suit::get_config_suit_tools(&self.db.pool, suit_id).await
                {
                    for tool in tools {
                        let entry = tool_map.entry(tool.tool_name.clone()).or_insert_with(|| {
                            MergedToolConfig {
                                tool_name: tool.tool_name.clone(),
                                enabled: false,
                                server_ids: vec![],
                                config: HashMap::new(), // Empty config for now
                                source_suits: vec![],
                            }
                        });

                        // Update enabled status (if any suit enables it, it's enabled)
                        if tool.enabled {
                            entry.enabled = true;
                        }

                        // Add server ID if not already present
                        if tool.enabled && !entry.server_ids.contains(&tool.server_id) {
                            entry.server_ids.push(tool.server_id.clone());
                        }

                        // Add this suit as a source
                        if !entry.source_suits.contains(suit_id) {
                            entry.source_suits.push(suit_id.clone());
                        }

                        // Note: ConfigSuitTool doesn't have a config field in the current model
                        // If configuration is needed, it would need to be added to the database schema
                    }
                }
            }
        }

        Ok(tool_map.into_values().collect())
    }

    /// Merge tools for a specific server
    async fn merge_tools_for_specific_server(
        &self,
        active_suits: &[crate::config::models::ConfigSuit],
        server_id: &str,
    ) -> RecoreResult<Vec<MergedToolConfig>> {
        use std::collections::HashMap;

        let mut tool_map: HashMap<String, MergedToolConfig> = HashMap::new();

        for suit in active_suits {
            if let Some(suit_id) = &suit.id {
                // Check if this suit contains the specified server and it's enabled
                if let Ok(servers) =
                    crate::config::suit::get_config_suit_servers(&self.db.pool, suit_id).await
                {
                    let has_enabled_server = servers
                        .iter()
                        .any(|s| s.server_id == server_id && s.enabled);

                    if has_enabled_server {
                        if let Ok(tools) =
                            crate::config::suit::get_config_suit_tools(&self.db.pool, suit_id).await
                        {
                            for tool in tools {
                                // Only include tools for the specified server
                                if tool.server_id != server_id {
                                    continue;
                                }

                                let entry =
                                    tool_map.entry(tool.tool_name.clone()).or_insert_with(|| {
                                        MergedToolConfig {
                                            tool_name: tool.tool_name.clone(),
                                            enabled: false,
                                            server_ids: vec![server_id.to_string()],
                                            config: HashMap::new(),
                                            source_suits: vec![],
                                        }
                                    });

                                // Update enabled status
                                if tool.enabled {
                                    entry.enabled = true;
                                }

                                // Add this suit as a source
                                if !entry.source_suits.contains(suit_id) {
                                    entry.source_suits.push(suit_id.clone());
                                }

                                // Note: ConfigSuitTool doesn't have a config field in the current model
                                // Configuration merging would need database schema updates
                            }
                        }
                    }
                }
            }
        }

        Ok(tool_map.into_values().collect())
    }
}
