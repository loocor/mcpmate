// Config Suit merge service
// Contains functions for merging and deduplicating configuration suits

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use anyhow::{Context, Result};
use tokio::sync::RwLock;
use tracing;

use crate::{
    api::routes::AppState,
    conf::{
        database::Database,
        models::{ConfigSuitTool, Server},
        operations::suit::get_active_config_suits,
    },
};

/// Configuration Suit Merge Service
///
/// This service is responsible for merging and deduplicating servers and tools
/// from multiple active configuration suits.
#[derive(Debug)]
pub struct ConfigSuitMergeService {
    /// Database reference
    pub db: Arc<Database>,
    /// Cache of merged servers (server_id -> Server)
    merged_servers: RwLock<HashMap<String, Server>>,
    /// Cache of merged tools (server_id -> (tool_id -> ConfigSuitTool))
    merged_tools: RwLock<HashMap<String, HashMap<String, ConfigSuitTool>>>,
    /// Last time the cache was updated
    last_updated: RwLock<Instant>,
}

impl ConfigSuitMergeService {
    /// Create a new ConfigSuitMergeService
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            merged_servers: RwLock::new(HashMap::new()),
            merged_tools: RwLock::new(HashMap::new()),
            last_updated: RwLock::new(Instant::now()),
        }
    }

    /// Start a background task to periodically update the cache
    pub fn start_background_update(service: Arc<Self>) {
        tokio::spawn(async move {
            let update_interval = std::time::Duration::from_secs(30); // Update every 30 seconds

            loop {
                // Sleep first to avoid immediate update after initialization
                tokio::time::sleep(update_interval).await;

                // Update the cache
                if let Err(e) = service.update_cache().await {
                    tracing::error!(
                        "Failed to update Config Suit merge cache in background task: {}",
                        e
                    );
                } else {
                    tracing::debug!(
                        "Config Suit merge cache updated successfully in background task"
                    );
                }
            }
        });

        tracing::info!("Started background task for Config Suit merge cache updates");
    }

    /// Update the cache of merged servers and tools
    ///
    /// This function fetches all active configuration suits, merges their servers and tools,
    /// and updates the cache.
    pub async fn update_cache(&self) -> Result<()> {
        tracing::debug!("Updating ConfigSuitMergeService cache");

        // Get all active configuration suits
        let active_suits = get_active_config_suits(&self.db.pool)
            .await
            .context("Failed to get active configuration suits")?;

        tracing::debug!("Found {} active configuration suits", active_suits.len());

        // Merge servers from all active suits
        let merged_servers = self.merge_servers(&active_suits).await?;

        // Merge tools for each server
        let mut merged_tools = HashMap::new();
        for server_id in merged_servers.keys() {
            let tools = self.merge_tools(&active_suits, server_id).await?;
            merged_tools.insert(server_id.clone(), tools);
        }

        // Update the cache
        {
            let mut servers_lock = self.merged_servers.write().await;
            *servers_lock = merged_servers;
        }

        {
            let mut tools_lock = self.merged_tools.write().await;
            *tools_lock = merged_tools;
        }

        {
            let mut time_lock = self.last_updated.write().await;
            *time_lock = Instant::now();
        }

        tracing::debug!("ConfigSuitMergeService cache updated successfully");
        Ok(())
    }

    /// Get all merged servers
    pub async fn get_merged_servers(&self) -> Result<Vec<Server>> {
        let servers_lock = self.merged_servers.read().await;
        let servers = servers_lock.values().cloned().collect();
        Ok(servers)
    }

    /// Get all merged tools for a specific server
    pub async fn get_merged_tools(
        &self,
        server_id: &str,
    ) -> Result<Vec<ConfigSuitTool>> {
        let tools_lock = self.merged_tools.read().await;

        if let Some(server_tools) = tools_lock.get(server_id) {
            let tools = server_tools.values().cloned().collect();
            Ok(tools)
        } else {
            Ok(Vec::new())
        }
    }

    /// Check if a tool is enabled
    pub async fn is_tool_enabled(
        &self,
        server_id: &str,
        tool_id: &str,
    ) -> Result<bool> {
        let tools_lock = self.merged_tools.read().await;

        if let Some(server_tools) = tools_lock.get(server_id) {
            if let Some(tool) = server_tools.get(tool_id) {
                return Ok(tool.enabled);
            }
        }

        // If the tool is not found, it's considered disabled
        Ok(false)
    }

    /// Synchronize server connections based on merged servers
    ///
    /// This function connects to servers that are enabled in the merged list
    /// and disconnects from servers that are not in the merged list.
    pub async fn sync_server_connections(
        &self,
        state: &Arc<AppState>,
    ) -> Result<()> {
        tracing::debug!("Synchronizing server connections");

        // Get merged servers
        let merged_servers = self.get_merged_servers().await?;
        let merged_server_ids: HashSet<String> =
            merged_servers.iter().filter_map(|s| s.id.clone()).collect();

        // Get connection pool
        let mut pool = state.connection_pool.lock().await;

        // Get all connected servers
        let mut connected_server_ids = HashSet::new();
        for (server_name, instances) in &pool.connections {
            // Get server ID
            if let Ok(Some(server)) =
                crate::conf::operations::get_server(&self.db.pool, server_name).await
            {
                if let Some(server_id) = server.id {
                    // Check if any instance is connected
                    for conn in instances.values() {
                        if conn.is_connected() {
                            connected_server_ids.insert(server_id.clone());
                            break;
                        }
                    }
                }
            }
        }

        // Servers to connect: in merged list but not connected
        let to_connect: Vec<&Server> = merged_servers
            .iter()
            .filter(|s| {
                if let Some(id) = &s.id {
                    !connected_server_ids.contains(id)
                } else {
                    false
                }
            })
            .collect();

        // Servers to disconnect: connected but not in merged list
        let to_disconnect: Vec<String> = connected_server_ids
            .iter()
            .filter(|id| !merged_server_ids.contains(*id))
            .cloned()
            .collect();

        // Connect to servers
        for server in to_connect {
            let name = &server.name;
            tracing::info!("Connecting to server '{}'", name);

            // Find the default instance
            if let Ok((instance_id, _)) = pool.get_default_instance(name) {
                // Connect to the server
                if let Err(e) = pool.connect(name, &instance_id).await {
                    tracing::error!("Failed to connect to server '{}': {}", name, e);
                }
            }
        }

        // Disconnect from servers
        for server_id in to_disconnect {
            // Get server name from ID
            if let Ok(Some(server)) =
                crate::conf::operations::get_server_by_id(&self.db.pool, &server_id).await
            {
                let name = &server.name;
                tracing::info!("Disconnecting from server '{}'", name);

                // Get instance IDs first to avoid borrowing issues
                let instance_ids: Vec<String> = if let Some(instances) = pool.connections.get(name)
                {
                    instances.keys().cloned().collect()
                } else {
                    Vec::new()
                };

                // Disconnect from each instance
                for instance_id in instance_ids {
                    // Disconnect from the server
                    if let Err(e) = pool.disconnect(name, &instance_id).await {
                        tracing::error!("Failed to disconnect from server '{}': {}", name, e);
                    }
                }
            }
        }

        tracing::debug!("Server connections synchronized successfully");
        Ok(())
    }

    /// Merge servers from all active configuration suits
    async fn merge_servers(
        &self,
        active_suits: &[crate::conf::models::ConfigSuit],
    ) -> Result<HashMap<String, Server>> {
        let mut merged_servers = HashMap::new();

        // Process each active suit
        for suit in active_suits {
            if let Some(suit_id) = &suit.id {
                // Get all servers in this suit
                let suit_servers =
                    crate::conf::operations::get_config_suit_servers(&self.db.pool, suit_id)
                        .await
                        .context(format!("Failed to get servers for suit '{suit_id}'"))?;

                // Process each server
                for server_config in suit_servers {
                    // Only include enabled servers
                    if server_config.enabled {
                        // Get server details
                        if let Ok(Some(server)) = crate::conf::operations::get_server_by_id(
                            &self.db.pool,
                            &server_config.server_id,
                        )
                        .await
                        {
                            // Add to merged servers, using server_id as the key
                            if let Some(server_id) = &server.id {
                                merged_servers.insert(server_id.clone(), server);
                            }
                        }
                    }
                }
            }
        }

        Ok(merged_servers)
    }

    /// Merge tools for a specific server from all active configuration suits
    async fn merge_tools(
        &self,
        active_suits: &[crate::conf::models::ConfigSuit],
        server_id: &str,
    ) -> Result<HashMap<String, ConfigSuitTool>> {
        let mut merged_tools = HashMap::new();

        // Process each active suit
        for suit in active_suits {
            if let Some(suit_id) = &suit.id {
                // Get all tools in this suit
                let suit_tools =
                    crate::conf::operations::tool::get_tools_by_suit_id(&self.db.pool, suit_id)
                        .await
                        .context(format!("Failed to get tools for suit '{suit_id}'"))?;

                // Process each tool
                for tool in suit_tools {
                    // Only include tools for the specified server
                    if tool.server_id == *server_id {
                        // Only include enabled tools
                        if tool.enabled {
                            // Add to merged tools, using tool_id as the key
                            if let Some(tool_id) = &tool.id {
                                merged_tools.insert(tool_id.clone(), tool);
                            }
                        }
                    }
                }
            }
        }

        Ok(merged_tools)
    }
}
