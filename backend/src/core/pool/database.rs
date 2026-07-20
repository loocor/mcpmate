//! Database synchronization functionality for connection pool
//!
//! Handles synchronization of MCP capabilities (tools, resources, prompts) from
//! connected servers to the database. This module provides a unified approach
//! to syncing different types of capabilities across profile.

use anyhow::{Context, Result as AnyhowResult};
use mcpmate_capability_store::{CapabilityKind as CatalogKind, KindObservation};
use rmcp::model::{PaginatedRequestParams, Resource, ResourceTemplate, Tool};
use rmcp::{model::ErrorCode, service::ServiceError};
use std::sync::Arc;
use tracing;

use super::UpstreamConnectionPool;
use crate::common::sync::SyncHelper;
use crate::config::server::capabilities::{
    CapabilityFailureEvidence, CapabilityProtocolObservation, commit_capability_protocol_observation,
    record_capability_failure, unsupported_complete_observation,
};

enum ResourceTemplateSyncObservation {
    Complete(Vec<ResourceTemplate>),
    Unsupported,
}

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
    async fn record_sync_failure(
        db: &Arc<crate::config::database::Database>,
        server_id: &str,
        _server_name: &str,
        instance_id: &str,
        kind: CatalogKind,
        error: anyhow::Error,
    ) -> AnyhowResult<()> {
        let reason = format!("{error:#}");
        record_capability_failure(
            &db.pool,
            db.capability_cache.as_ref(),
            CapabilityFailureEvidence {
                server_id: server_id.to_string(),
                kind,
                instance_id: Some(instance_id.to_string()),
                connection_generation: None,
                reason,
            },
        )
        .await
        .context("Failed to persist terminal background capability sync evidence")?;
        Err(error)
    }

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
        // enabled in any profiles. That prevented durable catalog snapshots from
        // being refreshed for unattached servers. We now keep proceeding even when
        // no profiles are bound, so the service-level snapshot is always current,
        // while profile_* seeding is conditionally executed only when profiles exist.
        if profile_data.is_empty() {
            tracing::debug!(
                "No profiles currently enable server '{}'; will persist the SQLite catalog snapshot without profile associations",
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
                .list_tools(Some(rmcp::model::PaginatedRequestParams::default().with_cursor(cursor)))
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
        let Some((resolved_server_id, _profile_data)) = Self::get_server_and_profile(db, server_id).await? else {
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
            _profile_data.len()
        );

        let initialize = service.peer_info().as_deref().cloned();
        let mut discovered_tools: Vec<Tool> = Vec::new();
        let mut discovered_resources: Vec<Resource> = Vec::new();
        let mut discovered_prompts: Vec<rmcp::model::Prompt> = Vec::new();
        let mut discovered_templates: Vec<ResourceTemplate> = Vec::new();
        let mut kind_states: Vec<KindObservation> = Vec::new();

        // TOOLS - unified pattern
        if flags.contains(CapSyncFlags::TOOLS) {
            let tools: Vec<Tool> = if let Some(slice) = tools_opt {
                slice.to_vec()
            } else {
                match Self::fetch_tools_from_service(service, &server_name, instance_id).await {
                    Ok(tools) => tools,
                    Err(error) => {
                        return Self::record_sync_failure(
                            db,
                            server_id,
                            &server_name,
                            instance_id,
                            CatalogKind::Tools,
                            error,
                        )
                        .await;
                    }
                }
            };

            if tools.is_empty() {
                tracing::debug!(
                    "Server '{}' (ID: {}) returned a complete empty tools inventory",
                    server_name,
                    server_id
                );
            }
            discovered_tools = tools;
        }

        // RESOURCES - unified pattern
        if flags.contains(CapSyncFlags::RESOURCES) {
            let resources = match Self::fetch_resources_from_service(service, &server_name, instance_id).await {
                Ok(resources) => resources,
                Err(error) => {
                    return Self::record_sync_failure(
                        db,
                        server_id,
                        &server_name,
                        instance_id,
                        CatalogKind::Resources,
                        error,
                    )
                    .await;
                }
            };

            if resources.is_empty() {
                tracing::debug!(
                    "Server '{}' (ID: {}) returned a complete empty resources inventory",
                    server_name,
                    server_id
                );
            }
            discovered_resources = resources;
        }

        // PROMPTS - unified pattern
        if flags.contains(CapSyncFlags::PROMPTS) {
            let prompts = match Self::fetch_prompts_from_service(service, &server_name, instance_id).await {
                Ok(prompts) => prompts,
                Err(error) => {
                    return Self::record_sync_failure(
                        db,
                        server_id,
                        &server_name,
                        instance_id,
                        CatalogKind::Prompts,
                        error,
                    )
                    .await;
                }
            };

            if prompts.is_empty() {
                tracing::debug!(
                    "Server '{}' (ID: {}) returned a complete empty prompts inventory",
                    server_name,
                    server_id
                );
            }
            discovered_prompts = prompts;
        }

        if flags.contains(CapSyncFlags::RESOURCE_TEMPLATES) {
            let templates = match Self::fetch_resource_templates_from_service(service, &server_name, instance_id).await
            {
                Ok(ResourceTemplateSyncObservation::Complete(items)) => items,
                Ok(ResourceTemplateSyncObservation::Unsupported) => {
                    kind_states.push(unsupported_complete_observation(CatalogKind::ResourceTemplates));
                    tracing::debug!(
                        server_id = %server_id,
                        server_name = %server_name,
                        instance_id = %instance_id,
                        "Resource templates are unsupported by the upstream peer"
                    );
                    Vec::new()
                }
                Err(error) => {
                    return Self::record_sync_failure(
                        db,
                        server_id,
                        &server_name,
                        instance_id,
                        CatalogKind::ResourceTemplates,
                        error,
                    )
                    .await;
                }
            };

            if templates.is_empty() {
                tracing::debug!(
                    "Server '{}' (ID: {}) returned a complete empty resource template inventory",
                    server_name,
                    server_id
                );
            }
            discovered_templates = templates;
        }

        if let Err(error) = commit_capability_protocol_observation(
            &db.pool,
            db.capability_cache.as_ref(),
            &resolved_server_id,
            &server_name,
            CapabilityProtocolObservation {
                initialize,
                tools: discovered_tools,
                resources: discovered_resources,
                prompts: discovered_prompts,
                templates: discovered_templates,
                kinds: flags,
                kind_states,
            },
        )
        .await
        {
            crate::config::server::namespace_repair::record_capability_collision_from_error(&db.pool, &error).await?;
            return Err(error).context("Failed to persist transactional capability inventories");
        }

        tracing::debug!(
            "Successfully synced capabilities (flags: {:?}) from server '{}' (ID: {})",
            flags,
            server_name,
            server_id
        );

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
                .list_prompts(Some(rmcp::model::PaginatedRequestParams::default().with_cursor(cursor)))
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
    ) -> AnyhowResult<ResourceTemplateSyncObservation> {
        let mut templates = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let response = match service
                .list_resource_templates(
                    cursor
                        .clone()
                        .map(|c| PaginatedRequestParams::default().with_cursor(Some(c))),
                )
                .await
            {
                Ok(response) => response,
                Err(ServiceError::McpError(error)) if error.code == ErrorCode::METHOD_NOT_FOUND => {
                    return Ok(ResourceTemplateSyncObservation::Unsupported);
                }
                Err(error) => {
                    return Err(anyhow::Error::new(error)).with_context(|| {
                        format!(
                            "Failed to list resource templates from upstream server '{}' instance '{}'",
                            server_name, instance_id
                        )
                    });
                }
            };

            templates.extend(response.resource_templates);
            cursor = response.next_cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(ResourceTemplateSyncObservation::Complete(templates))
    }
}
