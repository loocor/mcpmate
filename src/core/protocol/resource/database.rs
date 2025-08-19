//!
//! This module provides a unified service for all resource-related operations
//! that are driven by database configuration suits.

use std::{collections::HashSet, sync::Arc};

use anyhow::{Context, Result};
use rmcp::model::{Resource, ResourceTemplate};
use tokio::sync::Mutex;
use tracing;

use crate::{config::database::Database, core::pool::UpstreamConnectionPool};

/// Database-driven resource service
#[derive(Clone)]
pub struct DatabaseResourceService {
    /// Database connection
    db: Arc<Database>,
    /// Connection pool for accessing upstream servers
    pub(crate) connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
}

impl DatabaseResourceService {
    /// Create a new database resource service
    pub fn new(
        db: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        Self { db, connection_pool }
    }

    /// Get all enabled resources from the database.
    ///
    /// This function retrieves all enabled resources from active configuration suits,
    /// ensuring that no duplicates are returned.
    pub async fn get_enabled_resources(&self) -> Result<Vec<Resource>> {
        tracing::debug!("Getting all enabled resources from database");

        let query = crate::config::suit::resource::build_enabled_resources_query(None);
        let enabled_resources_tuples = sqlx::query_as::<_, (String, String)>(&query)
            .fetch_all(&self.db.pool)
            .await
            .context("Failed to query enabled resources from database")?;

        let enabled_set: HashSet<(String, String)> = enabled_resources_tuples.into_iter().collect();

        let mut all_resources = Vec::new();
        let pool = self.connection_pool.lock().await;

        for (server_name, instances) in pool.connections.iter() {
            for conn in instances.values() {
                if conn.is_connected() && !conn.is_disabled() && conn.supports_resources() {
                    // Get resources from the service connection dynamically
                    if let Some(service) = &conn.service {
                        match service.list_resources(None).await {
                            Ok(result) => {
                                for resource in result.resources {
                                    if enabled_set.contains(&(server_name.clone(), resource.uri.clone())) {
                                        all_resources.push(resource);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to list resources from server '{}': {}", server_name, e);
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("Found {} enabled resources from database", all_resources.len());

        Ok(all_resources)
    }

    /// Get all enabled resource templates from enabled servers only
    ///
    /// This function retrieves all resource templates from enabled servers,
    /// ensuring that no duplicates are returned and only templates from
    /// globally enabled servers are included.
    pub async fn get_enabled_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        tracing::debug!("Getting all enabled resource templates from enabled servers");

        // Get enabled servers from database
        let enabled_servers = match crate::config::server::get_enabled_servers(&self.db.pool).await {
            Ok(servers) => servers,
            Err(e) => {
                tracing::error!("Failed to get enabled servers: {}", e);
                return Ok(Vec::new());
            }
        };

        let enabled_server_names: HashSet<String> = enabled_servers.into_iter().map(|server| server.name).collect();

        let mut all_templates = Vec::new();
        let pool = self.connection_pool.lock().await;

        // Only get templates from enabled servers
        for (server_name, instances) in pool.connections.iter() {
            if !enabled_server_names.contains(server_name) {
                continue; // Skip disabled servers
            }

            for conn in instances.values() {
                if !conn.is_connected() || !conn.supports_resources() {
                    continue;
                }

                if let Some(service) = &conn.service {
                    match service.list_resource_templates(None).await {
                        Ok(result) => {
                            all_templates.extend(result.resource_templates);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to list resource templates from server '{}': {}", server_name, e);
                        }
                    }
                }
            }
        }

        // Use the helper function to deduplicate by (server_name, template_name)
        let all_templates =
            crate::core::foundation::utils::deduplicate_by_key(all_templates, |template| template.name.clone());

        tracing::info!(
            "Found {} enabled resource templates from enabled servers",
            all_templates.len()
        );

        Ok(all_templates)
    }
}
