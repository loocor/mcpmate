//! Database synchronization functionality for connection pool
//!
//! Handles synchronization of MCP capabilities (tools, resources, prompts) from
//! connected servers to the database. This module provides a unified approach
//! to syncing different types of capabilities across profile.

use anyhow::{Context, Result as AnyhowResult};
use rmcp::model::Tool;
use std::sync::Arc;
use tracing;

use super::UpstreamConnectionPool;
use crate::common::sync::SyncHelper;

/// Capability sync selection flags (bitmask style without external deps)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CapSyncFlags(pub u32);

impl CapSyncFlags {
    pub const NONE: Self = Self(0);
    pub const TOOLS: Self = Self(1 << 0);
    pub const RESOURCES: Self = Self(1 << 1);
    pub const PROMPTS: Self = Self(1 << 2);
    pub const RESOURCE_TEMPLATES: Self = Self(1 << 3);

    #[inline]
    pub fn contains(
        self,
        other: Self,
    ) -> bool {
        (self.0 & other.0) != 0
    }

    /// Convenience presets
    pub const ALL: Self = Self(Self::TOOLS.0 | Self::RESOURCES.0 | Self::PROMPTS.0 | Self::RESOURCE_TEMPLATES.0);
}

// Simplified approach - extract common database operations

impl UpstreamConnectionPool {
    /// Common helper to get server and profile for sync operations
    /// Now uses the unified SyncHelper framework
    async fn get_server_and_profile(
        db: &Arc<crate::config::database::Database>,
        server_id: &str,
    ) -> AnyhowResult<(String, Vec<(String, String)>)> {
        // Use the unified sync framework
        let sync_context = SyncHelper::get_server_context(&db.pool, server_id).await?;

        // Convert to the format expected by existing code
        let profile_data: Vec<(String, String)> = sync_context
            .profile_ids
            .into_iter()
            .map(|profile_id| {
                // Get profile name from metadata or use ID as fallback
                let profile_name = sync_context
                    .metadata
                    .get(&format!("profile_name_{}", profile_id))
                    .cloned()
                    .unwrap_or_else(|| profile_id.clone());
                (profile_id, profile_name)
            })
            .collect();

        Ok((sync_context.server_id, profile_data))
    }

    // Note: get_profile_with_server function removed as it's now handled by SyncHelper::get_server_context
    /// Fetch tools from service (with pagination when available)
    async fn fetch_tools_from_service(
        service: &rmcp::service::Peer<rmcp::service::RoleClient>,
        server_name: &str,
        instance_id: &str,
    ) -> AnyhowResult<Vec<Tool>> {
        use anyhow::Context;

        // Try paginated requests if supported; fallback to single call
        let mut all_tools = Vec::new();
        let mut cursor = None;

        loop {
            match service
                .list_tools(Some(rmcp::model::PaginatedRequestParam { cursor }))
                .await
            {
                Ok(result) => {
                    all_tools.extend(result.tools);
                    cursor = result.next_cursor;
                    if cursor.is_none() {
                        break;
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(e)).with_context(|| {
                        format!(
                            "Failed to list tools from upstream server '{}' instance '{}'",
                            server_name, instance_id
                        )
                    });
                }
            }
        }

        Ok(all_tools)
    }

