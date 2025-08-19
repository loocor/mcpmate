//! Database synchronization functionality for connection pool
//!
//! Handles synchronization of MCP capabilities (tools, resources, prompts) from
//! connected servers to the database. This module provides a unified approach
//! to syncing different types of capabilities across configuration suites.

use anyhow::{Context, Result as AnyhowResult};
use rmcp::model::Tool;
use std::sync::Arc;
use tracing;

use super::UpstreamConnectionPool;

// Simplified approach - extract common database operations

impl UpstreamConnectionPool {
    /// Common helper to get server and suits for sync operations
    /// This reduces duplication in the three sync methods
    async fn get_server_and_suits(
        db: &Arc<crate::config::database::Database>,
        server_name: &str,
    ) -> AnyhowResult<(String, Vec<(String, String)>)> {
        // Get the server ID
        let server = crate::config::server::get_server(&db.pool, server_name)
            .await
            .context(format!("Failed to get server '{server_name}'"))?;

        if let Some(server) = server {
            if let Some(server_id) = &server.id {
                // Get all config suits that have this server enabled
                let suits_with_server = Self::get_suits_with_server(&db.pool, server_id).await?;

                // Collect suit data for concurrent processing
                let suit_data: Vec<(String, String)> = suits_with_server
                    .into_iter()
                    .filter_map(|suit| suit.id.map(|suit_id| (suit_id, suit.name)))
                    .collect();

                return Ok((server_id.clone(), suit_data));
            }
        }

        Err(anyhow::anyhow!("Server '{}' not found", server_name))
    }

    /// Helper function to get config suits that have a specific server enabled
    async fn get_suits_with_server(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        server_id: &str,
    ) -> AnyhowResult<Vec<crate::config::models::ConfigSuit>> {
        // Get all config suits that have this server enabled
        let all_suits = crate::config::suit::get_all_config_suits(pool)
            .await
            .context("Failed to get all config suits")?;

        let mut suits_with_server = Vec::new();

        for suit in all_suits {
            if let Some(suit_id) = &suit.id {
                // Get all servers in this suit
                let suit_servers = crate::config::suit::get_config_suit_servers(pool, suit_id)
                    .await
                    .context(format!("Failed to get servers for suit '{suit_id}'"))?;

                // Check if this server is in the suit
                for suit_server in suit_servers {
                    if suit_server.server_id == *server_id {
                        suits_with_server.push(suit.clone());
                        break;
                    }
                }
            }
        }

        Ok(suits_with_server)
    }

    // Generic sync method removed - using specific implementations for each type

