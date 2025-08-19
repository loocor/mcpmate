//! Database-driven tool service
//!
//! This module provides a unified service for all tool-related operations
//! that are driven by database configuration suits. It replaces the dual
//! mapping system with a single, authoritative database-driven approach.

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use rmcp::model::Tool;
use tokio::sync::Mutex;
use tracing;

use super::types::ToolMapping;
use crate::{config::database::Database, core::pool::UpstreamConnectionPool};

/// Database-driven tool service
///
/// This service provides all tool-related operations using the database
/// as the single source of truth for tool mappings and configurations.
/// It replaces the previous dual mapping system (runtime + database).
#[derive(Clone)]
pub struct DatabaseToolService {
    /// Database connection
    db: Arc<Database>,
    /// Connection pool for accessing upstream servers
    pub(crate) connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
}

impl DatabaseToolService {
    /// Create a new database tool service
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `connection_pool` - Connection pool for upstream servers
    ///
    /// # Returns
    /// * `DatabaseToolService` - New service instance
    pub fn new(
        db: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        Self {
            db,
            connection_pool,
        }
    }

    /// Get all enabled tools from database with standardized names
    ///
    /// This function retrieves all enabled tools from active configuration suits
    /// and applies the pre-stored unique names from the database. This is the
    /// authoritative method for getting tools - no fallback to runtime calculation.
    ///
    /// Features fault isolation to prevent single server failures from affecting the entire system.
    ///
    /// # Returns
    /// * `Result<Vec<Tool>>` - A list of all enabled tools with standardized names
    pub async fn get_enabled_tools(&self) -> Result<Vec<Tool>> {
        tracing::debug!("Getting all enabled tools from database (authoritative method)");

        // Query enabled tools from active configuration suits (new architecture)
        let query = format!(
            "{} ORDER BY st.unique_name",
            crate::config::suit::tool::build_enabled_tools_query(None)
        );
        let enabled_tools = sqlx::query_as::<_, (String, String, String, String)>(&query)
            .fetch_all(&self.db.pool)
            .await
            .context("Failed to query enabled tools from database")?;

        let mut all_tools = Vec::new();

        // Use timeout to prevent indefinite blocking on connection pool lock
        let pool_result = tokio::time::timeout(
            std::time::Duration::from_millis(500), // 500ms timeout for connection pool access
            self.connection_pool.lock(),
        )
        .await;

        let pool = match pool_result {
            Ok(pool) => pool,
            Err(_) => {
                tracing::error!("Timeout waiting for connection pool lock in get_enabled_tools");
                return Err(anyhow::anyhow!("Connection pool access timeout"));
            }
        };

        // Build tool list for each enabled tool with fault isolation
        for (unique_name, server_name, tool_name, _server_id) in &enabled_tools {
            // Find the server instance in the connection pool
            if let Some(instances) = pool.connections.get(server_name) {
                // Find a connected instance for this server
                let mut found = false;
                for conn in instances.values() {
                    // Skip disabled servers completely (they should not appear in tool lists)
                    if conn.is_disabled() {
                        tracing::debug!(
                            "Skipping tool '{}' from disabled server '{}'",
                            tool_name,
                            server_name
                        );
                        continue;
                    }

                    // Skip failed or disconnected instances immediately
                    if !conn.is_connected() {
                        continue;
                    }

                    // Additional check: skip servers with permanent errors
                    if let crate::core::foundation::types::ConnectionStatus::Error(
                        ref error_details,
                    ) = conn.status
                    {
                        if error_details.error_type
                            == crate::core::foundation::types::ErrorType::Permanent
                        {
                            tracing::debug!(
                                "Skipping tool '{}' from server '{}' due to permanent error: {}",
                                tool_name,
                                server_name,
                                error_details.message
                            );
                            continue;
                        }
                    }

                    // Find the tool in this instance
                    if let Some(tool) = conn.tools.iter().find(|t| t.name == *tool_name) {
                        // Create a modified tool with the unique name from database
                        let mut unique_tool = tool.clone();
                        unique_tool.name = unique_name.clone().into();

                        all_tools.push(unique_tool);
                        found = true;
                        break; // Found the tool, move to next
                    }
                }

                if !found {
                    tracing::debug!(
                        "Tool '{}' from server '{}' is enabled in database but not available (server may be disconnected)",
                        tool_name,
                        server_name
                    );
                }
            } else {
                tracing::debug!(
                    "Server '{}' for tool '{}' not found in connection pool",
                    server_name,
                    tool_name
                );
            }
        }

        let tools_count = all_tools.len();
        let enabled_tools_count = enabled_tools.len();

        tracing::info!(
            "Found {} enabled tools from database (authoritative method, fault-isolated)",
            tools_count
        );

        // Add diagnostic logging when no tools are found
        if all_tools.is_empty() {
            tracing::warn!(
                "Tool database returned 0 enabled tools - Enabled tools in config: {}",
                enabled_tools_count
            );

            // Log connection states for each server
            for (unique_name, server_name, _tool_name, _server_id) in &enabled_tools {
                if let Some(instances) = pool.connections.get(server_name) {
                    for (inst_id, conn) in instances {
                        tracing::warn!(
                            "Tool '{}' from server '{}' instance '{}' - Status: {:?}, Connected: {}, Disabled: {}",
                            unique_name,
                            server_name,
                            inst_id,
                            conn.status,
                            conn.is_connected(),
                            conn.is_disabled()
                        );
                    }
                } else {
                    tracing::warn!(
                        "Tool '{}' from server '{}' - No instances in connection pool",
                        unique_name,
                        server_name
                    );
                }
            }
        }

        Ok(all_tools)
    }

