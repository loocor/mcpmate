//!
//! This module provides a unified service for all prompt-related operations
//! that are driven by database configuration suits.

use std::{collections::HashSet, sync::Arc};

use anyhow::{Context, Result};
use rmcp::model::Prompt;
use tokio::sync::Mutex;
use tracing;

use crate::{config::database::Database, core::pool::UpstreamConnectionPool};

/// Database-driven prompt service
#[derive(Clone)]
pub struct DatabasePromptService {
    /// Database connection
    db: Arc<Database>,
    /// Connection pool for accessing upstream servers
    pub(crate) connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
}

impl DatabasePromptService {
    /// Create a new database prompt service
    pub fn new(
        db: Arc<Database>,
        connection_pool: Arc<Mutex<UpstreamConnectionPool>>,
    ) -> Self {
        Self { db, connection_pool }
    }

    /// Get all enabled prompts from the database.
    ///
    /// This function retrieves all enabled prompts from active configuration suits,
    /// ensuring that no duplicates are returned.
    pub async fn get_enabled_prompts(&self) -> Result<Vec<Prompt>> {
        tracing::debug!("Getting all enabled prompts from database");

        let query = crate::config::suit::prompt::build_enabled_prompts_query(None);
        let enabled_prompts_tuples = sqlx::query_as::<_, (String, String)>(&query)
            .fetch_all(&self.db.pool)
            .await
            .context("Failed to query enabled prompts from database")?;

        let enabled_set: HashSet<(String, String)> = enabled_prompts_tuples.into_iter().collect();

        let mut all_prompts = Vec::new();
        let pool = self.connection_pool.lock().await;

        for (server_name, instances) in pool.connections.iter() {
            for conn in instances.values() {
                if conn.is_connected() && !conn.is_disabled() && conn.supports_prompts() {
                    // Get prompts from the service connection dynamically
                    if let Some(service) = &conn.service {
                        match service.list_prompts(None).await {
                            Ok(result) => {
                                for prompt in result.prompts {
                                    if enabled_set.contains(&(server_name.clone(), prompt.name.clone())) {
                                        all_prompts.push(prompt);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to list prompts from server '{}': {}", server_name, e);
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("Found {} enabled prompts from database", all_prompts.len());

        Ok(all_prompts)
    }
}
