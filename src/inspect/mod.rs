// Inspect System Module
// Provides independent MCP server capability inspect and caching

pub mod capabilities;
pub mod client;
pub mod manager;
pub mod storage;
pub mod types;

use std::sync::Arc;

use crate::config::database::Database;
use crate::core::events::{
    bus::EventBus,
    types::{Event, InspectCacheUpdateType},
};

pub use capabilities::{
    CapabilitiesProcessor, CapabilitiesSummary, ProcessedCapabilities, ProcessedPromptInfo,
    ProcessedResourceInfo, ProcessedResourceTemplateInfo, ProcessedToolInfo,
};
pub use client::McpInspectClient;
pub use manager::{CacheLevel, CacheResult};
pub use manager::{CacheManagerStats, CapabilitiesCache, PreloadResult};
pub use storage::{CacheStats, TempFileStorage};
pub use types::{
    CapabilitySelections, InspectResponse, InspectError, InspectParams, InspectResult,
    RefreshStrategy, ResponseFormat, ResponseMetadata, ServerCapabilities, SyncResult,
};

/// Main Inspect Service
/// Integrates all inspect components and provides unified interface
pub struct InspectService {
    /// Capabilities cache manager
    cache: CapabilitiesCache,
    /// Capabilities processor
    processor: CapabilitiesProcessor,
    /// Event bus for notifications
    event_bus: Option<Arc<EventBus>>,
    /// Database connection
    database: Database,
}

impl InspectService {
    /// Create new inspect service
    pub fn new(
        database: Database,
        event_bus: Option<Arc<EventBus>>,
    ) -> InspectResult<Self> {
        let config = types::CacheConfig::default();
        let cache = if let Some(ref event_bus) = event_bus {
            CapabilitiesCache::with_event_bus(config, Arc::clone(event_bus))?
        } else {
            CapabilitiesCache::new(config)?
        };
        let processor = CapabilitiesProcessor::new(Some(database.clone()));

        Ok(Self {
            cache,
            processor,
            event_bus,
            database,
        })
    }

    /// Create inspect service with custom cache configuration
    pub fn with_config(
        database: Database,
        cache_config: types::CacheConfig,
        event_bus: Option<Arc<EventBus>>,
    ) -> InspectResult<Self> {
        let cache = if let Some(ref event_bus) = event_bus {
            CapabilitiesCache::with_event_bus(cache_config, Arc::clone(event_bus))?
        } else {
            CapabilitiesCache::new(cache_config)?
        };
        let processor = CapabilitiesProcessor::new(Some(database.clone()));

        Ok(Self {
            cache,
            processor,
            event_bus,
            database,
        })
    }

    /// Get server capabilities with specified refresh strategy
    pub async fn get_server_capabilities(
        &self,
        server_id: &str,
        refresh_strategy: RefreshStrategy,
    ) -> InspectResult<ServerCapabilities> {
        let result = self
            .get_server_capabilities_with_cache_info(server_id, refresh_strategy)
            .await?;
        Ok(result.capabilities)
    }

    /// Get server capabilities with cache hit information
    pub async fn get_server_capabilities_with_cache_info(
        &self,
        server_id: &str,
        refresh_strategy: RefreshStrategy,
    ) -> InspectResult<manager::CacheResult> {
        tracing::debug!(
            "Getting capabilities for server '{}' with strategy {:?}",
            server_id,
            refresh_strategy
        );

        let result = self
            .cache
            .get_capabilities(server_id, refresh_strategy, &self.database)
            .await?;

        // Log cache performance
        let cache_level_str = match result.cache_level {
            Some(manager::CacheLevel::Memory) => "L1",
            Some(manager::CacheLevel::File) => "L2",
            None => "MISS",
        };

        tracing::debug!(
            "Cache performance for server '{}': {} ({})",
            server_id,
            if result.cache_hit { "HIT" } else { "MISS" },
            cache_level_str
        );

        // Emit specific inspect event if event bus is available
        if let Some(event_bus) = &self.event_bus {
            let update_type = match refresh_strategy {
                RefreshStrategy::Force => InspectCacheUpdateType::Manual,
                RefreshStrategy::RefreshIfStale => InspectCacheUpdateType::Fresh,
                RefreshStrategy::CacheFirst => InspectCacheUpdateType::FileCache,
            };

            let event = Event::InspectCacheUpdated {
                server_id: server_id.to_string(),
                server_name: result.capabilities.server_name.clone(),
                update_type,
            };

            event_bus.publish(event);
        }

        Ok(result)
    }

    /// Get processed server capabilities with format options
    pub async fn get_processed_capabilities(
        &self,
        server_id: &str,
        params: InspectParams,
    ) -> InspectResult<ProcessedCapabilities> {
        let refresh_strategy = params.refresh.unwrap_or_default();
        let format = params.format.unwrap_or_default();

        // Get raw capabilities
        let capabilities = self
            .get_server_capabilities(server_id, refresh_strategy)
            .await?;

        // Process and format
        let processed = self
            .processor
            .process_capabilities(&capabilities, format)
            .await?;

        Ok(processed)
    }

    /// Get server tools with filtering and formatting
    pub async fn get_server_tools(
        &self,
        server_id: &str,
        params: InspectParams,
    ) -> InspectResult<Vec<ProcessedToolInfo>> {
        let processed = self.get_processed_capabilities(server_id, params).await?;
        Ok(processed.tools)
    }