    /// Helper function to fetch prompts from service
    async fn fetch_prompts_from_service(
        service: &rmcp::service::Peer<rmcp::service::RoleClient>,
        server_name: &str,
        instance_id: &str,
    ) -> AnyhowResult<Vec<rmcp::model::Prompt>> {
        use anyhow::Context;

        // Collect all prompts from the server with pagination
        let mut all_prompts = Vec::new();
        let mut cursor = None;

        loop {
            let result = service
                .list_prompts(Some(rmcp::model::PaginatedRequestParam { cursor }))
                .await
                .context(format!(
                    "Failed to list prompts from upstream server '{}' instance '{}'",
                    server_name, instance_id
                ))?;

            all_prompts.extend(result.prompts);

            cursor = result.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(all_prompts)
    }

    /// Helper function to fetch resources from service
    async fn fetch_resources_from_service(
        service: &rmcp::service::Peer<rmcp::service::RoleClient>,
        server_name: &str,
        instance_id: &str,
    ) -> Vec<String> {
        match service.list_all_resources().await {
            Ok(resources) => resources.into_iter().map(|r| r.uri.clone()).collect::<Vec<String>>(),
            Err(e) => {
                tracing::error!(
                    "Failed to list resources from server '{}' (instance: {}): {}",
                    server_name,
                    instance_id,
                    e
                );
                Vec::new()
            }
        }
    }

    /// Sync tools to database
    ///
    /// This function syncs tools from a server to the database.
    /// It adds tools to all config suits that have the server enabled.
    pub(super) async fn sync_tools_to_database(
        db: &Arc<crate::config::database::Database>,
        server_name: &str,
        tools: &[Tool],
    ) -> AnyhowResult<()> {
        tracing::debug!(
            "Syncing {} tools from server '{}' to database",
            tools.len(),
            server_name
        );

        // Use common helper to get server and suits
        let (server_id, suit_data) = Self::get_server_and_suits(db, server_name).await?;

        // Sync tools to each suit concurrently
        let sync_futures: Vec<_> =
            suit_data
                .into_iter()
                .map(|(suit_id, suit_name)| {
                    let pool = db.pool.clone();
                    let server_id = server_id.clone();
                    let server_name = server_name.to_string();
                    let tools = tools.to_vec();

                    async move {
                        Self::sync_tools_to_suit(&pool, &suit_id, &server_id, &server_name, &tools, &suit_name).await
                    }
                })
                .collect();

        futures::future::try_join_all(sync_futures).await?;
        tracing::debug!(
            "Successfully synced {} tools from server '{}'",
            tools.len(),
            server_name
        );
        Ok(())
    }

    /// Helper function to sync tools to a specific suit
    async fn sync_tools_to_suit(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        suit_id: &str,
        server_id: &str,
        server_name: &str,
        tools: &[Tool],
        suit_name: &str,
    ) -> AnyhowResult<()> {
        // Get existing tools in this suit for this server
        let existing_tools = crate::config::suit::get_config_suit_tools(pool, suit_id)
            .await
            .context(format!("Failed to get tools for suit '{suit_id}'"))?;

        let existing_tool_names: std::collections::HashSet<String> = existing_tools
            .iter()
            .filter(|t| t.server_id == *server_id)
            .map(|t| t.tool_name.clone())
            .collect();

        // Add new tools to the suit
        for tool in tools {
            let tool_name = tool.name.to_string();

            // Skip if tool already exists in this suit
            if existing_tool_names.contains(&tool_name) {
                continue;
            }

            // Add the tool to the suit (enabled by default)
            match crate::config::suit::add_tool_to_config_suit(pool, suit_id, server_id, &tool_name, true).await {
                Ok(_) => {
                    tracing::debug!(
                        "Added tool '{}' from server '{}' to suit '{}'",
                        tool_name,
                        server_name,
                        suit_name
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to add tool '{}' from server '{}' to suit '{}': {}",
                        tool_name,
                        server_name,
                        suit_name,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    // Duplicate method removed - keeping only the first one

    /// Sync resources to database with service
    ///
    /// This function syncs resources from a server to the database.
    /// It adds resources to all config suits that have the server enabled.
    pub(super) async fn sync_resources_to_database_with_service(
        db: &Arc<crate::config::database::Database>,
        server_name: &str,
        instance_id: &str,
        service: &rmcp::service::Peer<rmcp::service::RoleClient>,
    ) -> AnyhowResult<()> {
        // Fetch resources from the service
        let server_resources = Self::fetch_resources_from_service(service, server_name, instance_id).await;

        tracing::debug!(
            "Syncing {} resources from server '{}' (instance: {}) to database",
            server_resources.len(),
            server_name,
            instance_id
        );

        // Use common helper to get server and suits
        let (server_id, suit_data) = Self::get_server_and_suits(db, server_name).await?;

        tracing::debug!(
            "Found {} config suits with server '{}' enabled",
            suit_data.len(),
            server_name
        );

        // Create concurrent futures for all suits
        let sync_futures: Vec<_> = suit_data
            .into_iter()
            .map(|(suit_id, suit_name)| {
                let pool = db.pool.clone();
                let server_id = server_id.clone();
                let server_name = server_name.to_string();
                let server_resources = server_resources.clone();

                async move {
                    Self::sync_resources_to_suit(
                        &pool,
                        &suit_id,
                        &server_id,
                        &server_name,
                        &server_resources,
                        &suit_name,
                    )
                    .await
                }
            })
            .collect();

        // Execute all sync operations concurrently
        futures::future::try_join_all(sync_futures).await?;

        tracing::debug!(
            "Successfully synced {} resources from server '{}' (instance: {}) to database",
            server_resources.len(),
            server_name,
            instance_id
        );

        Ok(())
    }

    /// Helper function to sync resources to a specific suit
    async fn sync_resources_to_suit(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        suit_id: &str,
        server_id: &str,
        server_name: &str,
        server_resources: &[String],
        suit_name: &str,
    ) -> AnyhowResult<()> {
        // Get existing resources in this suit for this server
        let existing_resources = crate::config::suit::get_resources_for_config_suit(pool, suit_id)
            .await
            .context(format!("Failed to get resources for suit '{suit_id}'"))?;

        let existing_resource_uris: std::collections::HashSet<String> = existing_resources
            .iter()
            .filter(|r| r.server_id == *server_id)
            .map(|r| r.resource_uri.clone())
            .collect();

        // Add new resources to the suit
        for resource_uri in server_resources {
            // Skip if resource already exists in this suit
            if existing_resource_uris.contains(resource_uri) {
                continue;
            }

            // Add the resource to the suit (enabled by default)
            match crate::config::suit::add_resource_to_config_suit(pool, suit_id, server_id, resource_uri, true).await {
                Ok(_) => {
                    tracing::debug!(
                        "Added resource '{}' from server '{}' to suit '{}'",
                        resource_uri,
                        server_name,
                        suit_name
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to add resource '{}' from server '{}' to suit '{}': {}",
                        resource_uri,
                        server_name,
                        suit_name,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Sync prompts to database with service
    ///
    /// This function syncs prompts from a server to the database.
    /// It adds prompts to all config suits that have the server enabled.
    pub(super) async fn sync_prompts_to_database_with_service(
        db: &Arc<crate::config::database::Database>,
        server_name: &str,
        instance_id: &str,
        service: &rmcp::service::Peer<rmcp::service::RoleClient>,
    ) -> AnyhowResult<()> {
        // Fetch prompts from the service
        let all_prompts = Self::fetch_prompts_from_service(service, server_name, instance_id).await?;

        tracing::debug!(
            "Syncing {} prompts from server '{}' (instance: {}) to database",
            all_prompts.len(),
            server_name,
            instance_id
        );

        // Use common helper to get server and suits
        let (server_id, suit_data) = Self::get_server_and_suits(db, server_name).await?;

        tracing::debug!(
            "Found {} config suits with server '{}' enabled",
            suit_data.len(),
            server_name
        );

        // Create concurrent futures for all suits
        let sync_futures: Vec<_> = suit_data
            .into_iter()
            .map(|(suit_id, suit_name)| {
                let pool = db.pool.clone();
                let server_id = server_id.clone();
                let server_name = server_name.to_string();
                let all_prompts = all_prompts.clone();

                async move {
                    Self::sync_prompts_to_suit(&pool, &suit_id, &server_id, &server_name, &all_prompts, &suit_name)
                        .await
                }
            })
            .collect();

        // Execute all sync operations concurrently
        futures::future::try_join_all(sync_futures).await?;

        tracing::debug!(
            "Successfully synced {} prompts from server '{}' (instance: {}) to database",
            all_prompts.len(),
            server_name,
            instance_id
        );

        Ok(())
    }

    /// Helper function to sync prompts to a specific suit
    async fn sync_prompts_to_suit(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        suit_id: &str,
        server_id: &str,
        server_name: &str,
        all_prompts: &[rmcp::model::Prompt],
        suit_name: &str,
    ) -> AnyhowResult<()> {
        // Get existing prompts in this suit for this server
        let existing_prompts = crate::config::suit::get_prompts_for_config_suit(pool, suit_id)
            .await
            .context(format!("Failed to get prompts for suit '{suit_id}'"))?;

        let existing_prompt_names: std::collections::HashSet<String> = existing_prompts
            .iter()
            .filter(|p| p.server_id == *server_id)
            .map(|p| p.prompt_name.clone())
            .collect();

        // Add new prompts to the suit
        for prompt in all_prompts {
            let prompt_name = prompt.name.to_string();

            // Skip if prompt already exists in this suit
            if existing_prompt_names.contains(&prompt_name) {
                continue;
            }

            // Add the prompt to the suit (enabled by default)
            match crate::config::suit::add_prompt_to_config_suit(pool, suit_id, server_id, &prompt_name, true).await {
                Ok(_) => {
                    tracing::debug!(
                        "Added prompt '{}' from server '{}' to suit '{}'",
                        prompt_name,
                        server_name,
                        suit_name
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to add prompt '{}' from server '{}' to suit '{}': {}",
                        prompt_name,
                        server_name,
                        suit_name,
                        e
                    );
                }
            }
        }

        Ok(())
    }
}
