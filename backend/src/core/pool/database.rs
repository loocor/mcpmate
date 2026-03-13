//! Database synchronization functionality for connection pool
//!
//! Handles synchronization of MCP capabilities (tools, resources, prompts) from
//! connected servers to the database. This module provides a unified approach
//! to syncing different types of capabilities across profile.

use anyhow::{Context, Result as AnyhowResult};
use chrono::Utc;
use rmcp::model::{PaginatedRequestParam, ResourceTemplate, Tool};
use std::sync::Arc;
use tracing;

use super::UpstreamConnectionPool;
use crate::common::sync::SyncHelper;
use crate::config::server::capabilities::store_dual_write;
use crate::core::cache::{
    CachedPromptInfo, CachedResourceInfo, CachedResourceTemplateInfo, CachedToolInfo, PromptArgument, RedbCacheManager,
};
use crate::core::capability::facade::is_method_not_supported;

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
    ) -> AnyhowResult<Option<(String, Vec<(String, String)>)>> {
        // Use the unified sync framework
        let Some(sync_context) = SyncHelper::get_server_context(&db.pool, server_id).await? else {
            tracing::warn!(
                "Skipping capability sync for server '{}' because configuration is missing",
                server_id
            );
            return Ok(None);
        };

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

        // NOTE:
        // Historically we aborted capability sync entirely when the server wasn't
        // enabled in any profiles. That prevented REDB/SQLite shadow snapshots from
        // being refreshed for unattached servers. We now keep proceeding even when
        // no profiles are bound, so the service-level snapshot is always current,
        // while profile_* seeding is conditionally executed only when profiles exist.
        if profile_data.is_empty() {
            tracing::debug!(
                "No profiles currently enable server '{}'; will persist snapshot (REDB/SQLite shadow) but skip profile seeding",
                server_id
            );
        }

        Ok(Some((sync_context.server_id, profile_data)))
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
        // Mark REDB as refreshing for this server to inform consumers
        if let Ok(cache_manager) = RedbCacheManager::global() {
            let _ = cache_manager
                .set_refreshing(server_id, std::time::Duration::from_secs(60))
                .await;
        }
        // Common setup: resolve server_name for logging
        let server_name = crate::config::operations::utils::get_server_name(&db.pool, server_id)
            .await
            .unwrap_or_else(|_| server_id.to_string());

        // Common setup: get server and profile data once
        let Some((resolved_server_id, profile_data)) = Self::get_server_and_profile(db, server_id).await? else {
            if let Ok(cache_manager) = RedbCacheManager::global() {
                let _ = cache_manager.clear_refreshing(server_id).await;
            }
            tracing::warn!(
                "Capability sync aborted for server '{}' because configuration context was missing",
                server_id
            );
            return Ok(());
        };

        tracing::debug!(
            "Syncing capabilities (flags: {:?}) from server '{}' (ID: {}, instance: {}) to {} profiles",
            flags,
            server_name,
            server_id,
            instance_id,
            profile_data.len()
        );

        let protocol_version = service.peer_info().map(|info| info.protocol_version.to_string());

        let mut cached_tools: Vec<CachedToolInfo> = Vec::new();
        let mut cached_resources: Vec<CachedResourceInfo> = Vec::new();
        let mut cached_prompts: Vec<CachedPromptInfo> = Vec::new();
        let mut cached_templates: Vec<CachedResourceTemplateInfo> = Vec::new();

        // TOOLS - unified pattern
        if flags.contains(CapSyncFlags::TOOLS) {
            let tools: Vec<Tool> = if let Some(slice) = tools_opt {
                slice.to_vec()
            } else {
                Self::fetch_tools_from_service(service, &server_name, instance_id).await?
            };

            if !tools.is_empty() {
                let mut tools = tools;
                crate::config::server::tools::assign_unique_names_to_tools(
                    &db.pool,
                    &resolved_server_id,
                    &server_name,
                    &mut tools,
                )
                .await?;

                // Only sync to profile_* when there are profiles bound to this server
                if !profile_data.is_empty() {
                    Self::sync_tools_to_database_internal(db, &resolved_server_id, &server_name, &tools, &profile_data)
                        .await?;
                }

                let now = Utc::now();
                cached_tools.extend(tools.iter().map(|tool| {
                    let schema = tool.schema_as_json_value();
                    let input_schema_json = serde_json::to_string(&schema).unwrap_or_else(|_| "{}".to_string());
                    CachedToolInfo {
                        name: tool.name.to_string(),
                        description: tool.description.clone().map(|d| d.into_owned()),
                        input_schema_json,
                        output_schema_json: tool.output_schema.as_ref().map(|s| {
                            serde_json::to_string(&serde_json::Value::Object((**s).clone()))
                                .unwrap_or_else(|_| "{}".to_string())
                        }),
                        unique_name: None,
                        icons: tool.icons.clone(),
                        enabled: true,
                        cached_at: now,
                    }
                }));
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
                let resource_uris: Vec<String> = resources.iter().map(|r| r.uri.clone()).collect();
                if !profile_data.is_empty() {
                    Self::sync_resources_to_database_internal(
                        db,
                        &resolved_server_id,
                        &server_name,
                        &resource_uris,
                        &profile_data,
                    )
                    .await?;
                }

                let now = Utc::now();
                cached_resources.extend(resources.iter().map(|resource| CachedResourceInfo {
                    uri: resource.uri.clone(),
                    name: Some(resource.name.clone()),
                    description: resource.description.clone(),
                    mime_type: resource.mime_type.clone(),
                    icons: resource.icons.clone(),
                    enabled: true,
                    cached_at: now,
                }));
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
                if !profile_data.is_empty() {
                    Self::sync_prompts_to_database_internal(
                        db,
                        &resolved_server_id,
                        &server_name,
                        &prompts,
                        &profile_data,
                    )
                    .await?;
                }

                let now = Utc::now();
                cached_prompts.extend(prompts.iter().map(|prompt| {
                    CachedPromptInfo {
                        name: prompt.name.to_string(),
                        description: prompt.description.clone(),
                        arguments: prompt
                            .arguments
                            .clone()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|arg| PromptArgument {
                                name: arg.name,
                                description: arg.description,
                                required: arg.required.unwrap_or(false),
                            })
                            .collect(),
                        icons: prompt.icons.clone(),
                        enabled: true,
                        cached_at: now,
                    }
                }));
            } else {
                tracing::debug!(
                    "No prompts fetched for server '{}' (ID: {}), skipping prompts sync",
                    server_name,
                    server_id
                );
            }
        }

        if flags.contains(CapSyncFlags::RESOURCE_TEMPLATES) {
            let templates = match Self::fetch_resource_templates_from_service(service, &server_name, instance_id).await
            {
                Ok(items) => items,
                Err(err) => {
                    let msg = err.to_string();
                    if is_method_not_supported(&msg) {
                        tracing::debug!(
                            server_id = %server_id,
                            server_name = %server_name,
                            instance_id = %instance_id,
                            error = %msg,
                            "Resource templates not supported upstream; skipping sync"
                        );
                        Vec::new()
                    } else {
                        return Err(err);
                    }
                }
            };

            if !templates.is_empty() {
                let now = Utc::now();
                cached_templates.extend(templates.iter().map(|template| CachedResourceTemplateInfo {
                    uri_template: template.uri_template.clone(),
                    name: Some(template.name.clone()),
                    description: template.description.clone(),
                    mime_type: template.mime_type.clone(),
                    enabled: true,
                    cached_at: now,
                }));
            } else {
                tracing::debug!(
                    "No resource templates fetched for server '{}' (ID: {}), skipping templates sync",
                    server_name,
                    server_id
                );
            }
        }

        tracing::debug!(
            "Successfully synced capabilities (flags: {:?}) from server '{}' (ID: {})",
            flags,
            server_name,
            server_id
        );

        if !(cached_tools.is_empty()
            && cached_resources.is_empty()
            && cached_prompts.is_empty()
            && cached_templates.is_empty())
        {
            if let Ok(cache_manager) = RedbCacheManager::global() {
                if let Err(e) = store_dual_write(
                    &db.pool,
                    cache_manager.as_ref(),
                    &resolved_server_id,
                    &server_name,
                    cached_tools,
                    cached_resources,
                    cached_prompts,
                    cached_templates,
                    protocol_version,
                )
                .await
                {
                    tracing::warn!(
                        server_id = %server_id,
                        error = %e,
                        "Failed to store capability snapshot to REDB"
                    );
                }
            }
        }

        // Ensure refreshing marker is cleared even if nothing was written
        if let Ok(cache_manager) = RedbCacheManager::global() {
            let _ = cache_manager.clear_refreshing(server_id).await;
        }
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
    ) -> AnyhowResult<Vec<rmcp::model::Resource>> {
        use anyhow::Context;

        let resources = service.list_all_resources().await.with_context(|| {
            format!(
                "Failed to list resources from upstream server '{}' instance '{}'",
                server_name, instance_id
            )
        })?;

        Ok(resources)
    }

    /// Helper function to fetch resource templates from service with pagination
    async fn fetch_resource_templates_from_service(
        service: &rmcp::service::Peer<rmcp::service::RoleClient>,
        server_name: &str,
        instance_id: &str,
    ) -> AnyhowResult<Vec<ResourceTemplate>> {
        use anyhow::Context;

        let mut templates = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let response = service
                .list_resource_templates(cursor.clone().map(|c| PaginatedRequestParam { cursor: Some(c) }))
                .await
                .with_context(|| {
                    format!(
                        "Failed to list resource templates from upstream server '{}' instance '{}'",
                        server_name, instance_id
                    )
                })?;

            templates.extend(response.resource_templates);
            cursor = response.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(templates)
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