    /// Unified capability sync: tools/resources/prompts (with resource templates as resources sub-item)
    ///
    /// This is the main entry point that consolidates database writes for all capabilities.
    /// Extracts common patterns: server/profile resolution, concurrent sync, logging, and error handling.
    pub(crate) async fn sync_capabilities(
        db: &Arc<crate::config::database::Database>,
        server_id: &str,
        instance_id: &str,
        service: &rmcp::service::Peer<rmcp::service::RoleClient>,
        flags: CapSyncFlags,
        tools_opt: Option<&[Tool]>,
    ) -> AnyhowResult<()> {
        // Common setup: resolve server_name for logging
        let server_name = crate::config::operations::utils::get_server_name(&db.pool, server_id)
            .await
            .unwrap_or_else(|_| server_id.to_string());

        // Common setup: get server and profile data once
        let (resolved_server_id, profile_data) = Self::get_server_and_profile(db, server_id).await?;

        tracing::debug!(
            "Syncing capabilities (flags: {:?}) from server '{}' (ID: {}, instance: {}) to {} profiles",
            flags,
            server_name,
            server_id,
            instance_id,
            profile_data.len()
        );

        // TOOLS - unified pattern
        if flags.contains(CapSyncFlags::TOOLS) {
            let tools: Vec<Tool> = if let Some(slice) = tools_opt {
                slice.to_vec()
            } else {
                Self::fetch_tools_from_service(service, &server_name, instance_id).await?
            };

            if !tools.is_empty() {
                Self::sync_tools_to_database_internal(db, &resolved_server_id, &server_name, &tools, &profile_data)
                    .await?;
            } else {
                tracing::debug!(
                    "No tools fetched for server '{}' (ID: {}), skipping tools sync",
                    server_name,
                    server_id
                );
            }
        }

        // RESOURCES - unified pattern
        if flags.contains(CapSyncFlags::RESOURCES) {
            let resources = Self::fetch_resources_from_service(service, &server_name, instance_id).await?;

            if !resources.is_empty() {
                Self::sync_resources_to_database_internal(
                    db,
                    &resolved_server_id,
                    &server_name,
                    &resources,
                    &profile_data,
                )
                .await?;
            } else {
                tracing::debug!(
                    "No resources fetched for server '{}' (ID: {}), skipping resources sync",
                    server_name,
                    server_id
                );
            }
        }

        // PROMPTS - unified pattern
        if flags.contains(CapSyncFlags::PROMPTS) {
            let prompts = Self::fetch_prompts_from_service(service, &server_name, instance_id).await?;

            if !prompts.is_empty() {
                Self::sync_prompts_to_database_internal(db, &resolved_server_id, &server_name, &prompts, &profile_data)
                    .await?;
            } else {
                tracing::debug!(
                    "No prompts fetched for server '{}' (ID: {}), skipping prompts sync",
                    server_name,
                    server_id
                );
            }
        }

        // RESOURCE_TEMPLATES - treat as resources sub-item (per MCP spec)
        if flags.contains(CapSyncFlags::RESOURCE_TEMPLATES) {
            tracing::debug!(
                "Resource templates sync integrated into resources for server '{}' (ID: {})",
                server_name,
                server_id
            );
            // Note: Resource templates are handled as part of resources according to MCP specification
            // If separate handling is needed for UI compatibility, implement here
        }

        tracing::debug!(
            "Successfully synced capabilities (flags: {:?}) from server '{}' (ID: {})",
            flags,
            server_name,
            server_id
        );

        Ok(())
    }

    // Removed unused compatibility wrapper: sync_capabilities_to_database_with_service

    /// Internal method to sync tools to database using unified pattern
    async fn sync_tools_to_database_internal(
        db: &Arc<crate::config::database::Database>,
        server_id: &str,
        server_name: &str,
        tools: &[Tool],
        profile_data: &[(String, String)],
    ) -> AnyhowResult<()> {
        let sync_items: Vec<_> = profile_data
            .iter()
            .map(|(profile_id, profile_name)| {
                (
                    profile_id.clone(),
                    profile_name.clone(),
                    db.pool.clone(),
                    server_id.to_string(),
                    server_name.to_string(),
                    tools.to_vec(),
                )
            })
            .collect();

        let _sync_result = SyncHelper::execute_concurrent_sync(
            sync_items,
            "tools_to_profile",
            4, // max concurrent operations
            |(profile_id, profile_name, pool, server_id, server_name, tools)| async move {
                Self::sync_tools_to_profile(&pool, &profile_id, &server_id, &server_name, &tools, &profile_name).await
            },
        )
        .await;

        Ok(())
    }

    /// Internal method to sync resources to database using unified pattern
    async fn sync_resources_to_database_internal(
        db: &Arc<crate::config::database::Database>,
        server_id: &str,
        server_name: &str,
        resources: &[String],
        profile_data: &[(String, String)],
    ) -> AnyhowResult<()> {
        let sync_items: Vec<_> = profile_data
            .iter()
            .map(|(profile_id, profile_name)| {
                (
                    profile_id.clone(),
                    profile_name.clone(),
                    db.pool.clone(),
                    server_id.to_string(),
                    server_name.to_string(),
                    resources.to_vec(),
                )
            })
            .collect();

        let _sync_result = SyncHelper::execute_concurrent_sync(
            sync_items,
            "resources_to_profile",
            4, // max concurrent operations
            |(profile_id, profile_name, pool, server_id, server_name, resources)| async move {
                Self::sync_resources_to_profile(&pool, &profile_id, &server_id, &server_name, &resources, &profile_name)
                    .await
            },
        )
        .await;

        Ok(())
    }