    /// Resolve tool name using database mapping (authoritative method)
    ///
    /// This function resolves a standardized tool name (e.g., "everything_add")
    /// to the original server name and tool name using the database mapping.
    /// This is the only authoritative method for tool resolution.
    ///
    /// # Arguments
    /// * `tool_name` - The standardized tool name to resolve
    ///
    /// # Returns
    /// * `Result<(String, String)>` - Tuple of (server_name, original_tool_name)
    pub async fn resolve_tool(
        &self,
        tool_name: &str,
    ) -> Result<(String, String)> {
        tracing::debug!(
            "Resolving tool '{}' using database mapping (authoritative method)",
            tool_name
        );

        // Query the database for the tool mapping (new architecture)
        let query = format!(
            "{} AND st.unique_name = ? LIMIT 1",
            crate::config::suit::tool::build_enabled_tools_query(None)
        );
        let result = sqlx::query_as::<_, (String, String, String, String)>(&query)
            .bind(tool_name)
            .fetch_optional(&self.db.pool)
            .await
            .context("Failed to query tool mapping from database")?;

        match result {
            Some((_unique_name, server_name, original_tool_name, _server_id)) => {
                tracing::debug!(
                    "Resolved tool '{}' -> server: '{}', original tool: '{}'",
                    tool_name,
                    server_name,
                    original_tool_name
                );
                Ok((server_name, original_tool_name))
            }
            None => Err(anyhow::anyhow!(
                "Tool '{}' not found in active configuration suits or is disabled",
                tool_name
            )),
        }
    }

    /// Build tool mapping from database (authoritative method)
    ///
    /// This function builds a complete mapping of tool names to server/instance
    /// information using only the database as the source of truth.
    ///
    /// # Returns
    /// * `Result<HashMap<String, ToolMapping>>` - A mapping of unique tool names to server/instance information
    pub async fn build_tool_mapping(&self) -> Result<HashMap<String, ToolMapping>> {
        tracing::debug!("Building tool mapping from database (authoritative method)");

        // Query enabled tools from active configuration suits (new architecture)
        let query = crate::config::suit::tool::build_enabled_tools_query(None);
        let enabled_tools = sqlx::query_as::<_, (String, String, String, String)>(&query)
            .fetch_all(&self.db.pool)
            .await
            .context("Failed to query enabled tools from database")?;

        let mut tool_mapping = HashMap::new();
        let pool = self.connection_pool.lock().await;

        // Build mapping for each enabled tool
        for (unique_name, server_name, tool_name, _server_id) in enabled_tools {
            // Find the server instance in the connection pool
            if let Some(instances) = pool.connections.get(&server_name) {
                // Find a connected instance for this server
                let mut found = false;
                #[allow(clippy::for_kv_map)] // We need both instance_id and conn
                for (instance_id, conn) in instances {
                    // Skip disabled servers completely
                    if conn.is_disabled() {
                        tracing::debug!(
                            "Skipping tool mapping for '{}' from disabled server '{}'",
                            tool_name,
                            server_name
                        );
                        continue;
                    }

                    if !conn.is_connected() {
                        continue;
                    }

                    // Find the tool in this instance
                    if let Some(tool) = conn.tools.iter().find(|t| t.name == *tool_name) {
                        // Create a modified tool with the unique name from database
                        let mut unique_tool = tool.clone();
                        unique_tool.name = unique_name.clone().into();

                        // Add to mapping
                        tool_mapping.insert(
                            unique_name.clone(),
                            ToolMapping {
                                server_name: server_name.clone(),
                                instance_id: instance_id.clone(),
                                tool: unique_tool,
                                upstream_tool_name: tool_name.clone(),
                            },
                        );

                        tracing::debug!(
                            "Added enabled tool '{}' -> '{}' from server '{}'",
                            tool_name,
                            unique_name,
                            server_name
                        );
                        found = true;
                        break; // Found the tool, move to next
                    }
                }

                if !found {
                    tracing::warn!(
                        "Tool '{}' from server '{}' is enabled in database but not found in connection pool",
                        tool_name,
                        server_name
                    );
                }
            } else {
                tracing::warn!(
                    "Server '{}' for tool '{}' not found in connection pool",
                    server_name,
                    tool_name
                );
            }
        }

        tracing::info!(
            "Built tool mapping from database with {} enabled tools (authoritative method)",
            tool_mapping.len()
        );
        Ok(tool_mapping)
    }

    /// Check if a tool is enabled in the database
    ///
    /// # Arguments
    /// * `tool_name` - The standardized tool name to check
    ///
    /// # Returns
    /// * `Result<bool>` - True if the tool is enabled, false otherwise
    pub async fn is_tool_enabled(
        &self,
        tool_name: &str,
    ) -> Result<bool> {
        let query = format!(
            "SELECT COUNT(*) FROM ({}) AS enabled_tools WHERE unique_name = ?",
            crate::config::suit::tool::build_enabled_tools_query(None)
        );
        let count = sqlx::query_scalar::<_, i64>(&query)
            .bind(tool_name)
            .fetch_one(&self.db.pool)
            .await
            .context("Failed to check tool enablement in database")?;

        Ok(count > 0)
    }
}
