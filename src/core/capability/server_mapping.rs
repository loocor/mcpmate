//! Server ID to Server Name mapping management
//!
//! This module provides a centralized mapping between server_id and server_name
//! to resolve inconsistencies in the codebase where some parts use server_id
//! and others use server_name as keys.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing;

use crate::config::database::Database;

/// Server mapping information
#[derive(Debug, Clone)]
pub struct ServerInfo {
    /// Server ID (authoritative key in connection pool)
    pub server_id: String,
    /// Server name (display name and legacy key)
    pub server_name: String,
}

/// In-memory server mapping manager
///
/// This manager maintains bidirectional mappings between server_id and server_name
/// to resolve inconsistencies where different parts of the system use different
/// identifiers for the same server.
#[derive(Debug)]
pub struct ServerMappingManager {
    /// Map from server_id to ServerInfo
    id_to_info: Arc<RwLock<HashMap<String, ServerInfo>>>,
    /// Map from server_name to server_id for reverse lookups
    name_to_id: Arc<RwLock<HashMap<String, String>>>,
}

impl ServerMappingManager {
    /// Create a new server mapping manager
    pub fn new() -> Self {
        Self {
            id_to_info: Arc::new(RwLock::new(HashMap::new())),
            name_to_id: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Refresh mappings from database
    ///
    /// This function loads all server configurations from the database and
    /// updates the in-memory mappings.
    pub async fn refresh_from_database(
        &self,
        database: &Database,
    ) -> anyhow::Result<()> {
        tracing::debug!("Refreshing server mappings from database");

        // Query all servers from database
        let query = r#"
            SELECT id, name
            FROM server_config
            WHERE id IS NOT NULL AND name IS NOT NULL
        "#;

        let rows = sqlx::query_as::<_, (String, String)>(query)
            .fetch_all(&database.pool)
            .await?;

        let mut id_to_info = self.id_to_info.write().await;
        let mut name_to_id = self.name_to_id.write().await;

        // Clear existing mappings
        id_to_info.clear();
        name_to_id.clear();

        // Build new mappings
        for (server_id, server_name) in rows {
            let server_info = ServerInfo {
                server_id: server_id.clone(),
                server_name: server_name.clone(),
            };

            id_to_info.insert(server_id.clone(), server_info);
            name_to_id.insert(server_name, server_id);
        }

        tracing::info!("Refreshed {} server mappings from database", id_to_info.len());
        Ok(())
    }

    /// Get server info by server_id
    pub async fn get_by_id(
        &self,
        server_id: &str,
    ) -> Option<ServerInfo> {
        let id_to_info = self.id_to_info.read().await;
        id_to_info.get(server_id).cloned()
    }

    /// Get server_id by server_name
    pub async fn get_id_by_name(
        &self,
        server_name: &str,
    ) -> Option<String> {
        let name_to_id = self.name_to_id.read().await;
        name_to_id.get(server_name).cloned()
    }

    /// Get server_name by server_id
    pub async fn get_name_by_id(
        &self,
        server_id: &str,
    ) -> Option<String> {
        let id_to_info = self.id_to_info.read().await;
        id_to_info.get(server_id).map(|info| info.server_name.clone())
    }

    /// Add or update a server mapping
    pub async fn upsert_mapping(
        &self,
        server_id: String,
        server_name: String,
    ) {
        let server_info = ServerInfo {
            server_id: server_id.clone(),
            server_name: server_name.clone(),
        };

        let mut id_to_info = self.id_to_info.write().await;
        let mut name_to_id = self.name_to_id.write().await;

        id_to_info.insert(server_id.clone(), server_info);
        name_to_id.insert(server_name, server_id);
    }

    /// Remove a server mapping by server_id
    pub async fn remove_by_id(
        &self,
        server_id: &str,
    ) {
        let mut id_to_info = self.id_to_info.write().await;
        let mut name_to_id = self.name_to_id.write().await;

        if let Some(server_info) = id_to_info.remove(server_id) {
            name_to_id.remove(&server_info.server_name);
        }
    }

    /// Get all server mappings
    pub async fn get_all(&self) -> Vec<ServerInfo> {
        let id_to_info = self.id_to_info.read().await;
        id_to_info.values().cloned().collect()
    }

    /// Get mapping statistics
    pub async fn get_stats(&self) -> (usize, usize) {
        let id_to_info = self.id_to_info.read().await;
        let name_to_id = self.name_to_id.read().await;
        (id_to_info.len(), name_to_id.len())
    }
}

impl Default for ServerMappingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global server mapping manager instance
static SERVER_MAPPING_MANAGER: once_cell::sync::OnceCell<ServerMappingManager> = once_cell::sync::OnceCell::new();

/// Get the global server mapping manager
pub fn global_server_mapping_manager() -> &'static ServerMappingManager {
    SERVER_MAPPING_MANAGER.get_or_init(ServerMappingManager::new)
}

/// Initialize the global server mapping manager with database data
pub async fn initialize_server_mapping_manager(database: &Database) -> anyhow::Result<()> {
    let manager = global_server_mapping_manager();
    manager.refresh_from_database(database).await?;
    Ok(())
}