    /// Internal method to sync prompts to database using unified pattern
    async fn sync_prompts_to_database_internal(
        db: &Arc<crate::config::database::Database>,
        server_id: &str,
        server_name: &str,
        prompts: &[rmcp::model::Prompt],
        profile_data: &[(String, String)],
    ) -> AnyhowResult<()> {
        let sync_items: Vec<_> = profile_data
            .iter()
            .map(|(profile_id, profile_name)| {
                (
                    profile_id.clone(),
                    profile_name.clone(),
                    db.pool.clone(),
                    server_id.to_string(),
                    server_name.to_string(),
                    prompts.to_vec(),
                )
            })
            .collect();

        let _sync_result = SyncHelper::execute_concurrent_sync(
            sync_items,
            "prompts_to_profile",
            4, // max concurrent operations
            |(profile_id, profile_name, pool, server_id, server_name, prompts)| async move {
                Self::sync_prompts_to_profile(&pool, &profile_id, &server_id, &server_name, &prompts, &profile_name)
                    .await
            },
        )
        .await;

        Ok(())
    }

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
    ) -> AnyhowResult<Vec<String>> {
        use anyhow::Context;

        let resources = service.list_all_resources().await.with_context(|| {
            format!(
                "Failed to list resources from upstream server '{}' instance '{}'",
                server_name, instance_id
            )
        })?;

        Ok(resources.into_iter().map(|r| r.uri.clone()).collect::<Vec<String>>())
    }

    // Removed unused public adapter: sync_tools_to_database (use sync_capabilities instead)

    /// Helper function to sync tools to a specific profile
    async fn sync_tools_to_profile(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        profile_id: &str,
        server_id: &str,
        server_name: &str,
        tools: &[Tool],
        profile_name: &str,
    ) -> AnyhowResult<()> {
        // Get existing tools in this profile for this server
        let existing_tools = crate::config::profile::get_profile_tools(pool, profile_id)
            .await
            .context(format!("Failed to get tools for profile '{profile_id}'"))?;

        let existing_tool_names: std::collections::HashSet<String> = existing_tools
            .iter()
            .filter(|t| t.server_id == *server_id)
            .map(|t| t.tool_name.clone())
            .collect();

        // Add new tools to the profile
        for tool in tools {
            let tool_name = tool.name.to_string();

            // Skip if tool already exists in this profile
            if existing_tool_names.contains(&tool_name) {
                continue;
            }

            // Add the tool to the profile (enabled by default)
            match crate::config::profile::add_tool_to_profile(pool, profile_id, server_id, &tool_name, true).await {
                Ok(_) => {
                    tracing::debug!(
                        "Added tool '{}' from server '{}' to profile '{}'",
                        tool_name,
                        server_name,
                        profile_name
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to add tool '{}' from server '{}' to profile '{}': {}",
                        tool_name,
                        server_name,
                        profile_name,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    // Duplicate method removed - keeping only the first one

    // Removed unused public adapter: sync_resources_to_database_with_service (use sync_capabilities instead)

    /// Helper function to sync resources to a specific profile
    async fn sync_resources_to_profile(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        profile_id: &str,
        server_id: &str,
        server_name: &str,
        server_resources: &[String],
        profile_name: &str,
    ) -> AnyhowResult<()> {
        // Get existing resources in this profile for this server
        let existing_resources = crate::config::profile::get_resources_for_profile(pool, profile_id)
            .await
            .context(format!("Failed to get resources for profile '{profile_id}'"))?;

        let existing_resource_uris: std::collections::HashSet<String> = existing_resources
            .iter()
            .filter(|r| r.server_id == *server_id)
            .map(|r| r.resource_uri.clone())
            .collect();

        // Add new resources to the profile
        for resource_uri in server_resources {
            // Skip if resource already exists in this profile
            if existing_resource_uris.contains(resource_uri) {
                continue;
            }

            // Add the resource to the profile (enabled by default)
            match crate::config::profile::add_resource_to_profile(pool, profile_id, server_id, resource_uri, true).await
            {
                Ok(_) => {
                    tracing::debug!(
                        "Added resource '{}' from server '{}' to profile '{}'",
                        resource_uri,
                        server_name,
                        profile_name
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to add resource '{}' from server '{}' to profile '{}': {}",
                        resource_uri,
                        server_name,
                        profile_name,
                        e
                    );
                }
            }
        }

        Ok(())
    }

    // Removed unused public adapter: sync_prompts_to_database_with_service (use sync_capabilities instead)

    /// Helper function to sync prompts to a specific profile
    async fn sync_prompts_to_profile(
        pool: &sqlx::Pool<sqlx::Sqlite>,
        profile_id: &str,
        server_id: &str,
        server_name: &str,
        all_prompts: &[rmcp::model::Prompt],
        profile_name: &str,
    ) -> AnyhowResult<()> {
        // Get existing prompts in this profile for this server
        let existing_prompts = crate::config::profile::get_prompts_for_profile(pool, profile_id)
            .await
            .context(format!("Failed to get prompts for profile '{profile_id}'"))?;

        let existing_prompt_names: std::collections::HashSet<String> = existing_prompts
            .iter()
            .filter(|p| p.server_id == *server_id)
            .map(|p| p.prompt_name.clone())
            .collect();

        // Add new prompts to the profile
        for prompt in all_prompts {
            let prompt_name = prompt.name.to_string();

            // Skip if prompt already exists in this profile
            if existing_prompt_names.contains(&prompt_name) {
                continue;
            }

            // Add the prompt to the profile (enabled by default)
            match crate::config::profile::add_prompt_to_profile(pool, profile_id, server_id, &prompt_name, true).await {
                Ok(_) => {
                    tracing::debug!(
                        "Added prompt '{}' from server '{}' to profile '{}'",
                        prompt_name,
                        server_name,
                        profile_name
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to add prompt '{}' from server '{}' to profile '{}': {}",
                        prompt_name,
                        server_name,
                        profile_name,
                        e
                    );
                }
            }
        }

        Ok(())
    }
}