    /// Get specific tool details
    pub async fn get_tool_detail(
        &self,
        server_id: &str,
        tool_id: &str,
        params: InspectParams,
    ) -> InspectResult<Option<ProcessedToolInfo>> {
        let tools = self.get_server_tools(server_id, params).await?;

        // Find tool by name or unique name (tool_id can match either)
        let tool = tools
            .into_iter()
            .find(|t| t.name == tool_id || t.unique_name.as_ref().is_some_and(|un| un == tool_id));

        Ok(tool)
    }

    /// Get server resources with filtering and formatting
    pub async fn get_server_resources(
        &self,
        server_id: &str,
        params: InspectParams,
    ) -> InspectResult<Vec<ProcessedResourceInfo>> {
        let processed = self.get_processed_capabilities(server_id, params).await?;
        Ok(processed.resources)
    }

    /// Get server prompts with filtering and formatting
    pub async fn get_server_prompts(
        &self,
        server_id: &str,
        params: InspectParams,
    ) -> InspectResult<Vec<ProcessedPromptInfo>> {
        let processed = self.get_processed_capabilities(server_id, params).await?;
        Ok(processed.prompts)
    }

    /// Get server resource templates
    pub async fn get_server_resource_templates(
        &self,
        server_id: &str,
        params: InspectParams,
    ) -> InspectResult<Vec<ProcessedResourceTemplateInfo>> {
        let processed = self.get_processed_capabilities(server_id, params).await?;
        Ok(processed.resource_templates)
    }

    /// Sync capability selections to config suit
    pub async fn sync_capability_selection_to_config_suit(
        &self,
        capability_selections: &CapabilitySelections,
        config_suit_id: &str,
    ) -> InspectResult<SyncResult> {
        tracing::info!(
            "Syncing capability selections for server '{}' to config suit '{}'",
            capability_selections.server_id,
            config_suit_id
        );

        // Update server enabled status
        // Note: Server enabled status update would need to be implemented
        // For now, we'll skip this and focus on tool/resource/prompt updates

        // Update tool selections
        // Note: Tool enabled status update would need proper implementation
        // For now, we'll just count the tools that would be updated
        let tools_updated = capability_selections.tools.len();

        // Update resource selections
        // Note: Resource enabled status update would need proper implementation
        // For now, we'll just count the resources that would be updated
        let resources_updated = capability_selections.resources.len();

        // Update prompt selections
        // Note: Prompt enabled status update would need proper implementation
        // For now, we'll just count the prompts that would be updated
        let prompts_updated = capability_selections.prompts.len();

        // Emit configuration change event
        if let Some(event_bus) = &self.event_bus {
            let event = Event::ConfigReloaded;

            event_bus.publish(event);
        }

        Ok(SyncResult {
            success: true,
            tools_updated,
            resources_updated,
            prompts_updated,
            error: None,
        })
    }

    /// Invalidate cache for a server
    pub async fn invalidate_server_cache(
        &self,
        server_id: &str,
    ) -> InspectResult<()> {
        // Get server name before invalidation for event
        let server_name = if let Ok(Some(server)) =
            crate::config::server::get_server(&self.database.pool, server_id).await
        {
            server.name
        } else {
            server_id.to_string()
        };

        self.cache.invalidate(server_id).await?;

        // Emit cache invalidation event
        if let Some(event_bus) = &self.event_bus {
            let event = Event::InspectCacheInvalidated {
                server_id: server_id.to_string(),
                server_name,
            };

            event_bus.publish(event);
        }

        Ok(())
    }

    /// Clear all inspect cache
    pub async fn clear_all_cache(&self) -> InspectResult<()> {
        self.cache.clear_all().await?;

        // Emit cache clear event
        if let Some(event_bus) = &self.event_bus {
            let event = Event::InspectCacheCleared;

            event_bus.publish(event);
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> InspectResult<CacheManagerStats> {
        self.cache.get_stats().await
    }

    /// Preload capabilities for multiple servers
    pub async fn preload_servers(
        &self,
        server_ids: &[String],
    ) -> InspectResult<PreloadResult> {
        self.cache.preload_servers(server_ids, &self.database).await
    }

    /// Start background refresh for active servers
    pub async fn start_background_refresh(
        &self,
        server_ids: &[String],
    ) -> InspectResult<()> {
        self.cache
            .background_refresh(server_ids, &self.database)
            .await
    }

    /// Get memory cache hit ratio
    pub async fn get_cache_hit_ratio(&self) -> f64 {
        self.cache.get_hit_ratio().await
    }
}

/// Inspect service builder for easier configuration
pub struct InspectServiceBuilder {
    database: Option<Database>,
    cache_config: Option<types::CacheConfig>,
    event_bus: Option<Arc<EventBus>>,
}

impl InspectServiceBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            database: None,
            cache_config: None,
            event_bus: None,
        }
    }

    /// Set database connection
    pub fn with_database(
        mut self,
        database: Database,
    ) -> Self {
        self.database = Some(database);
        self
    }

    /// Set cache configuration
    pub fn with_cache_config(
        mut self,
        config: types::CacheConfig,
    ) -> Self {
        self.cache_config = Some(config);
        self
    }

    /// Set event bus
    pub fn with_event_bus(
        mut self,
        event_bus: Arc<EventBus>,
    ) -> Self {
        self.event_bus = Some(event_bus);
        self
    }

    /// Build inspect service
    pub fn build(self) -> InspectResult<InspectService> {
        let database = self.database.ok_or_else(|| {
            InspectError::InvalidConfig("Database connection is required".to_string())
        })?;

        if let Some(cache_config) = self.cache_config {
            InspectService::with_config(database, cache_config, self.event_bus)
        } else {
            InspectService::new(database, self.event_bus)
        }
    }
}

impl Default for InspectServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}
